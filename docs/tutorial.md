# 使用教程

本教程面向 `orion-error` 当前公开 API，以当前源码与测试为准。

## 安装

```toml
[dependencies]
orion-error = "0.7.0"
```

可选特性：

```toml
[dependencies]
orion-error = { version = "0.7.0", features = ["serde"] }
# 或
orion-error = { version = "0.7.0", features = ["tracing"] }
```

默认启用 `log` 和 `derive`。

## 导入约定

- `orion_error::prelude::*`：当前主路径通配导入，只包含 `OrionError`、`StructError`、`IntoAs`、`ErrorWith`、`ErrorWrapAs`、`DefaultErrorPolicy`
- crate root 小集合导入：例如 `use orion_error::{StructError, OrionError};`，适合生产代码显式控制依赖
- `orion_error::runtime::*` / `conversion::*` / `reason::*` / `snapshot::*` / `report::*` / `bridge::*`：按职责分层导入入口，适合需要明确 runtime / snapshot / report 边界的模块
- `orion_error::advanced_prelude::*`：只用于高级协议/schema 检查和迁移测试，不作为业务默认入口
- crate root 下的 compat prelude / compat traits 模块：只用于兼容旧的 `owe(...)` 调用路径

如果是新代码，优先使用 `orion_error::prelude::*` 加少量分层导入，或使用 crate root 小集合导入；不要把业务代码的 import 结构绑定到版本阶段命名上。
如果是维护旧代码，兼容 helper 请显式从 `orion_error::compat_prelude::*` 或 `orion_error::compat_traits::*` 导入。

## 一分钟上手

```rust
use derive_more::From;
use orion_error::{
    prelude::*,
    reason::UvsReason,
    runtime::OperationContext,
};

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum UserError {
    #[orion_error(identity = "biz.user_not_found")]
    UserNotFound,
    #[orion_error(transparent)]
    Uvs(UvsReason),
}

fn load_user(user_id: u64) -> Result<String, StructError<UserError>> {
    let mut ctx = OperationContext::doing("load_user");
    ctx.record_field("user_id", user_id.to_string());

    std::fs::read_to_string("user.json")
        .into_as(UserError::from(UvsReason::system_error()), "read user profile failed")
        .doing("read user profile")
        .with_context(&ctx)
}
```

说明：

- 领域错误一般不必手写 `impl DomainReason`；满足 `From<UvsReason> + Display + PartialEq` 即自动实现。
- 领域错误推荐 derive `OrionError`，并用 `#[orion_error(...)]` 在变体上声明稳定身份。
- `record_field(...)` / `record_meta(...)` 是当前推荐的上下文字段写法。
- `into_as(...)` 是普通错误进入结构化体系的主路径。
- `doing(...)` 会写入 `action` 语义字段；与 `with_context(&ctx)` 组合时，最外层上下文仍决定主语义边界。
- 如果上游已经是 `StructError<_>`，优先用 `err_conv()` 或 `wrap_as(...)`，不要再回退到兼容态的 `owe(...)`。

## 协议消费接口

当前版本已经提供一组可直接使用的协议接口，用于稳定断言和统一出口消费：

- `assert_err_code(...)`
- `assert_err_category(...)`
- `assert_err_identity(...)`
- `StructError::identity_snapshot()`
- `StructError::policy_report()`
- `StructError::into_policy_report()`
- `StructError::policy_snapshot(...)`
- `StructError::http_response(...)`
- `StructError::cli_response(...)`
- `StructError::log_response(...)`
- `StructError::rpc_response(...)`
- `StructError::render_user_debug(...)`
- `StructError::render_user_debug_redacted(...)`

示例：

```rust
use orion_error::{
    assert_err_identity,
    conversion::{ErrorWith, IntoAs},
    reason::{ErrorCategory, UvsReason},
    report::DefaultErrorPolicy,
    runtime::OperationContext,
};

let err = std::fs::read_to_string("config.toml")
    .into_as(UvsReason::system_error(), "read config failed")
    .doing("read config")
    .with_context(OperationContext::doing("load config"))
    .unwrap_err();

assert_err_identity(&err, "sys.io_error", ErrorCategory::Sys);

let identity = err.identity_snapshot();
let policy_view = err.policy_report();
let snapshot = err.policy_snapshot(&DefaultErrorPolicy);
let http = err.http_response(&DefaultErrorPolicy);
let cli = err.cli_response(&DefaultErrorPolicy);
let log = err.log_response(&DefaultErrorPolicy);
let rpc = err.rpc_response(&DefaultErrorPolicy);
let user_debug = err.render_user_debug(&DefaultErrorPolicy);

assert_eq!(identity.code, "sys.io_error");
assert_eq!(policy_view.http_status(&DefaultErrorPolicy), 500);
assert_eq!(snapshot.decision.http_status, 500);
assert_eq!(snapshot.decision.retryable, false);
assert_eq!(http.code, "sys.io_error");
assert_eq!(cli.code, "sys.io_error");
assert_eq!(log.code, "sys.io_error");
assert_eq!(log.reason, "system error");
assert_eq!(rpc.code, "sys.io_error");
assert_eq!(rpc.detail, None);
assert!(user_debug.contains("code          : sys.io_error (Sys)"));
assert!(user_debug.contains("detail        : read config failed"));
```

## 1. 定义领域错误

推荐模式：

```rust
use derive_more::From;
use orion_error::{OrionError, UvsReason};

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum OrderError {
    #[orion_error(identity = "biz.insufficient_funds")]
    InsufficientFunds,
    #[orion_error(identity = "biz.order_not_found")]
    OrderNotFound,
    #[orion_error(transparent)]
    Uvs(UvsReason),
}
```

## 2. 创建结构化错误

### 直接构造

```rust
use orion_error::{reason::UvsReason, runtime::StructError};

let err = StructError::from(UvsReason::validation_error())
    .with_detail("field `email` is required");
```

### Builder 构造

```rust
use orion_error::{
    reason::UvsReason,
    runtime::{OperationContext, StructError},
};

let ctx = OperationContext::doing("validate_request");

let err = StructError::builder(UvsReason::validation_error())
    .detail("field `email` is required")
    .context_ref(&ctx)
    .finish();
```

### 保留真实 source

```rust
use orion_error::{reason::UvsReason, runtime::StructError};

let err = StructError::builder(UvsReason::system_error())
    .detail("failed to read config")
    .source(std::io::Error::other("disk offline"))
    .finish();
```

## 3. 使用上下文

```rust
use orion_error::{
    conversion::ErrorWith,
    runtime::OperationContext,
};

let mut ctx = OperationContext::doing("place_order");
ctx.record_field("order_id", "A-1001");
ctx.record_field("user_id", "42");

let result = check_inventory()
    .doing("check inventory")
    .with_context(&ctx);
```

推荐约定：

- `OperationContext::doing(...)` / `OperationContext::at(...)` 会分别写入 `action` / `locator`
- 错误链上的 `.doing(...)` / `.at(...)` 也会走同一套结构化上下文语义
- `target` / `path` 仍作为兼容投影保留，便于旧代码和旧导出继续读取
- `record_field(...)` 写展示型诊断键值
- `record_meta(...)` 写结构化 metadata
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
use orion_error::{conversion::IntoAs, reason::UvsReason};

std::fs::read_to_string("config.toml")
    .into_as(UvsReason::system_error(), "read config failed")?;

call_http_service()
    .into_as(UvsReason::network_error(), "call http service failed")?;
```

这是当前推荐主路径。

### 兼容路径：只保留 detail

当上游值只实现 `Display`，或者它本身不是一个真正的 error type 时，维护旧代码可以继续使用 `owe(...)`。新代码如果能拿到真实 `std::error::Error`，不要为了省事退回这个路径。

```rust
use orion_error::{compat_prelude::ErrorOweBase, reason::UvsReason};

parse_input().owe(UvsReason::validation_error())?;
run_business_rule().owe(UvsReason::business_error())?;
```

### 自定义 reason

```rust
use orion_error::{conversion::IntoAs, reason::UvsReason};

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
use orion_error::{
    conversion::ErrorWrapAs,
    reason::UvsReason,
    runtime::StructError,
};

fn service_call() -> Result<(), StructError<UvsReason>> {
    repo_call()
        .wrap_as(UvsReason::system_error(), "service call failed")
        .map(|_| ())
}
```

这种方式更适合作为当前公开主路径，用于 service/repository/infrastructure 分层包装。

兼容说明：

- `err_wrap(...)` / `wrap(...)` 仍然保留
- 但它们只属于 compat 入口，不属于当前推荐主路径

### 推荐决策顺序

- 上游是普通 `Error` 类型：优先 `into_as(...)`
- 需要显式声明“这是显式实现了 `RawStdError` 的 raw StdError 类型”时：`raw_source(...)` 后再 `into_as(...)`
- 上游只实现 `Display`：仍走兼容路径 `owe(...)`
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
use orion_error::op_context;

fn process_order(order_id: &str) -> Result<(), MyError> {
    let mut ctx = op_context!("process_order").with_auto_log();
    ctx.record_field("order_id", order_id);
    ctx.info("starting");

    {
        let mut scope = ctx.scoped_success();
        scope.record_field("phase", "validation");
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

推荐新代码默认使用 `OrionError`，不再需要额外 derive `thiserror::Error`：

1. 用 `OrionError` 定义领域错误枚举的展示文案、稳定身份、分类和兼容数值码
2. 用 `derive_more::From` 接入 `UvsReason`
3. 在业务里优先使用 `IntoAs` / `wrap_as(...)` / `err_conv()`
4. 只有已有类型必须实现 `std::error::Error` 或外部 API 要求时，再考虑和 `thiserror` 互操作

导入建议：

- 新代码优先 `use orion_error::prelude::*;` 或 crate root 小集合导入
- 需要明确职责边界时，再使用 `runtime` / `conversion` / `reason` / `report` 等分层模块
- 如果必须维护旧的 `owe(...)`，再显式从 compat prelude 模块导入对应 helper

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

启用 `serde` 后，`StructError` 默认序列化仍是兼容态 runtime projection；它会包含 `want`、`path`、`source_frames`、`source_message` 和 `source_chain`。

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

新的导出主路径优先使用 `snapshot()` / `report()` / `to_stable_snapshot_json()`。

底层 trait object 本体仍然不会直接序列化。

旧的 `owe_*_source()` 属于 `0.6.3` 已公开的兼容语义，仅用于维护旧代码；
`with_source(...)` 当前是推荐的自动 source 分流入口；如果调用点需要强制表达 source 分支，再使用 `with_std_source(...)` / `with_struct_source(...)`；
`owe(...)` / `want(...)` 仍保留为兼容路径，但新代码不建议新增使用。

## 9. 当前版本的兼容提示

以下旧写法不要再新增：

- `OperationContext::with(...)`
- `OperationContext::with_path(...)`
- `with_exit_log()`
- `want(...)`
- `owe_*_source()`
- `owe_*()`
- `owe(...)`，除非正在维护 legacy `Display`-only 场景
- `impl DomainReason for MyError {}` 这种空实现
- `UvsReason::validation_error("msg")` 这种带参数构造

`with_source(...)` 不是兼容路径；它是自动 source 分流入口。只在需要显式 source 分支时改用 `with_std_source(...)` / `with_struct_source(...)`。

当前正确写法：

```rust
let mut ctx = OperationContext::doing("op");
ctx.record_field("key", "value");

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
