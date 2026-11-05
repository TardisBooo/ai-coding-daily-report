use crate::config::Config;
use crate::discovery::scan_roots;
use crate::model::{Activity, AuditLine, CollectPayload, ProjectBundle, ToolStat, TOOLS};
use crate::util::{
    clean_prompt, day_bounds, decode_claude_dir, decode_pi_dir, glob_pat, in_day, is_filler,
    iter_jsonl, json_str, mtime_utc, parse_iso, parse_tz, short_project, truncate, worth_keeping,
};
use anyhow::Result;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use chrono_tz::Tz;
use glob::glob;
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Duration as StdDuration;

pub fn collect(cfg: &Config, date: NaiveDate) -> Result<CollectPayload> {
    let tz = parse_tz(&cfg.timezone)?;
    let (start, end) = day_bounds(date, tz);
    let roots = scan_roots(cfg);
    let mut acts = Vec::new();
    for r in &roots {
        if !r.detected {
            continue;
        }
        match r.id.as_str() {
            "codex" => acts.extend(collect_codex(&r.path, start, end)),
            "claude" => acts.extend(collect_claude(&r.path, start, end)),
            "grok" => acts.extend(collect_grok(&r.path, start, end)),
            "pi" | "omp" => acts.extend(collect_pi_like(&r.path, &r.id, start, end)),
            "dsh" => acts.extend(collect_dsh(&r.path, start, end)),
            "antigravity" => acts.extend(collect_antigravity(&r.path, start, end)),
            _ => {}
        }
    }
    Ok(bundle(date, tz, acts))
}

fn collect_codex(root: &Path, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<Activity> {
    let mut acts = Vec::new();
    let mut sid_cwd: HashMap<String, String> = HashMap::new();
    let pat = glob_pat(&root.join("sessions"), "*/*/*/rollout-*.jsonl");
    if let Ok(paths) = glob(&pat) {
        for p in paths.flatten() {
            let Ok(f) = File::open(&p) else { continue };
            let mut first = String::new();
            let mut r = BufReader::new(f);
            if r.read_line(&mut first).is_err() {
                continue;
            }
            let Ok(obj) = serde_json::from_str::<Value>(first.trim()) else {
                continue;
            };
            if json_str(&obj, "type") != Some("session_meta") {
                continue;
            }
            let payload = obj.get("payload").cloned().unwrap_or(Value::Null);
            let sid = json_str(&payload, "id")
                .or_else(|| json_str(&payload, "session_id"))
                .unwrap_or("")
                .to_string();
            let cwd = json_str(&payload, "cwd")
                .unwrap_or("(未知项目)")
                .to_string();
            if !sid.is_empty() {
                sid_cwd.insert(sid.clone(), cwd.clone());
            }
            if let Some(ts) = json_str(&payload, "timestamp").and_then(parse_iso) {
                if in_day(ts, start, end) {
                    acts.push(Activity {
                        tool: "codex".into(),
                        project: cwd,
                        time: ts,
                        text: None,
                        kind: "session".into(),
                    });
                }
            }
        }
    }
    let history = root.join("history.jsonl");
    let mut recent: HashMap<(String, String), DateTime<Utc>> = HashMap::new();
    if history.exists() {
        for obj in iter_jsonl(&history) {
            let Some(raw) = obj
                .get("ts")
                .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
            else {
                continue;
            };
            let ts = DateTime::<Utc>::from_timestamp(raw, 0).unwrap_or(Utc::now());
            if !in_day(ts, start, end) {
                continue;
            }
            let text = clean_prompt(json_str(&obj, "text").unwrap_or(""));
            if !worth_keeping(&text) {
                continue;
            }
            let sid = json_str(&obj, "session_id").unwrap_or("").to_string();
            let key = (sid.clone(), text.clone());
            if let Some(last) = recent.get(&key) {
                if ts - *last < Duration::minutes(2) {
                    continue;
                }
            }
            recent.insert(key, ts);
            let project = sid_cwd
                .get(&sid)
                .cloned()
                .unwrap_or_else(|| "(未定位项目)".into());
            acts.push(Activity {
                tool: "codex".into(),
                project,
                time: ts,
                text: Some(text),
                kind: "prompt".into(),
            });
        }
    }
    acts
}

fn collect_claude(root: &Path, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<Activity> {
    let mut acts = Vec::new();
    let pat = glob_pat(&root.join("projects"), "*/*.jsonl");
    let Ok(paths) = glob(&pat) else {
        return acts;
    };
    for p in paths.flatten() {
        if let Some(mt) = mtime_utc(&p) {
            if mt < start {
                continue;
            }
        }
        let objs: Vec<Value> = iter_jsonl(&p).collect();
        let cwd = objs
            .iter()
            .find_map(|o| json_str(o, "cwd").map(|s| s.to_string()));
        let parent = p
            .parent()
            .and_then(|x| x.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let project = cwd.unwrap_or_else(|| decode_claude_dir(&parent));
        let mut seen: HashMap<String, DateTime<Utc>> = HashMap::new();
        let mut had_session = false;
        for obj in objs {
            let ts = json_str(&obj, "timestamp").and_then(parse_iso);
            let in_d = ts.map(|t| in_day(t, start, end)).unwrap_or(false);
            if in_d && !had_session {
                had_session = true;
                acts.push(Activity {
                    tool: "claude".into(),
                    project: project.clone(),
                    time: ts.unwrap(),
                    text: None,
                    kind: "session".into(),
                });
            }
            let t = json_str(&obj, "type").unwrap_or("");
            let mut text = None;
            if t == "queue-operation" && json_str(&obj, "operation") == Some("enqueue") && in_d {
                text = Some(clean_prompt(json_str(&obj, "content").unwrap_or("")));
            } else if t == "user" && in_d {
                if let Some(content) = obj
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                {
                    text = Some(clean_prompt(content));
                }
            }
            if let (Some(tx), Some(ts)) = (text, ts) {
                if worth_keeping(&tx) {
                    if let Some(last) = seen.get(&tx) {
                        if ts - *last < Duration::minutes(2) {
                            continue;
                        }
                    }
                    seen.insert(tx.clone(), ts);
                    acts.push(Activity {
                        tool: "claude".into(),
                        project: project.clone(),
                        time: ts,
                        text: Some(tx),
                        kind: "prompt".into(),
                    });
                }
            }
        }
    }
    acts
}

fn collect_grok(root: &Path, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<Activity> {
    let mut acts = Vec::new();
    let pat = glob_pat(&root.join("sessions"), "*/prompt_history.jsonl");
    let Ok(paths) = glob(&pat) else {
        return acts;
    };
    for p in paths.flatten() {
        let encoded = p
            .parent()
            .and_then(|x| x.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let project = urlencoding::decode(&encoded)
            .map(|s| s.into_owned())
            .unwrap_or(encoded);
        let mut seen_sids = HashSet::new();
        for obj in iter_jsonl(&p) {
            let Some(ts) = json_str(&obj, "timestamp").and_then(parse_iso) else {
                continue;
            };
            if !in_day(ts, start, end) {
                continue;
            }
            if obj
                .get("is_bash")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                continue;
            }
            let sid = json_str(&obj, "session_id").unwrap_or("?").to_string();
            if seen_sids.insert(sid) {
                acts.push(Activity {
                    tool: "grok".into(),
                    project: project.clone(),
                    time: ts,
                    text: None,
                    kind: "session".into(),
                });
            }
            let text = clean_prompt(json_str(&obj, "prompt").unwrap_or(""));
            if worth_keeping(&text) {
                acts.push(Activity {
                    tool: "grok".into(),
                    project: project.clone(),
                    time: ts,
                    text: Some(text),
                    kind: "prompt".into(),
                });
            }
        }
    }
    acts
}

fn collect_pi_like(
    root: &Path,
    tool: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Vec<Activity> {
    let mut acts = Vec::new();
    let sessions = root.join("agent").join("sessions");
    if !sessions.exists() {
        return acts;
    }
    let pat = glob_pat(&sessions, "*/*.jsonl");
    let Ok(paths) = glob(&pat) else {
        return acts;
    };
    for sf in paths.flatten() {
        if let Some(mt) = mtime_utc(&sf) {
            if mt < start {
                continue;
            }
        }
        let parent = sf
            .parent()
            .and_then(|x| x.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut cwd: Option<String> = None;
        let mut title: Option<String> = None;
        let mut had_session = false;
        let mut seen: HashMap<String, DateTime<Utc>> = HashMap::new();
        for obj in iter_jsonl(&sf) {
            let t = json_str(&obj, "type").unwrap_or("");
            let mut ts = json_str(&obj, "timestamp").and_then(parse_iso);
            if t == "session" {
                if let Some(c) = json_str(&obj, "cwd") {
                    cwd = Some(c.to_string());
                }
                if let Some(ti) = json_str(&obj, "title") {
                    title = Some(ti.to_string());
                }
                if ts.is_none() {
                    if let Some(name) = sf.file_name().and_then(|s| s.to_str()) {
                        if name.len() >= 19 {
                            let cand = format!(
                                "{}T{}:{}:{}+00:00",
                                &name[0..10],
                                &name[11..13],
                                &name[14..16],
                                &name[17..19]
                            );
                            ts = parse_iso(&cand);
                        }
                    }
                }
            } else if t == "title" {
                if let Some(ti) = json_str(&obj, "title") {
                    title = Some(ti.to_string());
                }
            }
            let project = cwd.clone().unwrap_or_else(|| decode_pi_dir(&parent));
            if let Some(ts) = ts {
                if in_day(ts, start, end) && !had_session {
                    had_session = true;
                    acts.push(Activity {
                        tool: tool.into(),
                        project: project.clone(),
                        time: ts,
                        text: title.clone(),
                        kind: "session".into(),
                    });
                }
            }
            if t != "message" {
                continue;
            }
            let Some(ts) = ts else { continue };
            if !in_day(ts, start, end) {
                continue;
            }
            let msg = obj.get("message").cloned().unwrap_or(Value::Null);
            if json_str(&msg, "role") != Some("user") {
                continue;
            }
            let content = msg.get("content");
            let mut texts = Vec::new();
            if let Some(s) = content.and_then(|c| c.as_str()) {
                texts.push(s.to_string());
            } else if let Some(arr) = content.and_then(|c| c.as_array()) {
                for c in arr {
                    if json_str(c, "type") == Some("text") {
                        if let Some(tx) = json_str(c, "text") {
                            texts.push(tx.to_string());
                        }
                    }
                }
            }
            let text = clean_prompt(&texts.join(" "));
            if worth_keeping(&text) {
                if let Some(last) = seen.get(&text) {
                    if ts - *last < Duration::minutes(2) {
                        continue;
                    }
                }
                seen.insert(text.clone(), ts);
                acts.push(Activity {
                    tool: tool.into(),
                    project,
                    time: ts,
                    text: Some(text),
                    kind: "prompt".into(),
                });
            }
        }
    }
    acts
}

struct DshSession {
    id: String,
    project: String,
    created_at: Option<DateTime<Utc>>,
    activity_times: Vec<DateTime<Utc>>,
    prompts: Vec<(DateTime<Utc>, String)>,
}

fn collect_dsh(root: &Path, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<Activity> {
    let mut acts = Vec::new();
    let pat = glob_pat(&root.join("sessions"), "**/session*.jsonl*");
    let Ok(paths) = glob(&pat) else {
        return acts;
    };

    // A session directory can retain more than one generation. Prefer the
    // highest version and compressed storage when both describe one session.
    let mut selected: HashMap<std::path::PathBuf, (std::path::PathBuf, (u32, bool))> =
        HashMap::new();
    for path in paths.flatten() {
        let Some(parent) = path.parent() else {
            continue;
        };
        let rank = dsh_artifact_rank(&path);
        let entry = selected
            .entry(parent.to_path_buf())
            .or_insert_with(|| (path.clone(), rank));
        if rank > entry.1 {
            *entry = (path, rank);
        }
    }

    let mut artifacts: Vec<_> = selected.into_values().map(|(path, _)| path).collect();
    artifacts.sort();
    let mut seen_sessions = HashSet::new();
    let mut recent: HashMap<(String, String), DateTime<Utc>> = HashMap::new();
    for path in artifacts {
        let Some(session) = read_dsh_file(&path) else {
            continue;
        };
        if !seen_sessions.insert(session.id.clone()) {
            continue;
        }

        let session_time = session
            .activity_times
            .iter()
            .copied()
            .find(|ts| in_day(*ts, start, end))
            .or_else(|| {
                session
                    .prompts
                    .iter()
                    .map(|(ts, _)| *ts)
                    .find(|ts| in_day(*ts, start, end))
            })
            .or_else(|| session.created_at.filter(|ts| in_day(*ts, start, end)));
        if let Some(ts) = session_time {
            acts.push(Activity {
                tool: "dsh".into(),
                project: session.project.clone(),
                time: ts,
                text: None,
                kind: "session".into(),
            });
        }

        for (ts, text) in session.prompts {
            if !in_day(ts, start, end) {
                continue;
            }
            let key = (session.id.clone(), text.clone());
            if let Some(last) = recent.get(&key) {
                if ts - *last < Duration::minutes(2) {
                    continue;
                }
            }
            recent.insert(key, ts);
            acts.push(Activity {
                tool: "dsh".into(),
                project: session.project.clone(),
                time: ts,
                text: Some(text),
                kind: "prompt".into(),
            });
        }
    }
    acts
}

fn dsh_artifact_rank(path: &Path) -> (u32, bool) {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let version = name
        .strip_prefix("session.v")
        .and_then(|rest| rest.split('.').next())
        .and_then(|raw| raw.parse::<u32>().ok())
        .unwrap_or(0);
    (version, name.ends_with(".zstd"))
}

fn read_dsh_file(path: &Path) -> Option<DshSession> {
    let fallback_id = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    if path.extension().and_then(|s| s.to_str()) == Some("zstd") {
        let file = File::open(path).ok()?;
        let decoder = zstd::stream::read::Decoder::new(file).ok()?;
        read_dsh_session(BufReader::new(decoder), fallback_id)
    } else {
        let file = File::open(path).ok()?;
        read_dsh_session(BufReader::new(file), fallback_id)
    }
}

fn read_dsh_session<R: BufRead>(reader: R, fallback_id: String) -> Option<DshSession> {
    let mut header: Option<(String, String, Option<DateTime<Utc>>)> = None;
    let mut activity_times = Vec::new();
    let mut prompts = Vec::new();
    for line in reader.lines().map_while(|line| line.ok()) {
        let Ok(obj) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if json_str(&obj, "type") == Some("session") && header.is_none() {
            let id = json_str(&obj, "id").unwrap_or(&fallback_id).to_string();
            let project = json_str(&obj, "cwd")
                .filter(|cwd| !cwd.trim().is_empty())
                .unwrap_or("(未知项目)")
                .to_string();
            header = Some((id, project, dsh_timestamp(&obj, "createdAt")));
            continue;
        }
        if let Some(ts) = dsh_timestamp(&obj, "time") {
            activity_times.push(ts);
        }
        if json_str(&obj, "type") != Some("user/message") {
            continue;
        }
        let Some(data) = obj.get("data") else {
            continue;
        };
        if json_str(data, "role") != Some("user")
            || data
                .get("source")
                .and_then(|source| json_str(source, "kind"))
                != Some("user")
        {
            continue;
        }
        let Some(ts) = dsh_timestamp(&obj, "time") else {
            continue;
        };
        let text = clean_prompt(&content_text(data.get("content")));
        if worth_keeping(&text) {
            prompts.push((ts, text));
        }
    }
    let (id, project, created_at) = header?;
    Some(DshSession {
        id,
        project,
        created_at,
        activity_times,
        prompts,
    })
}

fn dsh_timestamp(obj: &Value, key: &str) -> Option<DateTime<Utc>> {
    let value = obj.get(key)?;
    if let Some(raw) = value.as_i64() {
        return DateTime::<Utc>::from_timestamp_millis(raw);
    }
    if let Some(raw) = value.as_f64() {
        return DateTime::<Utc>::from_timestamp_millis(raw as i64);
    }
    value.as_str().and_then(parse_iso)
}

fn content_text(content: Option<&Value>) -> String {
    if let Some(raw) = content.and_then(Value::as_str) {
        return raw.to_string();
    }
    content
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|part| json_str(part, "type") == Some("text"))
        .filter_map(|part| json_str(part, "text"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn collect_antigravity(root: &Path, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<Activity> {
    let mut acts = Vec::new();
    let pat = glob_pat(
        &root.join("brain"),
        "*/.system_generated/logs/transcript*.jsonl",
    );
    let Ok(paths) = glob(&pat) else {
        return acts;
    };

    // transcript.jsonl is the canonical file. Use transcript_full.jsonl only
    // when the canonical file is absent, so a conversation is not doubled.
    let mut selected: HashMap<std::path::PathBuf, (std::path::PathBuf, bool)> = HashMap::new();
    for path in paths.flatten() {
        let Some(log_dir) = path.parent() else {
            continue;
        };
        let is_canonical = path.file_name().and_then(|s| s.to_str()) == Some("transcript.jsonl");
        let entry = selected
            .entry(log_dir.to_path_buf())
            .or_insert_with(|| (path.clone(), is_canonical));
        if is_canonical && !entry.1 {
            *entry = (path, true);
        }
    }

    let workspaces = load_antigravity_workspaces(&root.join("conversation_summaries.db"));
    let mut logs: Vec<_> = selected.into_values().map(|(path, _)| path).collect();
    logs.sort();
    for path in logs {
        let conversation_id = path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".into());
        let project = workspaces
            .get(&conversation_id)
            .cloned()
            .unwrap_or_else(|| "(未定位项目)".into());
        let mut had_session = false;
        let mut seen: HashMap<String, DateTime<Utc>> = HashMap::new();
        for obj in iter_jsonl(&path) {
            if json_str(&obj, "type") != Some("USER_INPUT")
                || json_str(&obj, "source") != Some("USER_EXPLICIT")
            {
                continue;
            }
            let Some(ts) = json_str(&obj, "created_at").and_then(parse_iso) else {
                continue;
            };
            if !in_day(ts, start, end) {
                continue;
            }
            if !had_session {
                had_session = true;
                acts.push(Activity {
                    tool: "antigravity".into(),
                    project: project.clone(),
                    time: ts,
                    text: None,
                    kind: "session".into(),
                });
            }
            let text = clean_prompt(&antigravity_request(
                json_str(&obj, "content").unwrap_or(""),
            ));
            if !worth_keeping(&text) {
                continue;
            }
            if let Some(last) = seen.get(&text) {
                if ts - *last < Duration::minutes(2) {
                    continue;
                }
            }
            seen.insert(text.clone(), ts);
            acts.push(Activity {
                tool: "antigravity".into(),
                project: project.clone(),
                time: ts,
                text: Some(text),
                kind: "prompt".into(),
            });
        }
    }
    acts
}

fn antigravity_request(content: &str) -> String {
    const OPEN: &str = "<USER_REQUEST>";
    const CLOSE: &str = "</USER_REQUEST>";
    if let Some(start) = content.find(OPEN) {
        let rest = &content[start + OPEN.len()..];
        if let Some(end) = rest.find(CLOSE) {
            return rest[..end].to_string();
        }
    }
    content.to_string()
}

fn load_antigravity_workspaces(path: &Path) -> HashMap<String, String> {
    let mut result = HashMap::new();
    if !path.exists() {
        return result;
    }
    let Ok(connection) = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return result;
    };
    let _ = connection.busy_timeout(StdDuration::from_millis(250));
    let Ok(mut statement) =
        connection.prepare("SELECT conversation_id, workspace_uris FROM conversation_summaries")
    else {
        return result;
    };
    let Ok(rows) = statement.query_map([], |row| {
        let id: String = row.get(0)?;
        let raw: Option<String> = row.get(1)?;
        Ok((id, raw))
    }) else {
        return result;
    };
    for row in rows.flatten() {
        let (id, raw) = row;
        if let Some(path) = raw.as_deref().and_then(antigravity_workspace_path) {
            result.insert(id, path);
        }
    }
    result
}

fn antigravity_workspace_path(raw: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(raw).unwrap_or_else(|_| Value::String(raw.into()));
    let candidates = match value {
        Value::Array(items) => items
            .into_iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect::<Vec<_>>(),
        Value::String(item) => vec![item],
        _ => Vec::new(),
    };
    candidates.into_iter().find_map(|uri| {
        let decoded = urlencoding::decode(&uri).ok()?.into_owned();
        let path = decoded
            .strip_prefix("file:///")
            .or_else(|| decoded.strip_prefix("file://"))?;
        let path = if cfg!(windows) {
            path.replace('/', "\\")
        } else {
            format!("/{path}")
        };
        (!path.trim().is_empty()).then_some(path)
    })
}

fn bundle(date: NaiveDate, tz: Tz, acts: Vec<Activity>) -> CollectPayload {
    let mut by_proj: HashMap<String, Vec<Activity>> = HashMap::new();
    let mut notes = Vec::new();
    for a in &acts {
        if a.kind == "note" {
            if let Some(t) = &a.text {
                notes.push(t.clone());
            }
            continue;
        }
        by_proj
            .entry(a.project.clone())
            .or_default()
            .push(a.clone());
    }
    let mut projects: Vec<ProjectBundle> = by_proj
        .into_iter()
        .map(|(path, items)| {
            let mut tools: Vec<String> = items.iter().map(|a| a.tool.clone()).collect();
            tools.sort();
            tools.dedup();
            tools.sort_by_key(|t| TOOLS.iter().position(|x| x == t).unwrap_or(99));
            let times: Vec<_> = items.iter().map(|a| a.time).collect();
            let start = times.iter().min().copied().unwrap_or(Utc::now());
            let end = times.iter().max().copied().unwrap_or(start);
            let mut prompts: Vec<String> = items
                .iter()
                .filter(|a| a.kind == "prompt")
                .filter_map(|a| a.text.clone())
                .filter(|t| !is_filler(t))
                .collect();
            let mut seen = HashSet::new();
            prompts.retain(|t| seen.insert(t.clone()));
            ProjectBundle {
                name: short_project(&path),
                path: path.clone(),
                tools,
                start: start.with_timezone(&tz).format("%H:%M").to_string(),
                end: end.with_timezone(&tz).format("%H:%M").to_string(),
                rounds: items.iter().filter(|a| a.kind == "prompt").count(),
                sessions: items.iter().filter(|a| a.kind == "session").count(),
                prompts,
            }
        })
        .collect();
    projects.sort_by(|a, b| b.rounds.cmp(&a.rounds).then(a.start.cmp(&b.start)));

    let stats = TOOLS
        .iter()
        .map(|id| {
            let ta: Vec<_> = acts.iter().filter(|a| a.tool == *id).collect();
            ToolStat {
                tool: (*id).into(),
                label: crate::model::tool_label(id).into(),
                sessions: ta.iter().filter(|a| a.kind == "session").count(),
                prompts: ta.iter().filter(|a| a.kind == "prompt").count(),
            }
        })
        .collect();

    let mut audit: Vec<AuditLine> = acts
        .iter()
        .filter(|a| a.kind != "note")
        .map(|a| AuditLine {
            time: a.time.with_timezone(&tz).format("%H:%M").to_string(),
            tool: crate::model::tool_label(&a.tool).into(),
            project: a.project.clone(),
            kind: a.kind.clone(),
            text: a
                .text
                .as_deref()
                .map(|t| truncate(t, 160))
                .unwrap_or_default(),
        })
        .collect();
    audit.sort_by(|a, b| a.time.cmp(&b.time).then(a.tool.cmp(&b.tool)));

    CollectPayload {
        date: date.format("%Y-%m-%d").to_string(),
        timezone: tz.name().to_string(),
        projects,
        stats,
        notes,
        audit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rusqlite::params;
    use std::io::Write;

    fn test_bounds() -> (DateTime<Utc>, DateTime<Utc>) {
        (
            parse_iso("2026-09-12T00:00:00Z").unwrap(),
            parse_iso("2026-09-13T00:00:00Z").unwrap(),
        )
    }

    fn dsh_header(id: &str, cwd: &str) -> String {
        serde_json::json!({
            "type": "session",
            "version": 4,
            "id": id,
            "cwd": cwd,
            "createdAt": 1789171200000_i64,
            "delegationDepth": 0
        })
        .to_string()
    }

    fn dsh_message(time: i64, source: &str, text: &str) -> String {
        serde_json::json!({
            "type": "user/message",
            "time": time,
            "data": {
                "role": "user",
                "source": { "kind": source },
                "content": [{ "type": "text", "text": text }]
            }
        })
        .to_string()
    }

    fn write_zstd_frames(path: &std::path::Path, frames: &[String]) {
        let mut bytes = Vec::new();
        for frame in frames {
            bytes.extend(zstd::stream::encode_all(frame.as_bytes(), 0).unwrap());
        }
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn dsh_reads_concatenated_frames_and_filters_injected_messages() {
        let root = tempfile::tempdir().unwrap();
        let session_dir = root.path().join("sessions/project/session-1");
        std::fs::create_dir_all(&session_dir).unwrap();
        let path = session_dir.join("session.v4.jsonl.zstd");
        let first = format!("{}\n", dsh_header("session-1", "E:\\Workspaces\\demo"));
        let second = format!(
            "{}\n{}\n{}\n",
            dsh_message(1789174800000, "runtime-context", "系统注入内容"),
            dsh_message(1789174800000, "user", "完成 DSH 适配"),
            dsh_message(1789174860000, "user", "完成 DSH 适配")
        );
        write_zstd_frames(
            &session_dir.join("session.jsonl.zstd"),
            &[format!(
                "{}\n{}\n",
                dsh_header("session-1", "E:\\Workspaces\\demo"),
                dsh_message(1789174800000, "user", "旧版本内容")
            )],
        );
        write_zstd_frames(&path, &[first, second]);

        let (start, end) = test_bounds();
        let acts = collect_dsh(root.path(), start, end);
        assert_eq!(acts.iter().filter(|a| a.kind == "session").count(), 1);
        let prompts: Vec<_> = acts.iter().filter(|a| a.kind == "prompt").collect();
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].text.as_deref(), Some("完成 DSH 适配"));
        assert_eq!(prompts[0].project, "E:\\Workspaces\\demo");
    }

    #[test]
    fn dsh_reads_raw_logs() {
        let root = tempfile::tempdir().unwrap();
        let session_dir = root.path().join("sessions/project/session-raw");
        std::fs::create_dir_all(&session_dir).unwrap();
        let path = session_dir.join("session.jsonl");
        let mut file = std::fs::File::create(path).unwrap();
        writeln!(file, "{}", dsh_header("session-raw", "E:\\Workspaces\\raw")).unwrap();
        writeln!(
            file,
            "{}",
            dsh_message(1789178400000, "user", "读取原始日志")
        )
        .unwrap();

        let (start, end) = test_bounds();
        let acts = collect_dsh(root.path(), start, end);
        assert_eq!(acts.iter().filter(|a| a.kind == "prompt").count(), 1);
    }

    #[test]
    fn antigravity_extracts_request_and_workspace() {
        let root = tempfile::tempdir().unwrap();
        let id = "conversation-1";
        let log_dir = root
            .path()
            .join("brain")
            .join(id)
            .join(".system_generated/logs");
        std::fs::create_dir_all(&log_dir).unwrap();
        let transcript = log_dir.join("transcript.jsonl");
        let mut file = std::fs::File::create(transcript).unwrap();
        writeln!(
            file,
            "{}",
            serde_json::json!({
                "type": "USER_INPUT",
                "source": "USER_EXPLICIT",
                "created_at": "2026-09-12T08:00:00Z",
                "content": "<USER_REQUEST>实现 Antigravity 适配</USER_REQUEST><ADDITIONAL_METADATA>ignored</ADDITIONAL_METADATA>"
            })
        )
        .unwrap();
        writeln!(
            file,
            "{}",
            serde_json::json!({
                "type": "PLANNER_RESPONSE",
                "created_at": "2026-09-12T08:01:00Z",
                "content": "not a user prompt"
            })
        )
        .unwrap();

        let db = root.path().join("conversation_summaries.db");
        let connection = Connection::open(&db).unwrap();
        connection
            .execute(
                "CREATE TABLE conversation_summaries (conversation_id TEXT, workspace_uris TEXT)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO conversation_summaries (conversation_id, workspace_uris) VALUES (?1, ?2)",
                params![id, r#"["file:///e%3A/Workspaces/antigravity-demo"]"#],
            )
            .unwrap();
        drop(connection);

        let (start, end) = test_bounds();
        let acts = collect_antigravity(root.path(), start, end);
        assert_eq!(acts.iter().filter(|a| a.kind == "session").count(), 1);
        let prompt = acts.iter().find(|a| a.kind == "prompt").unwrap();
        assert_eq!(prompt.text.as_deref(), Some("实现 Antigravity 适配"));
        assert!(prompt.project.contains("Workspaces"));
    }

    #[test]
    fn antigravity_falls_back_when_workspace_db_is_unavailable() {
        let root = tempfile::tempdir().unwrap();
        let log_dir = root
            .path()
            .join("brain")
            .join("conversation-2")
            .join(".system_generated/logs");
        std::fs::create_dir_all(&log_dir).unwrap();
        std::fs::write(
            log_dir.join("transcript.jsonl"),
            serde_json::json!({
                "type": "USER_INPUT",
                "source": "USER_EXPLICIT",
                "created_at": "2026-09-12T08:00:00Z",
                "content": "<USER_REQUEST>使用未知项目继续</USER_REQUEST>"
            })
            .to_string()
                + "\n",
        )
        .unwrap();

        let (start, end) = test_bounds();
        let acts = collect_antigravity(root.path(), start, end);
        assert_eq!(acts[0].project, "(未定位项目)");
    }

    #[test]
    fn bundle_includes_new_tools_in_stats() {
        let tz = parse_tz("Asia/Shanghai").unwrap();
        let payload = bundle(
            NaiveDate::from_ymd_opt(2026, 9, 12).unwrap(),
            tz,
            vec![Activity {
                tool: "dsh".into(),
                project: "demo".into(),
                time: parse_iso("2026-09-12T08:00:00Z").unwrap(),
                text: Some("x".into()),
                kind: "prompt".into(),
            }],
        );
        assert!(payload.stats.iter().any(|s| s.tool == "dsh"));
        assert!(payload.stats.iter().any(|s| s.tool == "antigravity"));
    }
}
