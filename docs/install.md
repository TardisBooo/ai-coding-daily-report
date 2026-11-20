# Install AI Coding Daily Report

Install the `harness-daily` binary. If you use an agent in a terminal, you can also
install its plugin to call the same CLI from a session.

[简体中文](install.zh-CN.md)

## Install the binary

Open the [v0.1.1 release](https://github.com/TardisBooo/ai-coding-daily-report/releases/tag/v0.1.1). Download the ZIP for your platform.

| Platform | ZIP suffix |
|---|---|
| Windows x64 | `x86_64-pc-windows-msvc` |
| macOS Apple silicon | `aarch64-apple-darwin` |
| macOS Intel | `x86_64-apple-darwin` |
| Linux x64 with glibc | `x86_64-unknown-linux-gnu` |
| Linux x64 with musl | `x86_64-unknown-linux-musl` |

Find the downloaded file's `digest` under `assets` in the [v0.1.1 release metadata](https://api.github.com/repos/TardisBooo/ai-coding-daily-report/releases/tags/v0.1.1).
Compare its SHA-256 with the file hash before extracting:

- Windows PowerShell: `Get-FileHash -Algorithm SHA256 -LiteralPath "<downloaded ZIP>"`
- macOS: `shasum -a 256 "<downloaded ZIP>"`
- Linux: `sha256sum "<downloaded ZIP>"`

When the hash matches, extract the ZIP and place `harness-daily.exe` or `harness-daily`
in a directory on your PATH. Check it with `harness-daily --version`. If you build from
source, use a checkout at the [release tag](https://github.com/TardisBooo/ai-coding-daily-report/tree/v0.1.1)
and run `cargo install --locked --path .`.

## Install a plugin

### Claude Code

```text
/plugin marketplace add TardisBooo/ai-coding-daily-report
/plugin install harness-daily@harness-daily
/harness-daily:setup
```

### Grok Build

```sh
grok plugin install TardisBooo/ai-coding-daily-report@v0.1.1
```

Review the plugin and confirm trust in Grok Build. Then run `/harness-daily:setup`.

### Codex CLI

```sh
codex plugin marketplace add TardisBooo/ai-coding-daily-report
codex plugin add harness-daily@harness-daily
```

Use the CLI initialization below. In a Codex session, the `$harness-daily:harness-daily` skill
invokes the same report tool.

## Preview your first report

Choose the installed, signed-in writer CLI you want to use. Pass `grok`, `claude`,
or `codex` to `--host`.

```sh
harness-daily init --host codex --out "<reports-folder>"
harness-daily doctor
harness-daily report --dry-collect
```

Use `harness-daily scan` to review the session roots. To limit collection, set
`enabled = false` for unwanted sources under `[harnesses.<id>]` in the config.
Set `data_dir` there to use a moved source. Codex discovery checks `CODEX_HOME`.

The preview writes a collection file. An ordinary `harness-daily report` asks the
writer to generate yesterday's Markdown. It sends the report date, project names
and paths, and up to 12 prompt excerpts of up to 180 characters per project to
your selected writer CLI. The writer CLI may send these fields to its configured
model provider. Review the preview before prompting the writer if your work may
contain sensitive material.

## Enable or remove daily scheduling

After reviewing the report and sources, enable an OS task if you want automatic
reports:

```sh
harness-daily schedule install --time 08:00
harness-daily schedule status
```

On Linux, follow the printed cron instructions. `harness-daily schedule remove`
removes a registered task or prints how to remove the cron line.

## Uninstall

If you enabled scheduling, remove it before removing the binary. Uninstall
`harness-daily@harness-daily` using your host's plugin UI or CLI. Remove the
binary from your PATH directory and its installed copy if you ran `init`.
You can keep the report folder.

The config is in `%APPDATA%/harness-daily` on Windows,
`~/Library/Application Support/harness-daily` on macOS, and
`${XDG_CONFIG_HOME:-$HOME/.config}/harness-daily` on Linux. To isolate an
interactive run, set `HARNESS_DAILY_HOME` to the directory for its config and
initialized copy of the binary.
