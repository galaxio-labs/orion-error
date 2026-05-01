# 0.8 API Contract

更新时间：2026-05-01

本文档固定 `orion-error 0.8.x` 的公开 API 契约。它描述当前承诺的主路径、
分层模块、feature-gated API、稳定快照和协议 JSON 边界。

如果本文档与 `src/`、`tests/`、`examples/` 冲突，以代码和测试为准，并同步修正
本文档。

## 1. Root Exports

crate root 只承诺保留最小主路径入口：

- `StructError`
- `OperationContext`
- `UvsReason`
- derive feature 开启时的 derive 宏：
  - `OrionError`
  - `ErrorCode`
  - `ErrorIdentityProvider`

root 不承诺重新暴露 reason trait、protocol type、snapshot type、report type、
interop type 或测试 helper。它们的正式归属在分层模块中。

`ErrorCode` 作为 derive 宏名字和兼容数值码能力存在；面向外部协议、日志、快照和
监控的稳定机器主键是 `ErrorIdentity.code` / `stable_code()`。

## 2. Prelude

`orion_error::prelude::*` 是新业务代码的推荐导入入口，当前承诺包含：

- `StructError`
- `ErrorWith`
- `ErrorWrapAs`
- `IntoAs`
- derive feature 开启时的 `OrionError`

`prelude` 只放主传播路径需要的最小集合。协议、快照、report、interop 和测试 helper
应从各自分层模块导入。

## 3. Layered Modules

分层模块是非 root 类型和 trait 的正式归属。

- `runtime`
  运行时传播载体和上下文：`StructError`、`StructErrorBuilder`、
  `OperationContext`、`OperationScope`、`WithContext`、`ErrorMetadata`。
- `runtime::source`
  source 观察模型：`SourceFrame`、`SourcePayloadKind`、`SourcePayloadRef`。
- `conversion`
  主路径转换 trait：`IntoAs`、`ErrorWith`、`ErrorWrapAs`、`ErrorConv`、
  `ConvStructError`、`ToStructError`。
- `reason`
  reason trait、分类和内置 reason：`DomainReason`、`ErrorCode`、
  `ErrorIdentityProvider`、`ErrorCategory`、`UvsReason`、`ConfErrReason`。
- `report`
  人类诊断与 redaction：`DiagnosticReport`、`RedactPolicy`。
- `snapshot`
  快照和稳定导出：`ErrorIdentity`、`ErrorSnapshot`、`StableErrorSnapshot`、
  snapshot frame 类型和 `STABLE_SNAPSHOT_SCHEMA_VERSION`。
- `protocol`
  协议/exposure 投影：`DefaultExposurePolicy`、`ExposurePolicy`、
  `ExposureDecision`、`ErrorProtocolSnapshot`、`Visibility`。
- `interop`
  标准错误生态互操作：`StdStructRef`、`OwnedStdStructError`、
  `OwnedDynStdStructError`、`raw_source`、`RawSource`、`RawStdError`。
- `cli`
  CLI 输出辅助：`print_error(...)`。
- `dev::testing`
  测试断言 helper，不属于业务主路径。
- `dev::prelude`
  协议/schema 测试和迁移验证用宽导入，不属于业务主路径。

`bridge::*` 不是 0.8 当前公开分层入口；标准错误生态边界统一称为 `interop`。

## 4. Source Attachment

source 挂载的推荐主路径是：

- `StructError::with_source(...)`
- `StructErrorBuilder::source(...)`

调用者不需要区分 source 是普通 `StdError` 还是下层 `StructError<_>`；路由由 crate
内部完成。

以下 API 保留为维护旧代码、测试 source 分类或调试 auto-routing 的底层入口，
不作为教程和新业务代码的默认推荐：

- `with_std_source(...)`
- `with_struct_source(...)`
- `StructErrorBuilder::source_std(...)`
- `StructErrorBuilder::source_struct(...)`

## 5. Error Flow

当前推荐的错误流转决策：

- 上游是普通错误，第一次进入结构化体系：`source_err(reason, detail)`。
- 上游是 `StructError<R1>`，当前层只改变 reason 类型：`conv_err()`。
- 上游是 `StructError<R1>`，当前层建立新的语义边界：~~`wrap_as(reason, detail)`~~（已废弃，用 `source_err`）。
- 需要挂载 cause 到已有 `StructError`：`with_source(...)` 或 `builder.source(...)`。
- 需要进入 `std::error::Error` 生态：`as_std()`、`into_std()`、
  `into_boxed_std()`、`into_dyn_std()`。

旧 `owe(...)` / `owe_*()` / `err_wrap(...)` / `want(...)` / `with(...)` 不属于
0.8 当前主 API。

## 6. Feature-Gated API

默认 feature：

- `log`
- `derive`

可选 feature：

- `derive`
  开启 root derive 宏 re-export，并启用 `#[derive(OrionError)]` 等宏。
- `log`
  开启 `log` 集成和 `OperationContext` drop 日志路径。
- `tracing`
  开启 `tracing` 集成；同时启用 `log` 和 `tracing` 时，drop 日志优先走
  `tracing` 分支。
- `serde`
  开启主要结构的 `Serialize` / `Deserialize` 支持。
- `serde_json`
  开启 stable snapshot 和 protocol JSON projection 方法：
  `to_stable_snapshot_json()`、`to_http_error_json()`、`to_cli_error_json()`、
  `to_log_error_json()`、`to_rpc_error_json()`。
- `anyhow`
  开启 `anyhow::Error` 进入 `source_err(...)` 的适配，并支持官方 dyn interop
  wrapper 的结构化 source 恢复。
- `toml`
  开启 `toml::de::Error` / `toml::ser::Error` 进入 `source_err(...)` 的适配。

文档示例如果依赖 feature，应显式说明或用测试门控覆盖。

## 7. Stable Snapshot

稳定快照主入口：

- `snapshot()`
- `into_snapshot()`
- `snapshot().stable_export()`
- `to_stable_snapshot_json()`，需要 `serde_json`

稳定 schema 版本由 `STABLE_SNAPSHOT_SCHEMA_VERSION` 标识，当前为
`orion-error.snapshot.v3`。

`StableErrorSnapshot` 是跨进程、跨版本消费的稳定导出对象。稳定导出刻意剥离部分
runtime-only 兼容投影，例如 ad-hoc fields 和 operation result。需要完整运行时细节时，
使用 `ErrorSnapshot`，不要经由 stable export 再还原。

## 8. Protocol JSON

协议投影主入口：

- `identity_snapshot()`
- `exposure_snapshot(...)`
- `into_exposure_snapshot(...)`
- `ErrorProtocolSnapshot::to_http_error_json()`
- `ErrorProtocolSnapshot::to_cli_error_json()`
- `ErrorProtocolSnapshot::to_log_error_json()`
- `ErrorProtocolSnapshot::to_rpc_error_json()`

`ErrorProtocolSnapshot` 的稳定输入由三部分组成：

- `identity`
- `decision`
- embedded `DiagnosticReport`

稳定承诺：

- `identity.code` 是协议、日志、监控、测试断言的稳定机器主键。
- `identity.category` 是稳定分类。
- `ExposureDecision` 的字段名和含义稳定：`http_status`、`visibility`、
  `default_hints`、`retryable`。
- HTTP / CLI / log / RPC projection 的顶层用途稳定。

不承诺：

- `render_user_debug()` 的文本格式不是机器协议。
- JSON 中用于人工排障的 `summary` / `rendered_detail` 文本不作为精确稳定 schema。
- `source_frames` 的 `debug`、`display`、`type_name` 等诊断字段可能随实现调整。
- 未在 `docs/protocol-contract.md` 和测试中锁定的内部 helper 字段不作为公共协议。

## 9. Report And Redaction

`DiagnosticReport` 面向人类诊断，不要求 reason 实现 `ErrorIdentityProvider`。

主入口：

- `report()`
- `into_report()`
- `render()`
- `render_redacted(...)`
- `report_redacted(...)`

redaction 适用于 report、protocol projection 和 source frame 诊断视图。机器协议中的
稳定 code/category 不应被当成自然语言 detail 处理。

## 10. Compatibility Policy

0.8 当前策略：

- 保持主路径稳定。
- 保持 observation surface 可用，但不把它们放进 quick start。
- 保持 `dev::*` 面向测试和迁移验证。
- 不恢复 0.6 / 0.7 legacy API 作为 root 或 prelude 主路径。
- archive 文档保留历史语境，不代表当前推荐用法。
