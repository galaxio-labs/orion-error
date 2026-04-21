# `order_case` 示例说明

这个示例现在对齐当前推荐写法，重点展示三条路径：

1. 普通错误第一次进入结构化体系时，优先使用 `into_as(...)`。
2. 下层已经返回 `StructError<_>` 时，跨层传播优先使用 `err_conv()`。
3. `OperationContext::doing(...)` 是 V1 推荐主命名，链式 `.doing(...)` 只补充内部路径。
4. 诊断分类需要的稳定字段，优先写进 typed metadata，而不是塞进 `detail` 或依赖 `Display` 文本匹配。
5. 默认 `Display` 保持简洁；详细诊断输出应通过 `report()` / `render(RenderMode::Verbose)` / `render_redacted(...)` 显式获取。

## 示例里的推荐做法

- 入口上下文使用：

```rust
let mut ctx = OperationContext::doing("place_order");
ctx.record("order", order_txt);
ctx.record_meta("component.name", "order_service");
```

- 解析层已经返回 `StructError<ParseReason>`，服务层继续向上转成 `StructError<OrderReason>` 时，使用：

```rust
Self::parse_order(order_txt, amount)
    .doing("解析订单")
    .with(&ctx)
    .err_conv()?;
```

- 用户查询和存储层也都走 `err_conv()`，而不是再回退到 `.owe_*()`。

- 存储层把底层 `io::Error` 转成结构化错误时，直接保留真实 source：

```rust
StructError::from(StoreReason::StorageFull)
    .with_detail("storage capacity exceeded")
    .with_std_source(e)
```

- 解析层把机器分类信息写进 metadata，而不是写进 `detail`：

```rust
StructError::builder(ParseReason::FormatError)
    .detail("订单文本不能为空")
    .context(
        OperationContext::doing("parse order text")
            .with_meta("config.kind", "order_txt")
            .with_meta("parse.field", "order_txt")
    )
    .finish()
```

- 上层可以分别读取 root metadata 和 source frame metadata：

```rust
let root_meta = err.context_metadata();
let source_meta = &err.source_frames()[0].metadata;
```

- 需要结构化诊断快照时，可以先取 `report()`：

```rust
let report = err.report();
println!("report path: {:?}", report.path);
```

- 需要详细文本输出时，显式使用 verbose render：

```rust
println!("{}", err.render(RenderMode::Verbose));
```

- 需要写日志或外发时，先走 redaction：

```rust
struct ExampleRedactPolicy;

impl RedactPolicy for ExampleRedactPolicy {
    fn redact_key(&self, key: &str) -> bool {
        matches!(key, "order" | "config.secret")
    }

    fn redact_value(&self, _key: Option<&str>, _value: &str) -> Option<String> {
        Some("<redacted>".to_string())
    }
}

println!(
    "{}",
    err.render_redacted(RenderMode::Verbose, &ExampleRedactPolicy)
);
```

## 这个示例想表达什么

- `into_as(...)` 适合普通 `Result<T, E>` 第一次进入结构化体系。
- `err_conv()` 适合 `Result<T, StructError<R1>> -> Result<T, StructError<R2>>`。
- `wrap_as(...)` 适合上层要主动建立一个新的 reason 边界时使用。
- `with_meta()` / `record_meta()` 适合附加稳定、短小、机器可读的诊断字段。
- `report()` / `render(...)` 适合显式诊断输出；默认 `Display` 不应该被扩展成 verbose 调试出口。
- `render_redacted(...)` 适合在日志、审计、外发 JSON 前统一执行敏感信息清洗。

如果你只是想跑示例：

```bash
cargo run --example order_case
```
