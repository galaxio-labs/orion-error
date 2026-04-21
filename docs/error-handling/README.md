# 错误处理设计说明

本目录保存 `orion-error` 的设计与分层说明。

重要边界：

- 本目录下的 `01-08` 文档，主要作为历史设计参考和治理思路记录保留
- 它们不是 `orion-error 0.6.x / V1 API` 的一线使用手册
- 如果设计文档中的示意代码、旧命名或内部结构与当前实现冲突，应优先以 `README.md`、`docs/tutorial.md`、`docs/v1-migration-checklist.md`、`docs/v1-fix-and-review-plan.md` 和 `src/` / 测试为准

阅读顺序建议：

1. `01-error-classification.md`
2. `02-handling-strategies.md`
3. `04-handling-layers.md`
4. `07-monitoring-metrics.md`
5. `08-recovery-patterns.md`

## 当前版本使用约定

示例请以当前源码为准：

- `orion_error::prelude::*` 是 V1 主路径通配导入
- 如果只想导入 V1 主路径扩展 trait，可使用 `orion_error::traits_ext::*`
- `orion_error::compat_prelude::*` / `orion_error::compat_traits::*` 只用于维护旧的 `owe_*()` / `err_wrap(...)` 路径
- 使用 `OperationContext::record(...)`，不要继续新增 `ctx.with(...)`
- 使用 `StructError::from(UvsReason::validation_error()).with_detail(...)`
- 普通错误第一次进入结构化体系，优先 `into_as(...)`
- 对 `StructError<_>` 的跨层传播，优先使用 `err_conv()` 或 `wrap_as(...)`
- 普通 source 使用 `with_std_source(...)`，结构化 source 使用 `with_struct_source(...)`
- `OperationContext::doing(...)` 是 V1 推荐主命名；错误链上的 `.doing(...)` 用于补全内部 `Path`
- V1 中 `doing(...)` 只是 `want(...)` 的命名糖衣，`at(...)` 只是 `with(...)` 的命名糖衣
- `owe_*()` / `owe_*_source()`、`want(...)`、`err_wrap(...)` 仍保留，但属于兼容路径
- `UvsReason::*_error()` 构造器不接收消息参数

新代码默认不要把 `prelude::*` 和 compat 导入混成一个默认接口。

## 一个最小示例

```rust
use orion_error::{
    ContextRecord, ErrorWith, IntoAs, OperationContext, StructError, UvsReason,
};

fn place_order(order_txt: &str) -> Result<(), StructError<UvsReason>> {
    let mut ctx = OperationContext::doing("place_order");
    ctx.record("order_text", order_txt);

    std::fs::read_to_string("order.txt")
        .into_as(UvsReason::system_error(), "read order payload failed")
        .doing("read order payload")
        .with(&ctx)
        .map(|_| ())
}
```

如果目录中的历史设计文档与当前实现冲突，请以 `src/`、测试、`docs/README.md` 和顶层 `README.md` 中的 V1 主路径说明为准。
