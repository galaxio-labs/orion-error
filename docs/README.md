# 文档导航

当前文档以 `orion-error 0.6.x / V1 API` 为准。

建议阅读顺序：

1. [顶层 README](../README.md)
2. [V1 修复与评审基线](./v1-fix-and-review-plan.md)
3. [V1 结案说明](./v1-closure-summary.md)
4. [使用教程](./tutorial.md)
5. [日志说明](./LOGGING.md)
6. [与 thiserror 的配合](./thiserror-comparison.md)
7. [设计文档目录](./error-handling/README.md)

## 重要说明

旧版本文档中常见的过期写法包括：

- `orion-error = "0.2"` / `"0.3"` / `"0.4"`
- `impl DomainReason for MyError {}`
- `ctx.with("key", "value")`
- `UvsReason::validation_error("message")`
- `with_exit_log()`

当前版本对应写法：

- `orion-error = "0.6.1"`
- 一般不需要手写 `DomainReason`
- 使用 `ctx.record("key", "value")`
- 使用 `StructError::from(UvsReason::validation_error()).with_detail("message")`
- 使用 `with_auto_log()`
- `OperationContext::doing("op")` 是 V1 推荐主命名，但在 V1 中仍只是 `want(...)` 的命名糖衣
- 普通错误优先 `into_as(...)`；已是 `StructError<_>` 的跨层传播优先 `err_conv()` / `wrap_as(...)`

## V1 迁移主路径

V1 推荐的新调用主路径是：

- 普通错误第一次进入结构化体系：`into_as(...)`
- 已结构化错误向上层建立新边界：`wrap_as(...)`
- 普通 source：`with_std_source(...)`
- 结构化 source：`with_struct_source(...)`
- 上下文主命名：`at(...)` / `doing(...)`
  - 其中 `at(...)` 在 V1 中只是 `with(...)` 的命名糖衣
  - `doing(...)` 在 V1 中只是 `want(...)` 的命名糖衣

旧 API 仍然保留，但已经进入 deprecated path：

- `with_source(...)`
- `want(...)`
- `owe_*()` / `owe_*_source()`
- `err_wrap(...)`

如果其他文档与源码冲突，请以 `src/`、测试和顶层 README 为准。
