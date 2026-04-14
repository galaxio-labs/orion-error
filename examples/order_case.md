# `order_case` 示例说明

这个示例现在对齐当前推荐写法，重点展示三条路径：

1. 普通错误转结构化错误时，优先保留真实 source。
2. 下层已经返回 `StructError<_>` 时，跨层传播优先使用 `err_conv()`。
3. `OperationContext::want(...)` 只表达最外层目标，链式 `.want(...)` 只补充内部路径。

## 示例里的推荐做法

- 入口上下文使用：

```rust
let mut ctx = OperationContext::want("place_order");
ctx.record("order", order_txt);
```

- 解析层已经返回 `StructError<ParseReason>`，服务层继续向上转成 `StructError<OrderReason>` 时，使用：

```rust
Self::parse_order(order_txt, amount)
    .want("解析订单")
    .with(&ctx)
    .err_conv()?;
```

- 用户查询和存储层也都走 `err_conv()`，而不是再回退到 `.owe_*()`。

- 存储层把底层 `io::Error` 转成结构化错误时，直接保留真实 source：

```rust
StructError::from(StoreReason::StorageFull)
    .with_detail("storage capacity exceeded")
    .with_source(e)
```

## 这个示例想表达什么

- `owe_*_source()` 适合普通 `Result<T, E>`，其中 `E` 是真实错误类型。
- `err_conv()` 适合 `Result<T, StructError<R1>> -> Result<T, StructError<R2>>`。
- `err_wrap(...)` 适合上层要主动建立一个新的 reason 边界时使用。

如果你只是想跑示例：

```bash
cargo run --example order_case
```
