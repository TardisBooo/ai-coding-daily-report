use crate::config::Config;
use crate::model::HarnessRoot;
use crate::util::resolve_path;
use std::path::{Path, PathBuf};

pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

pub fn builtin_candidates(id: &str) -> Vec<PathBuf> {
    let home = home_dir();
    if id == "antigravity" {
        return vec![
            home.join(".gemini").join("antigravity"),
            home.join(".gemini").join("antigravity-cli"),
        ];
    }
    let default = home.join(format!(".{id}"));
    if id == "codex" {
        if let Some(path) = std::env::var_os("CODEX_HOME").filter(|path| !path.is_empty()) {
            return vec![PathBuf::from(path), default];
        }
    }
    vec![default]
}

pub fn detect_root(id: &str, cfg: &Config) -> Option<PathBuf> {
    if let Some(p) = cfg.harness_override_dir(id) {
        if p.exists() {
            return Some(resolve_path(&p));
        }
    }
    let candidates = builtin_candidates(id);
    for c in &candidates {
        if c.exists() && fingerprint_ok(id, c) {
            return Some(resolve_path(c));
        }
    }
    for c in candidates {
        if c.exists() {
            return Some(resolve_path(&c));
        }
    }
    None
}

fn fingerprint_ok(id: &str, root: &Path) -> bool {
    match id {
        "codex" => root.join("history.jsonl").exists() || root.join("sessions").exists(),
        "claude" => root.join("projects").exists(),
        "grok" => root.join("sessions").exists(),
        "pi" | "omp" => root.join("agent").join("sessions").exists() || root.join("agent").exists(),
        "dsh" => root.join("sessions").exists(),
        "antigravity" => root.join("brain").exists(),
        _ => root.exists(),
    }
}

pub fn scan_roots(cfg: &Config) -> Vec<HarnessRoot> {
    let mut out = Vec::new();
    for id in crate::model::TOOLS {
        let label = crate::model::tool_label(id).to_string();
        if !cfg.harness_enabled(id) {
            continue;
        }
        match detect_root(id, cfg) {
            Some(path) if fingerprint_ok(id, &path) => out.push(HarnessRoot {
                id: id.into(),
                label,
                path,
                detected: true,
            }),
            Some(path) => out.push(HarnessRoot {
                id: id.into(),
                label,
                path,
                detected: false,
            }),
            None => out.push(HarnessRoot {
                id: id.into(),
                label,
                path: builtin_candidates(id)
                    .into_iter()
                    .next()
                    .unwrap_or_default(),
                detected: false,
            }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_candidates_use_source_specific_defaults() {
        let home = home_dir();
        assert_eq!(builtin_candidates("dsh")[0], home.join(".dsh"));
        assert_eq!(
            builtin_candidates("antigravity")[0],
            home.join(".gemini").join("antigravity")
        );
        assert_eq!(
            builtin_candidates("antigravity")[1],
            home.join(".gemini").join("antigravity-cli")
        );
    }

    #[test]
    fn fingerprints_recognize_new_sources() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sessions")).unwrap();
        assert!(fingerprint_ok("dsh", dir.path()));
        std::fs::remove_dir(dir.path().join("sessions")).unwrap();
        std::fs::create_dir(dir.path().join("brain")).unwrap();
        assert!(fingerprint_ok("antigravity", dir.path()));
    }
}
