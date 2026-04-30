# Report Exposure Boundary

更新时间：2026-04-28

本文档记录 `DiagnosticReport` 与 `ErrorProtocolSnapshot` 之间的职责边界。

## 当前状态

`category` 和 `code` 已从 `DiagnosticReport` 中移除，identity 数据只存在于
`ErrorProtocolSnapshot.identity`。`DiagnosticReport` 上的 exposure bridge 方法
（`exposure_identity`、`http_status`、`visibility`、`default_hints`、
`decision`、`exposure_snapshot`、`to_exposure_snapshot_json`）已全部删除。

替代入口为 `ErrorProtocolSnapshot::from_report_skeleton(report, identity, policy)`。
注意：该入口只会基于 `report + identity` 构造协议骨架；
如果调用方还需要完整的 root metadata / source frames / path projection，
应优先从 `StructError::exposure_snapshot(...)` 进入。

`StructError<T>::report()` 只要求 `DomainReason`，不再需要 `ErrorIdentityProvider`。

## 1. 当前对象分工

当前主路径涉及三个对象：

1. `StructError<R>`
2. `DiagnosticReport`
3. `ErrorProtocolSnapshot`

当前职责大致是：

- `StructError<R>`
  - 运行时传播
  - source 链持有
  - 上下文挂载
- `DiagnosticReport`
  - 人类诊断视图
  - redaction
  - report 文本 render
- `ErrorProtocolSnapshot`
  - identity + exposure decision + report
  - user debug 摘要
  - 协议 JSON projection

这条主线总体已经成立。

## 2. 当前边界问题

虽然协议 JSON projection 已经主要集中到 `ErrorProtocolSnapshot`，但
`DiagnosticReport` 当前仍然保留了一整组 exposure bridge 方法：

- `exposure_identity()`
- `http_status(...)`
- `visibility(...)`
- `default_hints(...)`
- `decision(...)`
- `exposure_snapshot(...)`
- `to_exposure_snapshot_json(...)`

这会带来两个问题：

1. `DiagnosticReport` 不再只是“诊断对象”，而是同时承担：
   - 文本 render
   - redaction
   - exposure 决策桥接
2. public API 会形成两条并行路径：
   - `StructError -> exposure_snapshot(...)`
   - `DiagnosticReport -> exposure_snapshot(...)`

这两条路径都能工作，但第二条路径会继续把 protocol/exposure 逻辑压回 report 层，导致 `DiagnosticReport` 持续变胖。

## 3. 当前建议主路径

推荐只把下面两条路径当成正式主路径：

### 3.1 人类诊断路径

```rust,ignore
let report = err.report();
let text = report.render();
```

这条路径只关注：

- 诊断字段
- redaction
- 文本 render

### 3.2 协议/投影路径

```rust,ignore
let proto = err.exposure_snapshot(&policy);
let debug = proto.render_user_debug();
let http = proto.to_http_error_json()?;
```

这条路径只关注：

- stable identity
- exposure decision
- user debug
- HTTP / CLI / log / RPC projection

## 4. 收口原则

核心原则是：

- `DiagnosticReport` 保持“诊断对象”定位
- `ErrorProtocolSnapshot` 成为唯一的 exposure / projection 闭包对象
- `StructError` 作为主入口负责把运行时错误推进到 report 或 protocol 层

换句话说：

- 要文本诊断：走 `report()`
- 要 exposure / JSON projection：走 `exposure_snapshot(...)`

## 5. 建议 API 收口方案

### 5.1 保留的 `DiagnosticReport` 能力

建议长期保留：

- `render()`
- `redacted(...)`
- `render_redacted(...)`

这些方法和 report 层职责天然一致。

### 5.2 新增 canonical 入口

如果仍需要“从 `DiagnosticReport` 继续进入 protocol 层”的能力，建议把入口收成一个显式构造函数：

```rust,ignore
impl ErrorProtocolSnapshot {
    pub fn from_report_skeleton(
        report: DiagnosticReport,
        identity: ErrorIdentity,
        policy: &impl ExposurePolicy,
    ) -> Self;
}
```

这样：

- protocol 入口集中到 `ErrorProtocolSnapshot`
- `DiagnosticReport` 不需要继续挂越来越多的 exposure 方法
- `DiagnosticReport` 本身也不需要继续持有 protocol projection 数据

### 5.3 降级 `DiagnosticReport` 上的 exposure bridge

建议逐步降级下面这组方法的 public 地位：

- `exposure_identity()`
- `http_status(...)`
- `visibility(...)`
- `default_hints(...)`
- `decision(...)`
- `exposure_snapshot(...)`
- `to_exposure_snapshot_json(...)`

可选做法：

1. 先保留实现，但在文档里降级为 secondary path
2. 后续改成内部转发到 `ErrorProtocolSnapshot::from_report_skeleton(...)`
3. 再决定是否在下一个 breaking window 正式移除

## 6. 建议迁移后的调用形态

推荐调用形态应收敛为：

```rust,ignore
let report = err.report();
println!("{}", report.render());

let proto = err.exposure_snapshot(&policy);
println!("{}", proto.render_user_debug());
let http = proto.to_http_error_json()?;
```

如果调用方起点不是 `StructError`，而是一个已有的 `DiagnosticReport`，则显式写：

```rust,ignore
let proto = ErrorProtocolSnapshot::from_report_skeleton(report, identity, &policy);
```

这样可以避免把“诊断对象”和“协议对象”继续混成一层。
但如果调用方关心完整 projection 数据，仍应直接使用
`StructError::exposure_snapshot(...)`。

## 7. 结论

当前设计的问题不在于 `DiagnosticReport` 有 `render()`，而在于它仍然保留了一整组 exposure bridge 方法。

因此建议的收口方向不是削弱 report 文本能力，而是：

- 维持 `DiagnosticReport` 的诊断定位
- 让 `ErrorProtocolSnapshot` 成为唯一的 exposure / projection 闭包对象
- 把 `DiagnosticReport -> exposure` 这组能力降为次路径，最终收成单一入口
