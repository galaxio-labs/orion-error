# 文档导航

当前文档以 `orion-error 0.7.0` 为准：

- `V1` 主路径已经收口完成
- `V2` 第一阶段主路径已经可用
- `V3` 已开始冻结最小落地范围，但当前仍处于规划启动阶段
- 如果 `V1` 文档、`V2` 计划文档与源码冲突，以 `src/`、测试和顶层 `README` 为准

建议阅读顺序：

1. [顶层 README](../README.md)
2. [V2 开发计划](./v2-development-plan.md)
3. [V2 Runtime / Snapshot / Report 分层草案](./v2-runtime-snapshot-report-layering.md)
4. [V2 Stable Snapshot Schema](./v2-stable-snapshot-schema.md)
5. [V2 Bridge / Source Payload 草案](./v2-bridge-source-payload.md)
6. [V2 StructError: StdError 策略](./v2-structerror-stderror-strategy.md)
7. [V2 Compat Deprecation Plan](./v2-compat-deprecation-plan.md)
8. [V3 最小落地计划](./v3-minimum-plan.md)
9. [V3 Stable Code Policy](./v3-stable-code-policy.md)
10. [V3 协议契约](./v3-protocol-contract.md)
11. [V1 修复与评审基线](./v1-fix-and-review-plan.md)
12. [V1 结案说明](./v1-closure-summary.md)
13. [使用教程](./tutorial.md)
14. [日志说明](./LOGGING.md)
15. [与 thiserror 的配合](./thiserror-comparison.md)
16. [设计文档目录](./error-handling/README.md)

## 重要说明

旧版本文档中常见的过期写法包括：

- `orion-error = "0.2"` / `"0.3"` / `"0.4"`
- `impl DomainReason for MyError {}`
- 旧的 `ctx` 链式附加键值写法
- `UvsReason::validation_error("message")`
- `with_exit_log()`

当前版本对应写法：

- `orion-error = "0.7.0"`
- 一般不需要手写 `DomainReason`
- 使用 `ctx.record("key", "value")`
- 使用 `StructError::from(UvsReason::validation_error()).with_detail("message")`
- 使用 `with_auto_log()`
- `OperationContext::doing("op")` / `OperationContext::at("resource")` 已写入 V2 的 `action` / `locator` 语义字段，并继续保留 `target` / `path` 兼容投影
- 普通错误优先 `into_as(...)`；已是 `StructError<_>` 的跨层传播优先 `err_conv()` / `wrap_as(...)`
- V1 兼容路径已可通过 `orion_error::v1::*` 显式进入
- V2 新代码优先使用 `orion_error::v2::*` 或 `orion_error::v2::prelude::*`
- V3 当前只冻结下一阶段协议方向；当前行为仍以 `V2` 实现和测试为准
- V3 已开始引入最小 enforcement：`scripts/check-v3-policy.sh`
- V3 当前协议落地面的正式归档见 `docs/v3-protocol-contract.md`
- V3 当前已可用的最小消费接口包括：
  - `StructError::identity_snapshot()`
  - `StructError::policy_report()`
  - `StructError::into_policy_report()`
  - `StructError::policy_snapshot(...)`
  - `StructError::http_response(...)`
  - `StructError::cli_response(...)`
  - `StructError::log_response(...)`
  - `StructError::rpc_response(...)`
  - `assert_err_code(...)` / `assert_err_category(...)` / `assert_err_identity(...)`
- V2 已提供按层导入入口：
  - `orion_error::runtime::*`
  - `orion_error::conversion::*`
  - `orion_error::snapshot::*`
  - `orion_error::report::*`
  - `orion_error::bridge::*`
  - `orion_error::reason::*`
- V2 已提供统一 source 主路径：
  - `StructError::attach_source(...)`
  - `StructErrorBuilder::attach_source(...)`
  - `IntoSourcePayload`

## V1 迁移主路径

V1 推荐的新调用主路径是：

- 普通错误第一次进入结构化体系：`into_as(...)`
- 已结构化错误向上层建立新边界：`wrap_as(...)`
- 普通 source：`with_std_source(...)`
- 结构化 source：`with_struct_source(...)`
- 完整上下文帧：`with_context(...)`
- 上下文语义糖衣：`at(...)` / `doing(...)`

已退出主路径、仅保留兼容或便捷糖衣的旧/旧式 API：

- `with_source(...)`
- `builder.source(...)`
- `err_wrap(...)`
- `wrap(...)`
- `owe_source(...)`
- `owe_*_source()`
- `owe_*()`

其中：

- `with_source(...)` / `builder.source(...)` 当前仍作为 `IntoSourcePayload` 自动分流糖衣保留
- 新代码若希望 source 通道语义在调用点显式可见，仍优先使用 `with_std_source(...)` / `with_struct_source(...)` 与 `source_std(...)` / `source_struct(...)`

已正式进入 `#[deprecated]` 的旧 API：

- 旧的 `OperationContext` target helper
- 旧的 `OperationContext` 复合 helper 写法
- 旧的 `ErrorWith` target helper
- 旧的 `ErrorWith` context helper
- `owe(...)`

如果其他文档与源码冲突，请以 `src/`、测试和顶层 README 为准。
