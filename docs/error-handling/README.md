# 错误处理设计说明

本目录保存 `orion-error` 的设计与分层说明。

阅读顺序建议：

1. `01-error-classification.md`
2. `02-handling-strategies.md`
3. `04-handling-layers.md`
4. `07-monitoring-metrics.md`
5. `08-recovery-patterns.md`

## 当前版本使用约定

示例请以当前源码为准：

- 使用 `OperationContext::record(...)`，不要继续新增 `ctx.with(...)`
- 使用 `StructError::from(UvsReason::validation_error()).with_detail(...)`
- 使用 `owe_*_source()` 保留真实 source chain
- `OperationContext::want(...)` 表示最外层目标；错误链上的 `.want(...)` 用于补全内部 `Path`
- `UvsReason::*_error()` 构造器不接收消息参数

## 一个最小示例

```rust
use orion_error::{
    ContextRecord, ErrorOweSource, ErrorWith, OperationContext, StructError, UvsReason,
};

fn place_order(order_txt: &str) -> Result<(), StructError<UvsReason>> {
    let mut ctx = OperationContext::want("place_order");
    ctx.record("order_text", order_txt);

    std::fs::read_to_string("order.txt")
        .owe_sys_source()
        .want("read order payload")
        .with(&ctx)
        .map(|_| ())
}
```

如果目录中的历史设计文档与当前实现冲突，请以 `src/`、测试和顶层 `README.md` 为准。
