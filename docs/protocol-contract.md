# 协议契约

更新时间：2026-04-23

本文档描述 `orion-error` 当前已经落地的协议层设计。

这里说的“协议层”不是新的 runtime 传播模型，而是对外稳定消费接口。

## 1. 三层结构

当前协议层由三层组成：

1. 稳定身份：`ErrorIdentity`
2. exposure 决策：`ExposureDecision`
3. 出口投影：HTTP / CLI / log / RPC / user debug

推荐把它理解为：

- `StructError<R>` 负责运行时传播
- `ErrorIdentity` 负责稳定识别
- `DiagnosticReport` 负责人类诊断
- `ErrorProtocolSnapshot` 负责把 identity + decision + report 组装成统一消费输入

## 2. 稳定身份

稳定身份结构是 [`ErrorIdentity`](/Users/zuowenjian/devspace/wp-labs/dev/crates/orion-error/src/core/snapshot.rs:134)。

字段：

- `code`
- `category`
- `reason`
- `detail`
- `position`
- `want`
- `path`

语义：

- `code`：稳定机器主键
- `category`：稳定分类
- `reason`：稳定的人类摘要
- `detail`：可变补充说明，不是主键
- `want` / `path`：兼容投影字段；当前主语义应优先理解为 `action` / `locator`

入口：

- `StructError::identity_snapshot()`
- `assert_err_code(...)`
- `assert_err_category(...)`
- `assert_err_identity(...)`

注意：当前 `assert_err_code(...)` 断言的是 stable code 字符串，不是数值 `error_code()`。

## 3. Exposure

exposure 决策结构是 [`ExposureDecision`](/Users/zuowenjian/devspace/wp-labs/dev/crates/orion-error/src/core/report.rs:82)。

字段：

- `http_status`
- `visibility`
- `default_hints`
- `retryable`

默认 exposure 策略实现是 `DefaultExposurePolicy`。

当前默认规则：

- `Biz -> 400 + Public`
- `Conf / Logic / Sys -> 500 + Internal`
- `sys.network_error` / `sys.timeout` -> `retryable = true`
- 其他 stable code 默认 `retryable = false`

说明：

- 当前文档中的运行时主路径仍然是 `doing(...)`
- `want` 继续存在于 identity / report / snapshot 投影里，主要用于兼容旧消费方
- 不应在新文档或新示例里把 `want(...)` 当成主路径 API 介绍

主要入口：

- `ExposurePolicy::decide(...)`
- `ExposureView::decision(...)`
- `StructError::exposure_view()`
- `StructError::exposure_snapshot(...)`
- `DiagnosticReport::exposure_snapshot(...)`

## 4. `ErrorProtocolSnapshot`

`ErrorProtocolSnapshot` 是当前统一协议输入。

结构：

- `identity`
- `decision`
- `report`

入口：

- `StructError::exposure_snapshot(...)`
- `StructError::into_exposure_snapshot(...)`
- `DiagnosticReport::exposure_snapshot(...)`

适用场景：

- 测试快照
- 网关二次投影
- 协议统一出口
- 用户调试摘要

## 5. HTTP Projection

类型：

- `ErrorHttpResponse`

字段：

- `status`
- `code`
- `category`
- `message`
- `visibility`
- `hints`

默认规则：

- `Public` 时，`message` 优先使用 `detail`
- `Internal` 时，`message` 使用稳定 `reason`

入口：

- `StructError::http_response(...)`
- `DiagnosticReport::http_response(...)`
- `ErrorProtocolSnapshot::http_response()`

如果启用了 `serde_json` feature，还可以使用：

- `DiagnosticReport::to_http_error_json(...)`
- `ErrorProtocolSnapshot::to_http_error_json()`

## 6. CLI Projection

类型：

- `ErrorCliResponse`

字段：

- `code`
- `category`
- `summary`
- `detail`
- `visibility`
- `hints`

默认规则：

- `summary` 使用 compact render
- `detail` 使用 verbose render

入口：

- `StructError::cli_response(...)`
- `DiagnosticReport::cli_response(...)`
- `ErrorProtocolSnapshot::cli_response()`

`serde_json` feature 开启后可用：

- `DiagnosticReport::to_cli_error_json(...)`
- `ErrorProtocolSnapshot::to_cli_error_json()`

## 7. Log Projection

类型：

- `ErrorLogResponse`

字段：

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

默认规则：

- 保留完整 `context`
- 保留 `root_metadata`
- 保留 `source_frames`

入口：

- `StructError::log_response(...)`
- `DiagnosticReport::log_response(...)`
- `ErrorProtocolSnapshot::log_response()`

`serde_json` feature 开启后可用：

- `DiagnosticReport::to_log_error_json(...)`
- `ErrorProtocolSnapshot::to_log_error_json()`

## 8. RPC Projection

类型：

- `ErrorRpcResponse`

字段：

- `status`
- `code`
- `category`
- `reason`
- `detail`
- `visibility`
- `hints`
- `retryable`

默认规则：

- `detail` 只在 `Public` 可见时保留
- `retryable` 完全来自 exposure decision

入口：

- `StructError::rpc_response(...)`
- `DiagnosticReport::rpc_response(...)`
- `ErrorProtocolSnapshot::rpc_response()`

`serde_json` feature 开启后可用：

- `DiagnosticReport::to_rpc_error_json(...)`
- `ErrorProtocolSnapshot::to_rpc_error_json()`

## 9. User Debug Summary

`render_user_debug(...)` 是给人看的调试摘要，不是机器协议。

入口：

- `StructError::render_user_debug(...)`
- `StructError::render_user_debug_redacted(...)`
- `ExposureView::render_user_debug(...)`
- `ErrorProtocolSnapshot::render_user_debug()`

它的定位是：

- 本地调试
- 示例输出
- 人工排障

它不是：

- public HTTP message
- 稳定 JSON schema

## 10. `DiagnosticReport`

`DiagnosticReport` 是 report 层对象。

它不依赖 `ErrorIdentityProvider`，因此更适合：

- 文本渲染
- redaction
- 人类诊断

常用入口：

- `StructError::report()`
- `StructError::into_report()`
- `ErrorSnapshot::report()`
- `StableErrorSnapshot::report()`

如果启用了 `serde_json` feature，还可以使用：

- `DiagnosticReport::to_exposure_snapshot_json(...)`
- `DiagnosticReport::to_exposure_snapshot_json(...)`

## 11. 建议的消费路径

推荐顺序：

1. 运行时传播用 `StructError<R>`
2. 要稳定识别时取 `identity_snapshot()`
3. 要统一出口规则时取 `exposure_view()` 或 `exposure_snapshot(...)`
4. 要协议出口时使用 projection API
5. 要人类摘要时使用 `render_user_debug(...)`

不建议：

- 直接把 `Display` 文本当协议主键
- 直接把 CLI 文本当机器协议
- 用 `detail` 全文本做唯一稳定断言
