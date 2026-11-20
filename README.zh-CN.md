<div align="center">

# AI 编程日报

**汇总本地 AI 编码会话，用 Grok Build、Claude Code 或 Codex 生成 Markdown 日报。**

_可直接使用 `harness-daily` CLI，也可安装宿主插件。_

[![CI](https://github.com/TardisBooo/ai-coding-daily-report/actions/workflows/ci.yml/badge.svg)](https://github.com/TardisBooo/ai-coding-daily-report/actions/workflows/ci.yml)
[![Release](https://github.com/TardisBooo/ai-coding-daily-report/actions/workflows/release.yml/badge.svg)](https://github.com/TardisBooo/ai-coding-daily-report/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-32CD32.svg)](LICENSE)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![Platforms](https://img.shields.io/badge/platform-windows%20%7C%20macos%20%7C%20linux-blue)

[English](README.md) | **简体中文**

</div>

---

## 📋 快速导航

[为什么需要 harness-daily？](#-为什么需要-harness-daily) ·
[工作原理](#-工作原理) ·
[环境要求](#-环境要求) ·
[安装](#-安装) ·
[安装教程](docs/install.zh-CN.md) ·
[快速上手](#-快速上手) ·
[命令](#-命令) ·
[配置](#-配置) ·
[日报格式](#-日报格式) ·
[扩展](#-扩展) ·
[开发](#-开发) ·
[参与贡献](#-参与贡献) ·
[许可](#-许可)

---

## 🤔 为什么需要 harness-daily？

同时用多个 AI 编码工具（Codex、Claude Code、Grok Build、Pi、OMP、DSH、Antigravity）的人，一天的工作散落在
几十个会话、若干个项目里。写日报 = 手工翻历史。

harness-daily 替你做：

- **共用 CLI 和技能**。插件可装进 Grok Build、Claude Code 或 Codex。
- **在本机采集**。CLI 读取受支持的会话日志。生成日报时会把日期、项目名、项目路径和提问片段
  交给选定的 CLI。每个项目最多 12 条，每条最多 180 字符。该 CLI 可能把这些字段发给已配置的模型服务。
- **正文由当前 agent CLI 撰写** —— Grok Build、Claude Code 或 Codex，用你已有的登录态。不需要第二个 API key。`init --host auto` 按 grok → claude → codex 选用第一套能找到的。
- **企业日报格式** —— 「今日工作」按项目合并成完整工作条目；原始提问放审计附录，不混进总结。
- **自愈调度** —— 系统任务每天执行 `report --auto --backfill 7`，关机错过 08:00，开机自动补。

```text
08:00（系统任务）→ harness-daily report
   ├─ 发现数据目录   （codex / claude / grok / pi / omp / dsh / antigravity）
   ├─ 采集           （解析 JSONL、去重、剔除「继续/ok/连通性测试」）
   ├─ 写正文         grok --prompt-file / claude -p / codex exec（你的登录态与默认模型）
   └─ 渲染           ~/work-logs/日报-YYYY-MM-DD.md   ← 总结 · 明细 · 问题 · 统计 · 审计
```

## 🧭 工作原理

| 组件 | 职责 |
|---|---|
| `harness-daily`（Rust 二进制） | 发现数据目录、解析会话、按项目聚合、调用写正文 CLI、渲染 Markdown、安装系统定时任务 |
| 通用插件（`skills/`、`commands/`、`plugin.json`、`.claude-plugin/`、`.grok-plugin/`、`plugin.yaml`） | 同一仓库可装进 **Grok Build / Claude Code / Codex** |
| 写正文宿主 | `grok --prompt-file` · `claude -p --output-format json` · `codex exec`（stdin） |
| 系统调度器 | Windows `schtasks` / macOS `launchd` / Linux cron，每天触发 `report --auto --backfill 7` |

内置支持：**Codex CLI**（`~/.codex`）、**Claude Code**（`~/.claude`）、**Grok Build**
（`~/.grok`）、**Pi**（`~/.pi`）、**OMP**（`~/.omp`）、**DSH**（`~/.dsh`）和
**Antigravity**（`~/.gemini/antigravity`，CLI 数据目录回退到 `~/.gemini/antigravity-cli`）。数据目录搬家了，把 `[harnesses.<id>].data_dir` 指过去即可。

## ✅ 环境要求

- 至少安装并**登录** [Grok Build](https://x.ai/cli)、[Claude Code](https://docs.anthropic.com/en/docs/claude-code) 或 [Codex CLI](https://github.com/openai/codex) 之一
- Windows / macOS / Linux
- 可选：[Rust](https://rustup.rs)（源码构建时）

## 📦 安装

先按固定版本下载并校验二进制。需要在会话中调用 CLI 时，再安装插件。
见 [安装教程](docs/install.zh-CN.md) · [English](docs/install.md)。

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

### Codex CLI

```sh
codex plugin marketplace add TardisBooo/ai-coding-daily-report
codex plugin add harness-daily@harness-daily
```

### 只安装二进制

按 [二进制安装步骤](docs/install.zh-CN.md#安装二进制) 安装后，直接初始化 CLI。

## 🚀 快速上手

```sh
harness-daily init --host codex --out "<日报目录>"
harness-daily doctor
harness-daily report --dry-collect
```

`--host` 也可以填 `grok` 或 `claude`。检查采集内容后，运行 `harness-daily report`。
生成指定日期的日报，或补齐上周的日报：

```sh
harness-daily report --date 2026-09-16
harness-daily report --backfill 7 --auto
```

如果需要每天自动生成，再启用系统定时任务：

```sh
harness-daily schedule install --time 08:00
```

## 🎛 命令

```
harness-daily init --host auto|grok|claude|codex [--out DIR] [--time 08:00]
harness-daily scan                                          列出探测到的数据目录
harness-daily doctor                                        自检（写正文 CLI、路径、输出目录）
harness-daily report [--date D] [--backfill N] [--auto]
                         [--dry-collect] [--out DIR]        生成日报
harness-daily schedule install|remove|status|run            管理系统定时任务
```

`--auto` 跳过已存在的日报文件；`--dry-collect` 只输出采集 JSON，不调用模型（调试用）。

## ⚙️ 配置

见 [各系统的配置路径](docs/install.zh-CN.md#卸载)。

```toml
timezone    = "Asia/Shanghai"
output_dir  = '/path/to/work-logs'
report_time = "08:00"
backfill_days = 7
extra_roots = []            # 额外扫描根（搬过家的数据目录）

[writer]
host = "auto"               # grok | claude | codex | auto
bin  = ""                   # 空=自动探测；或填绝对路径

[report]
include_audit_log = true    # false 可关闭原始提问附录
include_stats     = true

[harnesses.codex]
enabled  = true
data_dir = ""               # ~/.codex 搬家后在此覆盖

[harnesses.dsh]
enabled  = true
data_dir = ""               # ~/.dsh 搬家后在此覆盖

[harnesses.antigravity]
enabled  = true
data_dir = ""               # ~/.gemini/antigravity 搬家后在此覆盖
```

## 📄 日报格式

```markdown
# 工作日报 · 2026-09-16（周三）
## 一、今日工作总结      ← 按项目组织，合并后的工作条目（不写工具、时间）
## 二、工作明细          ← 路径 · 时段 · 完成内容
## 三、问题与待办        ← 模型提取的待跟进问题
## 四、工作量统计        ← 各 harness 会话数 / 提问数
## 附录：操作审计日志    ← 当日全部原始提问，按时间排序
```

## 🧩 扩展

- **新增 harness / 桌面端**：复制 [`adapters/example-jsonl.toml`](adapters/example-jsonl.toml)
  到配置目录改路径/字段；内置 Rust 适配器在 `src/collect.rs`，欢迎 PR（见
  [`CONTRIBUTING.md`](CONTRIBUTING.md)）。
- **写正文 CLI**：`--host grok|claude|codex|auto`。采集覆盖内置七类来源。
- **隐私**。`report --dry-collect` 仅在本地写采集文件，不调用写作 CLI。审计附录可能包含原始提问，可在配置中关闭。

## 🛠 开发

```sh
cargo test                 # 单元测试（JSON 解析、时区边界、噪声过滤）
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo run -- scan           # 用你真实的机器试探测
```

目录结构：`src/collect.rs`（各 harness 采集器）· `src/writer.rs`（Grok / Claude / Codex 无头调用与严格 JSON
恢复）· `src/render.rs`（Markdown 拼装）· `src/schedule.rs`（三平台调度器）· `skills/` +
`commands/` + `.claude-plugin/` + `.grok-plugin/` + `plugin.yaml`（通用插件）。

## 🤝 参与贡献

欢迎提交 bug 报告与新的 harness 适配器 —— 见 [CONTRIBUTING.md](CONTRIBUTING.md)。
安全问题见 [SECURITY.md](SECURITY.md)。

## 📜 许可

[MIT](LICENSE)
