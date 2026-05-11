# orion-error 中文文档

`orion-error` 就是 WuKong 错误治理模型在 Rust 中的一种工程实现。

在文档首页，最重要的定位先说明清楚：

- **契约通道**：稳定 identity、category、retryable、visibility
- **诊断通道**：detail、source chain、操作上下文、关键字段
- **适配输出**：按策略生成 HTTP / RPC / CLI / 日志投影视图

在这个 crate 里，这些理念落到：

- 用 `#[derive(OrionError)]` 定义稳定语义身份
- 用 `StructError<R>` 作为统一运行时载体
- 用 `source_err(...)` 处理首次进入和新语义边界包装
- 用 `conv_err()` 做 reason 收敛，不重写错误叙事
- 用 `report()` / `identity_snapshot()` / `exposure(...)` 做边界输出

`orion-error` 面向 Rust 服务中的结构化错误治理：让错误在跨层传播时保留稳定身份、上下文、来源链、日志材料和协议暴露视图，而不是退化成不可治理的字符串。

推荐先阅读“为什么需要 orion-error”，再进入教程和协议文档。

## 用户文档

| 文档 | 内容 |
|------|------|
| [为什么需要 orion-error](./user/why-orion-error.md) | 解释错误治理的核心问题：环境信息、技术细节抽象、错误链、日志和多视图呈现 |
| [使用教程](./user/tutorial.md) | 从定义 reason、构造 `StructError`、使用 `source_err` / `conv_err` 到输出报告 |
| [OrionError 与稳定身份](./user/reason-identity-guide.md) | 说明 `ErrorIdentity.code`、业务 reason、透明 `UvsReason` 变体的设计 |
| [协议契约](./user/protocol-contract.md) | 说明 HTTP / RPC / CLI / log 投影的稳定边界 |
| [Report / Exposure 边界](./user/report-exposure-boundary.md) | 区分内部诊断报告和对外暴露视图 |
| [日志说明](./user/LOGGING.md) | 说明如何在错误边界输出有效日志，避免到处散落日志代码 |
| [生态方案对比](./user/ecosystem-comparison.md) | 对比 `anyhow`、`thiserror`、`color-eyre` 和 `orion-error` 的适用边界 |
| [与 thiserror 的关系](./user/thiserror-comparison.md) | 说明两者不是简单替代关系，分别适合不同层级 |
| [大型工程错误治理宣言](./user/manifesto.md) | WuKong 模型、治理原则与工业级验证 |
| [设计约束](./user/design-constraints.md) | 说明 orphan rule 等 Rust 语言约束下的 API 取舍 |

## 开发文档

| 文档 | 内容 |
|------|------|
| [API Contract](./dev/api-contract.md) | 0.8 公共 API、分层模块、feature-gated API 和稳定快照契约 |
| [兼容与迁移](./dev/compat-migration.md) | 旧 API 到当前 API 的迁移说明 |
| [Public Surface Grading](./dev/public-surface-grading.md) | 公共暴露面的分级评估 |
| [Release Checklist](./dev/release-checklist.md) | 发布前检查项 |
| [StructError Allocation](./dev/perf/structerror-allocation.md) | 分配行为与性能记录 |
| [StructError Source Debug](./dev/perf/structerror-source-debug.md) | source debug 路径的性能记录 |

## 当前主路径 API

新代码优先使用：

- `reason.to_err()`：把单个 reason 转成 `StructError`
- `result.source_err(reason, detail)`：让普通错误进入结构化错误系统，或建立新的上层语义边界
- `result.conv_err()`：对已有 `StructError<R1>` 做 reason-only 类型转换
- `err.with_source(source)` / `StructError::builder(reason).source(source)`：自动识别 raw std error 或下层 `StructError` source
- `OperationContext::doing(...).with_field(...).with_meta(...)`：链式携带结构化上下文

稳定外部身份使用 `ErrorIdentity.code`。`ErrorCode` 是兼容数字码，不应作为主要治理身份。

## English

English documentation starts from [orion-error Documentation](../en/).
