# 协议契约

更新时间：2026-04-23

本文档描述 `orion-error` 当前已经实际落地的协议面。

这里不是新的规划文档，而是对当前源码、测试和对外导出面的归档。

如果本文档与实现冲突，以 `src/`、测试和顶层 `README` 为准。

## 1. 当前范围

当前已经落地的协议由三层组成：

1. 稳定身份
2. policy decision
3. 出口投影

当前覆盖的出口投影包括：

- `HTTP`
- `CLI`
- `log`
- `RPC`
- `human debug summary`
- `test helper`

当前还没有单独的 typed governance/export 协议文档；治理系统如果需要稳定主键和消费规则，应优先复用本文档列出的 `identity_snapshot()`、`policy_snapshot()` 和各 projection。

## 2. 稳定身份

稳定身份由 [src/core/snapshot.rs](/Users/zuowenjian/devspace/wp-labs/dev/crates/orion-error/src/core/snapshot.rs) 中的 `ErrorIdentity` 表达：

- `code`
- `category`
- `reason`
- `detail`
- `position`
- `want`
- `path`

设计约束：

- `code` 是长期稳定主键
- `category` 是稳定分类位
- `reason` 是稳定展示语义的一部分，但不是唯一主键
- `detail` 不是身份主键，只是补充说明

主调用入口：

- `StructError::identity_snapshot()`
- `assert_err_code(...)`
- `assert_err_category(...)`
- `assert_err_identity(...)`

## 3. Policy Decision

policy decision 由 [src/core/report.rs](/Users/zuowenjian/devspace/wp-labs/dev/crates/orion-error/src/core/report.rs) 中的 `ErrorPolicyDecision` 表达：

- `http_status`
- `visibility`
- `default_hints`
- `retryable`

当前默认 policy 为 `DefaultErrorPolicy`。

当前默认规则：

- `Biz -> 400 + Public`
- `Conf/Logic/Sys -> 500 + Internal`
- `sys.network_error` / `sys.timeout` 默认为 `retryable = true`
- 其他内建稳定 code 默认 `retryable = false`

`retryable` 已经属于 policy 语义，不再是 `RPC` projection 的本地硬编码字段。

主调用入口：

- `ErrorPolicy::decide(...)`
- `ErrorPolicyInput::decision(...)`
- `StructError::policy_snapshot(...)`
- `ErrorReport::policy_snapshot(...)`

## 4. Policy Snapshot

`policy snapshot` 是当前最完整的统一协议输入，结构为：

- `identity`
- `decision`
- `report`

对应 schema 常量：

- `POLICY_SNAPSHOT_TOP_LEVEL_FIELDS`
- `POLICY_DECISION_FIELDS`

主调用入口：

- `StructError::policy_snapshot(...)`
- `StructError::into_policy_snapshot(...)`
- `ErrorReport::policy_snapshot(...)`
- `ErrorReport::to_policy_snapshot_json(...)`
- `ErrorReport::to_policy_report_json(...)`
- `StructError::render_user_debug(...)`
- `StructError::render_user_debug_redacted(...)`

适用场景：

- 测试快照
- 统一导出
- 上层 API/网关自己做二次投影

不建议直接把 CLI 文本当成机器接口协议。

`render_user_debug(...)` 的定位是“给人读的调试摘要”，用于本地调试、示例展示和人工排障。
它不是 `HTTP` public message 的替代物，当前也不会按 `Visibility` 自动裁剪成对外暴露文案。

## 5. HTTP Projection

HTTP projection 结构：

- `status`
- `code`
- `category`
- `message`
- `visibility`
- `hints`

对应类型与常量：

- `ErrorHttpResponse`
- `HTTP_ERROR_RESPONSE_FIELDS`

默认规则：

- `Public` 可见错误优先暴露 `detail`
- `Internal` 可见错误默认只暴露稳定 `reason`

主调用入口：

- `StructError::http_response(...)`
- `ErrorReport::http_response(...)`
- `ErrorProtocolSnapshot::http_response()`
- `to_http_error_json(...)`

## 6. CLI Projection

CLI projection 结构：

- `code`
- `category`
- `summary`
- `detail`
- `visibility`
- `hints`

对应类型与常量：

- `ErrorCliResponse`
- `CLI_ERROR_RESPONSE_FIELDS`

默认规则：

- `summary` 使用 compact render
- `detail` 使用 verbose render

主调用入口：

- `StructError::cli_response(...)`
- `ErrorReport::cli_response(...)`
- `ErrorProtocolSnapshot::cli_response()`
- `to_cli_error_json(...)`

## 7. Log Projection

log projection 结构：

- `code`
- `category`
- `reason`
- `detail`
- `operation`
- `path`
- `visibility`
- `hints`
- `root_metadata`
- `context`
- `source_frames`

对应类型与常量：

- `ErrorLogResponse`
- `LOG_ERROR_RESPONSE_FIELDS`

默认规则：

- 保留完整 `context`
- 保留 `root_metadata`
- 保留 `source_frames`
- 适合日志、机器采集和诊断链路

主调用入口：

- `StructError::log_response(...)`
- `ErrorReport::log_response(...)`
- `ErrorProtocolSnapshot::log_response()`
- `to_log_error_json(...)`

## 8. RPC Projection

RPC projection 结构：

- `status`
- `code`
- `category`
- `reason`
- `detail`
- `visibility`
- `hints`
- `retryable`

对应类型与常量：

- `ErrorRpcResponse`
- `RPC_ERROR_RESPONSE_FIELDS`

默认规则：

- `detail` 只在 `Public` 可见时保留
- `retryable` 来自 `policy decision`
- 当前 `DefaultErrorPolicy` 只把 `sys.network_error` / `sys.timeout` 视为可重试

主调用入口：

- `StructError::rpc_response(...)`
- `ErrorReport::rpc_response(...)`
- `ErrorProtocolSnapshot::rpc_response()`
- `to_rpc_error_json(...)`

## 9. Test Helper

当前已落地的测试 helper：

- `assert_err_code(...)`
- `assert_err_category(...)`
- `assert_err_identity(...)`
- `assert_err_operation(...)`
- `assert_err_path(...)`

推荐断言顺序：

1. 先断言 `code`
2. 再断言 `category`
3. 再断言 `operation/path/meta`
4. 最后才断言渲染文本

不建议把 `detail` 全文本作为唯一断言目标。

## 10. 调用路径约定

当前推荐的消费路径：

1. 如果只需要稳定身份：`identity_snapshot()`
2. 如果需要统一协议输入：`policy_snapshot(...)`
3. 如果是面向具体出口：`http_response(...)` / `cli_response(...)` / `log_response(...)` / `rpc_response(...)`

如果上层要自定义策略，应优先实现自己的 `ErrorPolicy`，而不是复制 projection 逻辑。

## 11. 已知限制

当前仍有这些限制：

- `ErrorReport::policy_identity()` 仍是启发式兜底，不等于稳定身份主路径
- `report().to_*_json(...)` 在没有显式 identity 时，仍可能退化成 `report.unclassified`
- `retryable` 目前还是最小规则，不是完整重试策略系统
- 还没有单独的 protocol version 字段；当前依赖各结构字段集和测试锁定行为

因此：

- 新代码优先从 `StructError<_>` 直接走 `policy_report()` / `policy_snapshot(...)`
- 不要把 `ErrorReport` 的兜底 identity 当成稳定协议主路径
