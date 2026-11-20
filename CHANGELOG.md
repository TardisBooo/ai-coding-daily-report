# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to
[Semantic Versioning](https://semver.org/).

## [0.1.1]

### Added

- Built-in DSH and Antigravity session collectors.
- DSH zstd JSONL discovery and Antigravity transcript workspace mapping.
- Native Codex plugin marketplace metadata.

### Changed

- Documented as a **universal plugin** for Grok Build, Claude Code, and Codex (same repo,
  same skills/commands). GitHub About, crate description, CLI `--help`, and report footer
  no longer say the body is written only by Grok Build CLI.
- AI Coding Daily Report is the repository's display name. Plugin and CLI IDs stay `harness-daily`.
- Setup uses an installed, verified binary. Scheduling is a separate action.

### Fixed

- Scheduled Grok writer no longer injects the harness-daily skill / MCP tools
  (max-turns cancel left 日报-YYYY-MM-DD.md unwritten). Headless call is
  tool-restricted, logs writer stdout/stderr, and Windows schtasks now runs on
  battery, starts when available, and appends `task.log`.
- `--auto --backfill` continues remaining dates if one day fails.

### Added

- Universal writer hosts: `--host auto|grok|claude|codex` (`grok --prompt-file`,
  `claude -p`, `codex exec` on stdin).
- Plugin manifests for Claude Code and Codex in the same repository as the Grok plugin.
- Marketplace install and local setup for the Grok and Claude plugins.

## [0.1.0] — 2026-09-17

### Added

- Collectors for Codex CLI (`~/.codex`), Claude Code (`~/.claude`), Grok Build (`~/.grok`),
  Pi (`~/.pi`), OMP (`~/.omp`) with junction-aware root discovery on Windows.
- Report pipeline: per-project prompt aggregation → noise/smoke-test filtering →
  body written by the current agent CLI (`grok --prompt-file` / `claude -p` / `codex exec`,
  strict JSON with multi-shape recovery) → Markdown render.
- Enterprise-style report: summary (projects only), details, issues, per-harness stats,
  and an optional raw-prompt audit appendix.
- `init` / `scan` / `doctor` / `report` / `schedule` commands.
- OS scheduling: Windows `schtasks`, macOS `launchd`, Linux cron guidance;
  `--auto --backfill N` self-healing for missed days.
- Plugin layout (`skills/harness-daily`, `/harness-daily` slash command); later extended
  as a universal plugin for Grok / Claude / Codex.
- CI (fmt + clippy + test on Windows/macOS/Linux) and release workflow with
  per-platform binaries + install scripts.

[0.1.1]: https://github.com/TardisBooo/ai-coding-daily-report/releases/tag/v0.1.1
[0.1.0]: https://github.com/TardisBooo/ai-coding-daily-report/releases/tag/v0.1.0
