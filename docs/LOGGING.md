# 日志记录

`orion-error` 的日志能力围绕 `OperationContext` 展开。

## 启用方式

```toml
[dependencies]
orion-error = { version = "0.6", features = ["log"] }
# 或
orion-error = { version = "0.6", features = ["tracing"] }
```

默认 feature 已包含 `log`。

## 基本用法

```rust
use orion_error::{ContextRecord, OperationContext};

let mut ctx = OperationContext::want("order_processing");
ctx.record("order_id", "123");
ctx.record("amount", "100.0");

ctx.info("start");
ctx.debug("payload prepared");
ctx.warn("slow upstream");
ctx.error("final failure");
ctx.trace("verbose trace");
```

也可以使用别名方法：

- `log_info`
- `log_debug`
- `log_warn`
- `log_error`
- `log_trace`

## 自动结果日志

```rust
use orion_error::{ContextRecord, OperationContext};

let mut ctx = OperationContext::want("sync_user").with_auto_log();
ctx.record("user_id", "42");

do_sync()?;
ctx.mark_suc();
```

默认结果是失败。如果启用了 `with_auto_log()` 但没有标记成功，Drop 时会输出失败日志。

## 使用 `OperationScope`

```rust
use orion_error::{ContextRecord, OperationContext};

let mut ctx = OperationContext::want("sync_user").with_auto_log();

{
    let mut scope = ctx.scoped_success();
    scope.record("user_id", "42");
    validate()?;
}
```

可选方法：

- `scope()`：默认失败，需要显式 `mark_success()`
- `scoped_success()`：作用域结束时自动标记成功
- `cancel()`：标记取消

## `op_context!` 宏

```rust
use orion_error::{op_context, ContextRecord};

let mut ctx = op_context!("load_config").with_auto_log();
ctx.record("path", "config.toml");
```

这个宏会在调用处展开 `module_path!()`，方便日志系统显示正确模块路径。

## log 与 tracing

- 只启用 `log` 时，使用 `log` 宏输出
- 启用 `tracing` 时，优先使用 `tracing`
- 同时启用时，代码路径以 `tracing` 为准

## 推荐实践

- 用 `want(...)` 描述操作目标
- 用 `record(...)` 记录关键诊断字段
- 用 `with_auto_log()` 只包裹真正需要结果日志的作用域
- 对成功路径使用 `mark_suc()` 或 `scoped_success()`
- 不要继续新增 `with_exit_log()`；它已废弃
