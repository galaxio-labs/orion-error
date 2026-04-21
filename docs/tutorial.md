# 使用教程

本教程面向 `orion-error 0.6.x` 的 `V1 API`，以当前源码与测试为准。

在调整本教程中的主路径、边界约束和评审标准前，先以 [V1 修复与评审基线](./v1-fix-and-review-plan.md) 为准。

## 安装

```toml
[dependencies]
orion-error = "0.6.1"
```

可选特性：

```toml
[dependencies]
orion-error = { version = "0.6.1", features = ["serde"] }
# 或
orion-error = { version = "0.6.1", features = ["tracing"] }
```

默认启用 `log`。

## 导入约定

- `orion_error::prelude::*`：V1 主路径通配导入，包含 `IntoAs`、`ErrorWrapAs`、`ErrorConv`、`ErrorWith` 等推荐 API
- `orion_error::traits_ext::*`：如果你只想按 trait 分组导入 V1 主路径扩展 trait，可以用这一层
- `orion_error::compat_prelude::*` / `orion_error::compat_traits::*`：只用于兼容旧的 `owe_*()` / `err_wrap(...)` 调用路径

如果是新代码，默认不要把 compat 导入和 `prelude::*` 混成一个体系。

## 一分钟上手

```rust
use derive_more::From;
use orion_error::{
    ContextRecord, ErrorCode, ErrorWith, IntoAs, OperationContext, StructError, UvsReason,
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
    let mut ctx = OperationContext::doing("load_user");
    ctx.record("user_id", user_id.to_string());

    std::fs::read_to_string("user.json")
        .into_as(UserError::from(UvsReason::system_error()), "read user profile failed")
        .doing("read user profile")
        .with(&ctx)
}
```

说明：

- 领域错误一般不必手写 `impl DomainReason`；满足 `From<UvsReason> + Display + PartialEq` 即自动实现。
- `record(...)` 是当前推荐的上下文写法。
- `into_as(...)` 是普通错误进入结构化体系的主路径。
- `doing(...)` 在 V1 中只是 `want(...)` 的命名糖衣，不改变 `OperationContext` 底层语义。
- 如果上游已经是 `StructError<_>`，优先用 `err_conv()` 或 `wrap_as(...)`，不要再回退到 `.owe_*()`。

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

let ctx = OperationContext::doing("validate_request");

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
    .source_std(std::io::Error::other("disk offline"))
    .finish();
```

## 3. 使用上下文

```rust
use orion_error::{ContextRecord, ErrorWith, OperationContext};

let mut ctx = OperationContext::doing("place_order");
ctx.record("order_id", "A-1001");
ctx.record("user_id", "42");

let result = check_inventory()
    .doing("check inventory")
    .with(&ctx);
```

推荐约定：

- `OperationContext::doing(...)` 是 V1 推荐主命名，但在 V1 中仍只是 `want(...)` 的命名糖衣
- 错误链上的 `.doing(...)` 只追加内部步骤，形成完整 `Path`
- `at(...)` 在 V1 中只是 `with(...)` 的命名糖衣，不承诺写入结构化 target/path
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

### 默认推荐：普通错误进入结构化体系

当上游错误实现 `std::error::Error`，并且这是第一次进入结构化错误体系时，优先使用 `into_as(...)`：

```rust
use orion_error::IntoAs;

std::fs::read_to_string("config.toml")
    .into_as(UvsReason::system_error(), "read config failed")?;

call_http_service()
    .into_as(UvsReason::network_error(), "call http service failed")?;
```

这是 V1 推荐主路径。

### 兼容路径：只保留 detail

当上游值只实现 `Display`，或者它本身不是一个真正的 error type 时，再使用 `owe_*()`：

```rust
parse_input().owe_validation()?;
run_business_rule().owe_biz()?;
```

### 自定义 reason

```rust
use orion_error::IntoAs;

some_io_result.into_as(UvsReason::system_error(), "load file failed")?;
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
use orion_error::{ErrorWrapAs, StructError, UvsReason};

fn service_call() -> Result<(), StructError<UvsReason>> {
    repo_call()
        .wrap_as(UvsReason::system_error(), "service call failed")
        .map(|_| ())
}
```

这种方式更适合作为 V1 的公开主路径，用于 service/repository/infrastructure 分层包装。

兼容说明：

- `err_wrap(...)` 仍然保留
- 但它属于兼容入口，不属于 V1 推荐主路径

### 推荐决策顺序

- 上游是普通 `Error` 类型：优先 `into_as(...)`
- 需要显式声明“这是显式实现了 `RawStdError` 的 raw StdError 类型”时：`raw_source(...)` 后再 `into_as(...)`
- 上游只实现 `Display`：仍走兼容路径 `owe_*()`
- 上游已经是 `StructError<_>` 且只做 reason 映射：优先 `err_conv()`
- 上游已经是 `StructError<_>` 且要新建上层语义边界：优先 `wrap_as(...)`

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
4. 在业务里优先使用 `IntoAs` / `wrap_as(...)` / `err_conv()`

导入建议：

- 新代码优先 `use orion_error::prelude::*;`
- 如果必须维护旧的 `owe_*()` / `err_wrap(...)`，再显式补 `use orion_error::compat_prelude::*;`

详见 [thiserror-comparison.md](./thiserror-comparison.md)。

## 8. source-chain 使用建议

如果你需要：

- `source()`
- `root_cause()`
- 更真实的底层错误链
- 监控里区分包装错误和根因

优先使用：

- `with_std_source(...)`
- `builder.source_std(...)`
- `into_as(...)`
- `with_struct_source(...)`
- `wrap_as(...)`

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

旧的 `owe_*()` / `owe_*_source()` / `want(...)` / `with_source(...)` 仍可用，但都已进入 deprecated path；V1 不建议新增使用。

## 9. 当前版本的兼容提示

以下旧写法不要再新增：

- `OperationContext::with(...)`
- `OperationContext::with_path(...)`
- `with_exit_log()`
- `with_source(...)`
- `want(...)`
- `owe_*()` / `owe_*_source()`
- `impl DomainReason for MyError {}` 这种空实现
- `UvsReason::validation_error("msg")` 这种带参数构造

当前正确写法：

```rust
let mut ctx = OperationContext::doing("op");
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
