# 安装 AI 编程日报

先安装 `harness-daily` 二进制。你也可以在常用的编码工具里安装插件，从会话中调用同一个 CLI。

[English](install.md)

## 安装二进制

打开 [v0.1.1 发布页](https://github.com/TardisBooo/ai-coding-daily-report/releases/tag/v0.1.1)，选择平台对应的 ZIP。

| 平台 | ZIP 文件后缀 |
|---|---|
| Windows x64 | `x86_64-pc-windows-msvc` |
| macOS Apple 芯片 | `aarch64-apple-darwin` |
| macOS Intel | `x86_64-apple-darwin` |
| Linux x64，glibc | `x86_64-unknown-linux-gnu` |
| Linux x64，musl | `x86_64-unknown-linux-musl` |

打开 [v0.1.1 发布元数据](https://api.github.com/repos/TardisBooo/ai-coding-daily-report/releases/tags/v0.1.1)，
在 `assets` 中找到对应文件的 `digest`。用以下命令计算 ZIP 的 SHA-256，核对一致后再解压。

- Windows PowerShell 使用 `Get-FileHash -Algorithm SHA256 -LiteralPath "<下载的 ZIP>"`。
- macOS 使用 `shasum -a 256 "<下载的 ZIP>"`。
- Linux 使用 `sha256sum "<下载的 ZIP>"`。

把解压出的 `harness-daily.exe` 或 `harness-daily` 放到 PATH 下的目录，再运行
`harness-daily --version`。如果从源码安装，请检出 [发布标签](https://github.com/TardisBooo/ai-coding-daily-report/tree/v0.1.1) 后运行 `cargo install --locked --path .`。

## 安装插件

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

在 Grok Build 审阅插件并确认信任，再运行 `/harness-daily:setup`。

### Codex CLI

```sh
codex plugin marketplace add TardisBooo/ai-coding-daily-report
codex plugin add harness-daily@harness-daily
```

接着在终端初始化。在 Codex 会话中，用 `$harness-daily:harness-daily` 技能调用同一个工具。

## 预览第一份日报

选择已登录的写作 CLI。`--host` 可填 `grok`、`claude` 或 `codex`。

```sh
harness-daily init --host codex --out "<日报目录>"
harness-daily doctor
harness-daily report --dry-collect
```

用 `harness-daily scan` 确认会话目录。只希望采集部分来源时，在配置的 `[harnesses.<id>]`
中把其余来源的 `enabled` 设为 `false`，或用 `data_dir` 指向迁移后的目录。Codex 支持按 `CODEX_HOME` 查找。

预览只在本地写采集文件。运行 `harness-daily report` 才生成昨天的 Markdown 日报。该命令会把
日期、项目名、项目路径和提问片段交给选定的 CLI。每个项目最多 12 条，每条最多 180 字符。
该 CLI 可能把这些字段发给已配置的模型服务。会话里可能包含敏感数据时，先检查预览。

## 启用或移除日报定时任务

确认日报和来源后，如需自动生成，再执行：

```sh
harness-daily schedule install --time 08:00
harness-daily schedule status
```

Linux 按输出配置 cron。`harness-daily schedule remove` 移除已注册的任务，或提示如何移除 cron 条目。

## 卸载

如果已启动定时任务，先移除任务。然后在宿主里卸载 `harness-daily@harness-daily`，
删除 PATH 中的二进制以及 `init` 创建的副本。日报文件可以留在原目录。

Windows 配置在 `%APPDATA%/harness-daily`，macOS 配置在 `~/Library/Application Support/harness-daily`，
Linux 配置在 `${XDG_CONFIG_HOME:-$HOME/.config}/harness-daily`。隔离运行时，可用
`HARNESS_DAILY_HOME` 指定配置与初始化二进制副本的目录。
