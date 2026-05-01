# 文档导航

建议阅读顺序：先看 `user/` 了解概念和用法，遇到边界问题再查 `dev/`。

---

## [user/](user/) — 使用者

| 文档 | 内容 |
|------|------|
| [使用教程](./user/tutorial.md) | 从零开始，推荐第一份文档 |
| [OrionError 与稳定身份](./user/reason-identity-guide.md) | 如何定义和设计领域 reason |
| [协议契约](./user/protocol-contract.md) | exposure 投影的输出契约 |
| [Stable Snapshot Schema](./user/stable-snapshot-schema.md) | 稳定快照的结构和版本化 |
| [Report / Exposure Boundary](./user/report-exposure-boundary.md) | 诊断报告与 exposure 的分界 |
| [日志说明](./user/LOGGING.md) | OperationContext 日志集成 |
| [与 thiserror 的关系](./user/thiserror-comparison.md) | 与 thiserror 的差异和配合 |
| [与生态方案对比](./user/ecosystem-comparison.md) | anyhow / thiserror / color-eyre / orion-error |
| [设计约束](./user/design-constraints.md) | orphan rule 限制等已知约束 |

## [dev/](dev/) — 设计 / 开发者

| 文档 | 内容 |
|------|------|
| [API Contract](./dev/api-contract.md) | 当前公开 API 的职责和边界 |
| [Public Surface Grading](./dev/public-surface-grading.md) | 分层导出的评分和守卫 |
| [Release Checklist](./dev/release-checklist.md) | 发布前的检查列表 |
| [性能基准](./dev/perf/structerror-allocation.md) | StructError 堆分配性能 |
| [Source Debug 性能](./dev/perf/structerror-source-debug.md) | SourceFrame Debug 格式化性能 |

---

*文档和实现冲突时，以 `src/`、`tests/`、`examples/` 为准。*
