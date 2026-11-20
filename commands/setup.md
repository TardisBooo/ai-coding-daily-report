---
description: Configure the locally installed harness-daily CLI
argument-hint: "[output-dir]"
---

# Set up AI Coding Daily Report

Check that the local `harness-daily --version` reports 0.1.1 or later. If the binary
is missing or older, direct the user to the [verified release install guide](https://github.com/TardisBooo/ai-coding-daily-report/blob/main/docs/install.md#install-the-binary).
Resume setup after the user installs the binary.

Confirm an output directory and a writer CLI with the user. Use the current host when they choose it.
Keep an existing configuration unless the user asks to update it. Initialize the local configuration:

```sh
harness-daily init --host <grok|claude|codex> --out "<output-dir>"
harness-daily doctor
```

Setup ends after initialization. For a local preview, use `harness-daily report --dry-collect`.
An ordinary report passes the report date, project names and paths, and short excerpts of collected
user prompts to the selected writer CLI. The CLI may send that data to its configured provider.

Set a daily OS task only when the user requests automatic reports:

```sh
harness-daily schedule install --time 08:00
```
