# 文档导航

当前文档以 `orion-error 0.6.x` 为准。

建议阅读顺序：

1. [顶层 README](../README.md)
2. [使用教程](./tutorial.md)
3. [日志说明](./LOGGING.md)
4. [与 thiserror 的配合](./thiserror-comparison.md)
5. [设计文档目录](./error-handling/README.md)

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
- `OperationContext::want("op")` 表示最外层目标；链式 `.want("step")` 表示内部路径
- 普通错误优先 `owe_*_source()`；已是 `StructError<_>` 的跨层传播优先 `err_conv()` / `err_wrap(...)`

如果其他文档与源码冲突，请以 `src/`、测试和顶层 README 为准。
