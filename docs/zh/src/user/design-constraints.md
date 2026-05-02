# 设计约束

## 跨 StructError 的 From 转换：orphan rule 限制

### 问题

跨层错误转换（`StructError<ParseReason>` → `StructError<OrderReason>`）需要调用 `.conv_err()`。

```rust
// 期望但不能实现
fn place_order() -> Result<OrderDraft, StructError<OrderReason>> {
    let draft = parse_order()?;  // 期望自动 From<ParseError> → OrderError
    Ok(draft)
}

// 实际需要显式调用
fn place_order() -> Result<OrderDraft, StructError<OrderReason>> {
    let draft = parse_order().conv_err()?;  // 显式转换
    Ok(draft)
}
```

### 原因

Rust 的 orphan rule 不允许从下游 crate 中实现 `From<Foreign<Local>> for Foreign<Local2>`：

```rust
// 这行代码在用户 crate 中展开
impl From<orion_error::StructError<UserLocalReason>>   // Foreign<Local>
    for orion_error::StructError<UserLocalReason2>      // Foreign<Local2>
```

- `From` = 标准库 trait（外来）
- `StructError` = orion-error 的类型（外来）
- 即使 `LocalReason` 和 `LocalReason2` 是本地类型

Orphan rule 要求 trait 或 self type 至少有一个本地锚点，但这里 `From` 是外来 trait，`StructError<_>` 也是外来类型。
即使 `LocalReason` 和 `LocalReason2` 是本地类型，`From<StructError<Local>> for StructError<Local2>` 仍然不能在下游 crate 里实现。

### 已经尝试过的方案

| 方案 | 结果 |
|------|------|
| 下游 crate 直接 `impl From<StructError<A>> for StructError<B>` | ❌ orphan rule |
| derive 属性 `upcast_from(SubReason)` 在目标类型上 | ❌ orphan rule |
| derive 属性 `upcast_to(MainReason)` 在源类型上 | ❌ orphan rule |
| 让 `?` 自动触发跨 reason 转换 | ❌ 不能靠 `From` 达成 |
| newtype `struct AppError(StructError<T>)` | ✅ 可行，但所有 API 返回类型都需要改 |

### 结论

`.conv_err()` 是唯一的路径。
newtype 可以绕过 orphan rule，但代价太大——为了省一次显式调用而把所有函数返回类型包一层，收益远低于成本。
Rust 的 orphan rule 是生态兼容性的核心保证，短期内不会为此放宽。
