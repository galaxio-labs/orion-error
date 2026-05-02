# Report / Exposure 边界

本文档描述 `DiagnosticReport` 与 `ErrorProtocolSnapshot` 之间的职责边界。

## 对象分工

| 对象 | 职责 |
|------|------|
| `StructError<R>` | 运行时传播、source 链、上下文挂载 |
| `DiagnosticReport` | 人类诊断视图、redaction、文本渲染 |
| `ErrorProtocolSnapshot` | identity + exposure decision + report、协议 JSON projection |

## 推荐主路径

**人类诊断：**
```rust
let report = err.report();
let text = report.render();
```

**协议/投影：**
```rust
let proto = err.exposure_snapshot(&policy);
proto.to_http_error_json()?;
proto.to_rpc_error_json()?;
```

## 原则

- `DiagnosticReport` 保持诊断对象定位
- `ErrorProtocolSnapshot` 是唯一的 exposure/projection 闭包对象
- 要文本诊断走 `report()`，要 JSON projection 走 `exposure_snapshot(...)`
