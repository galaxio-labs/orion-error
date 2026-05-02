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

稳定身份结构是 `snapshot::ErrorIdentity`。

字段：

- `code`
- `category`
- `reason`
- `detail`
- `position`
- `path`

语义：

- `code`：稳定机器主键
- `category`：稳定分类
- `reason`：稳定的人类摘要
- `detail`：可变补充说明，不是主键
- `path`：稳定导出的路径投影
- 当前 runtime 主语义仍应优先理解为 `action` / `locator` / path segments

入口：

- `StructError::identity_snapshot()`
- `assert_err_code(...)`
- `assert_err_category(...)`
- `assert_err_identity(...)`

注意：当前 `assert_err_code(...)` 断言的是 stable code 字符串，不是数值 `error_code()`。

## 3. Exposure

exposure 决策结构是 `protocol::ExposureDecision`。

字段：

- `http_status`
- `visibility`
- `default_hints`
- `retryable`

默认 exposure 策略实现是 `protocol::DefaultExposurePolicy`。

当前默认规则：

- `Biz -> 400 + Public`
- `Conf / Logic / Sys -> 500 + Internal`
- `sys.network_error` / `sys.timeout` -> `retryable = true`
- 其他 stable code 默认 `retryable = false`

说明：

- 当前文档中的运行时主路径仍然是 `doing(...)`
- top-level `want` 已从 identity / snapshot / protocol 投影中移除
- 兼容残留主要收在 context frame 的 `target`

主要入口：

- `ExposurePolicy::decide(...)`
- `StructError::exposure(...)`
- `StructError::into_exposure(...)`
- 完整 projection 数据以 `StructError::exposure(...)` 为主路径

## 4. `ErrorProtocolSnapshot`

`ErrorProtocolSnapshot` 是当前统一协议输入。

结构：

- `identity`
- `decision`
- 内嵌诊断 report，可通过 `report()` 只读访问

入口：

- `StructError::exposure(...)`
- `StructError::into_exposure(...)`

适用场景：

- 测试快照
- 网关二次投影
- 协议统一出口
- 用户调试摘要

## 5. HTTP Projection

JSON 字段：

- `status`
- `code`
- `category`
- `message`
- `visibility`
- `hints`

规则：

- `Public` 时，`message` 优先使用 `detail`
- `Internal` 时，`message` 使用稳定 `reason`

入口（需要 `serde_json` feature）：

- `ErrorProtocolSnapshot::to_http_error_json()`

## 6. CLI Projection

JSON 字段：

- `code`
- `category`
- `summary`
- `detail`
- `visibility`
- `hints`

规则：

- `summary` 使用 compact render
- `detail` 使用 verbose render

入口（需要 `serde_json` feature）：

- `ErrorProtocolSnapshot::to_cli_error_json()`

## 7. Log Projection

JSON 字段：

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

规则：

- 保留完整 `context`
- 保留 `root_metadata`
- 保留 `source_frames`

入口（需要 `serde_json` feature）：

- `ErrorProtocolSnapshot::to_log_error_json()`

## 8. RPC Projection

JSON 字段：

- `status`
- `code`
- `category`
- `reason`
- `detail`
- `visibility`
- `hints`
- `retryable`

规则：

- `detail` 只在 `Public` 可见时保留
- `retryable` 完全来自 exposure decision

入口（需要 `serde_json` feature）：

- `ErrorProtocolSnapshot::to_rpc_error_json()`

## 9. User Debug Summary

`render_user_debug(...)` 是给人看的调试摘要，不是机器协议。

入口：

- `ErrorProtocolSnapshot::render_user_debug()`
- `ErrorProtocolSnapshot::render_user_debug_redacted(...)`

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

- 协议投影应改走 `StructError::exposure(...)`
- 然后使用 `ErrorProtocolSnapshot::to_*_json()`

## 11. 建议的消费路径

推荐顺序：

1. 运行时传播用 `StructError<R>`
2. 要稳定识别时取 `identity_snapshot()`
3. 要统一出口规则时取 `exposure(...)`
4. 要协议出口时使用 projection API
5. 要人类摘要时使用 `render_user_debug(...)`

不建议：

- 直接把 `Display` 文本当协议主键
- 直接把 CLI 文本当机器协议
- 用 `detail` 全文本做唯一稳定断言
