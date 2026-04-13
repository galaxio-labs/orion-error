# 使用教程

本教程面向 `orion-error 0.6.x`，以当前源码与测试为准。

## 安装

```toml
[dependencies]
orion-error = "0.6"
```

可选特性：

```toml
[dependencies]
orion-error = { version = "0.6", features = ["serde"] }
# 或
orion-error = { version = "0.6", features = ["tracing"] }
```

默认启用 `log`。

## 一分钟上手

```rust
use derive_more::From;
use orion_error::{
    ContextRecord, ErrorCode, ErrorOweSource, ErrorWith, OperationContext, StructError, UvsReason,
};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, From)]
enum UserError {
    #[error("user not found")]
    UserNotFound,
    #[error("{0}")]
    Uvs(UvsReason),
}

impl ErrorCode for UserError {
    fn error_code(&self) -> i32 {
        match self {
            Self::UserNotFound => 1001,
            Self::Uvs(reason) => reason.error_code(),
        }
    }
}

fn load_user(user_id: u64) -> Result<String, StructError<UserError>> {
    let mut ctx = OperationContext::want("load_user");
    ctx.record("user_id", user_id.to_string());

    std::fs::read_to_string("user.json")
        .owe_sys_source()
        .want("read user profile")
        .with(&ctx)
}
```

说明：

- 领域错误一般不必手写 `impl DomainReason`；满足 `From<UvsReason> + Display + PartialEq` 即自动实现。
- `record(...)` 是当前推荐的上下文写法。
- `owe_sys_source()` 会保留底层 `io::Error`。

## 1. 定义领域错误

推荐模式：

```rust
use derive_more::From;
use orion_error::{ErrorCode, UvsReason};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, From)]
enum OrderError {
    #[error("insufficient funds")]
    InsufficientFunds,
    #[error("order not found")]
    OrderNotFound,
    #[error("{0}")]
    Uvs(UvsReason),
}

impl ErrorCode for OrderError {
    fn error_code(&self) -> i32 {
        match self {
            Self::InsufficientFunds => 2001,
            Self::OrderNotFound => 2002,
            Self::Uvs(reason) => reason.error_code(),
        }
    }
}
```

## 2. 创建结构化错误

### 直接构造

```rust
use orion_error::{StructError, UvsReason};

let err = StructError::from(UvsReason::validation_error())
    .with_detail("field `email` is required");
```

### Builder 构造

```rust
use orion_error::{OperationContext, StructError, UvsReason};

let ctx = OperationContext::want("validate_request");

let err = StructError::builder(UvsReason::validation_error())
    .detail("field `email` is required")
    .context_ref(&ctx)
    .finish();
```

### 保留真实 source

```rust
use orion_error::{StructError, UvsReason};

let err = StructError::builder(UvsReason::system_error())
    .detail("failed to read config")
    .source(std::io::Error::other("disk offline"))
    .finish();
```

## 3. 使用上下文

```rust
use orion_error::{ContextRecord, ErrorWith, OperationContext};

let mut ctx = OperationContext::want("place_order");
ctx.record("order_id", "A-1001");
ctx.record("user_id", "42");

let result = check_inventory()
    .want("check inventory")
    .with(&ctx);
```

推荐约定：

- `OperationContext::want(...)` 写本次调用的最外层目标
- 错误链上的 `.want(...)` 只追加内部步骤，形成完整 `Path`
- `record(...)` 写关键诊断键值
- `detail(...)` / `with_detail(...)` 写补充调试说明

上面的例子里，最终错误会体现为：

- `Want`: `place_order`
- `Path`: `place_order / check inventory`

如果你要读取这个语义：

- `target_main()` 返回最外层目标
- `target_path()` 返回完整调用路径
- `target()` 保留为 `target_main()` 的兼容别名

## 4. 错误转换策略

### 默认推荐：保留真实 source

当上游错误实现 `std::error::Error` 时，优先使用 `owe_*_source()`：

```rust
std::fs::read_to_string("config.toml").owe_sys_source()?;
call_http_service().owe_net_source()?;
```

这是当前推荐路径。

### 兼容路径：只保留 detail

当上游值只实现 `Display`，或者它本身不是一个真正的 error type 时，再使用 `owe_*()`：

```rust
parse_input().owe_validation()?;
run_business_rule().owe_biz()?;
```

### 自定义 reason

```rust
some_result.owe(UvsReason::permission_error())?;
some_io_result.owe_source(UvsReason::system_error())?;
```

### `StructError<R1>` 到 `StructError<R2>`

```rust
repo_call().err_conv()?;
```

`err_conv()` 会保留：

- `detail`
- `position`
- context stack
- source

### 跨层包装

如果你不是做 reason 类型转换，而是要在上层重新定义一个新 reason，同时把下层 `StructError` 整个作为 source 保留下来：

```rust
use orion_error::{ErrorWrap, StructError, UvsReason};

fn service_call() -> Result<(), StructError<UvsReason>> {
    repo_call()
        .err_wrap(UvsReason::system_error())
        .map(|_| ())
}
```

这种方式更适合 service/repository/infrastructure 分层包装。

## 5. `UvsReason` 选择建议

- `validation_error()`：输入、格式、约束校验失败
- `business_error()`：业务规则冲突
- `not_found_error()`：资源不存在
- `permission_error()`：认证或授权失败
- `logic_error()`：内部逻辑错误或不变量被破坏
- `data_error()`：数据处理或序列化问题
- `system_error()`：文件系统、OS、锁、进程环境问题
- `network_error()`：网络连接、DNS、HTTP 传输失败
- `resource_error()`：资源耗尽
- `timeout_error()`：超时
- `core_conf()` / `feature_conf()` / `dynamic_conf()`：配置错误
- `external_error()`：第三方系统失败

## 6. 日志与作用域

```rust
use orion_error::{op_context, ContextRecord};

fn process_order(order_id: &str) -> Result<(), MyError> {
    let mut ctx = op_context!("process_order").with_auto_log();
    ctx.record("order_id", order_id);
    ctx.info("starting");

    {
        let mut scope = ctx.scoped_success();
        scope.record("phase", "validation");
        validate(order_id)?;
    }

    Ok(())
}
```

注意：

- `OperationContext` 默认结果是失败
- 成功路径要么显式 `mark_suc()`，要么使用 `scoped_success()`
- `with_exit_log()` 已废弃，改用 `with_auto_log()`

## 7. 与 `thiserror` 的配合

推荐：

1. 用 `thiserror` 定义领域错误枚举
2. 用 `derive_more::From` 接入 `UvsReason`
3. 实现 `ErrorCode`
4. 在业务里优先使用 `ErrorOweSource`，必要时再用 `ErrorOwe` / `ErrorConv`

详见 [thiserror-comparison.md](./thiserror-comparison.md)。

## 8. source-chain 使用建议

如果你需要：

- `source()`
- `root_cause()`
- 更真实的底层错误链
- 监控里区分包装错误和根因

优先使用：

- `with_source(...)`
- `builder.source(...)`
- `owe_source(...)`
- `owe_*_source()`
- `wrap(...)` / `err_wrap(...)`

常用链路查看方法：

- `source_ref()`
- `root_cause()`
- `root_cause_frame()`
- `source_chain()`
- `source_frames()`
- `display_chain()`

启用 `serde` 后，序列化输出会包含 `want`、`path`、`source_frames`、`source_message` 和 `source_chain`。

其中 `source_frames` 是结构化链路，每一帧至少包含：

- `index`
- `message`
- 可选 `display`
- 可选 `type_name`
- 可选 `error_code`
- 可选 `reason`
- 可选 `want`
- 可选 `path`
- 可选 `detail`
- `is_root_cause`

当 source frame 来自下层 `StructError` 时，`message` 是稳定的 reason 文本，`display` 才是完整格式化错误。`debug` 在运行时仍可通过 `SourceFrame` 读取，但默认不会进入 serde 输出，因为 `Debug` 文本可能包含内部字段或敏感信息。

治理侧建议优先消费 `source_frames`；`source_chain` / `source_message` 主要作为兼容摘要保留。`type_name` 是 best-effort 信息，不应作为完整且稳定的分类键。

底层 trait object 本体仍然不会直接序列化。

旧的 `owe_*()` 仍可用，但只会把字符串放进 `detail`，因此不应作为普通 Rust error 的默认写法。

## 9. 当前版本的兼容提示

以下旧写法不要再新增：

- `OperationContext::with(...)`
- `OperationContext::with_path(...)`
- `with_exit_log()`
- `impl DomainReason for MyError {}` 这种空实现
- `UvsReason::validation_error("msg")` 这种带参数构造

当前正确写法：

```rust
let mut ctx = OperationContext::want("op");
ctx.record("key", "value");

let err = StructError::from(UvsReason::validation_error())
    .with_detail("message");
```

## 10. 验证命令

在 `orion-error/` 目录执行：

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features -- --test-threads=1
cargo run --example order_case
cargo run --example logging_example --features log
```
