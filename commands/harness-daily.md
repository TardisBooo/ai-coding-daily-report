---
description: 扫描本机 AI coding harness 会话并生成日报
argument-hint: "[--date YYYY-MM-DD]"
---

按用户要求运行本机采集器。普通报告会把日期、项目名、项目路径和少量提问片段交给已配置的写作 CLI。
该 CLI 可能把这些字段发给已配置的模型服务。只检查本地采集结果时，使用 `--dry-collect`。

```bash
harness-daily report $ARGUMENTS
```

若二进制不在 PATH：

- Windows：`%LOCALAPPDATA%\harness-daily\harness-daily.exe`
- macOS / Linux：`~/.local/bin/harness-daily`

尚未初始化时，先按 [安装指南](https://github.com/TardisBooo/ai-coding-daily-report/blob/main/docs/install.zh-CN.md) 请用户选择写作 CLI 和输出目录。
