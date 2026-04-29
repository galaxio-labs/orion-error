# 使用教程

本文档以当前源码、测试和 `examples/` 为准，描述 `orion-error` 的主路径用法。

## 安装

```toml
[dependencies]
orion-error = "0.7.0"
```

常见可选 feature：

```toml
[dependencies]
orion-error = { version = "0.7.0", features = ["serde"] }
# 或
orion-error = { version = "0.7.0", features = ["tracing"] }
# 或
orion-error = { version = "0.7.0", features = ["serde_json"] }
```

默认 feature 包含：

- `derive`
- `log`

## 导入约定

推荐优先使用下面两种方式：

```rust
use orion_error::prelude::*;
use orion_error::reason::UvsReason;
use orion_error::runtime::OperationContext;
```

或：

```rust
use orion_error::{StructError, DefaultExposurePolicy, OrionError};
use orion_error::conversion::{ErrorWith, IntoAs, ErrorWrapAs};
use orion_error::reason::UvsReason;
use orion_error::runtime::OperationContext;
```

其中：

- `prelude::*` 只导出主路径：`OrionError`、`StructError`、`IntoAs`、`ErrorWith`、`ErrorWrapAs`、`DefaultExposurePolicy`
- 需要更明确边界时，再按职责补 `runtime` / `conversion` / `snapshot` / `report` / `bridge` / `reason`
- 旧 `owe(...)` 路径只从 `compat_prelude::*` 或 `compat_traits::*` 导入

## 一分钟上手

```rust
use derive_more::From;
use orion_error::{
    prelude::*,
    reason::UvsReason,
    runtime::OperationContext,
};

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum AppError {
    #[orion_error(identity = "biz.invalid_request")]
    InvalidRequest,
    #[orion_error(transparent)]
    Uvs(UvsReason),
}

fn load_config() -> Result<String, StructError<AppError>> {
    let mut ctx = OperationContext::doing("load config");
    ctx.record_field("path", "config.toml");
    ctx.record_meta("component.name", "config_loader");

    std::fs::read_to_string("config.toml")
        .into_as(AppError::from(UvsReason::system_error()), "read config failed")
        .doing("read config file")
        .with_context(&ctx)
}
```

这个例子覆盖了当前主路径的四个核心点：

- 领域 reason 用 `OrionError` 定义
- 普通错误第一次进入结构化体系用 `into_as(...)`
- 运行时语义上下文用 `doing(...)`
- 诊断字段和 metadata 写到 `OperationContext`

## 1. 定义 reason

### 1.1 领域 reason

新代码推荐直接 derive `OrionError`：

```rust
use derive_more::From;
use orion_error::{OrionError, UvsReason};

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum OrderReason {
    #[orion_error(identity = "biz.order_not_found")]
    OrderNotFound,
    #[orion_error(identity = "biz.insufficient_funds")]
    InsufficientFunds,
    #[orion_error(transparent)]
    Uvs(UvsReason),
}
```

`OrionError` 会为该类型生成：

- `Display`
- `DomainReason`
- `ErrorCode`
- `ErrorIdentityProvider`

默认规则：

- `identity = "biz.order_not_found"` 生成 stable code
- `category` 默认由 `identity` 前缀推导
- `message` 未显式指定时，会从 `identity` 最后一段推导出显示文案
- `code` 未显式指定时，兼容数值码默认是 `500`

### 1.2 通用 reason

`UvsReason` 是 crate 内置的通用错误分类，已经实现：

- `DomainReason`
- `ErrorCode`
- `ErrorIdentityProvider`

常用构造：

- `UvsReason::validation_error()`
- `UvsReason::business_error()`
- `UvsReason::system_error()`
- `UvsReason::network_error()`
- `UvsReason::timeout_error()`
- `UvsReason::core_conf()`
- `UvsReason::logic_error()`

## 2. 构造 `StructError`

### 2.1 直接构造

```rust
use orion_error::{StructError, UvsReason};

let err = StructError::from(UvsReason::validation_error())
    .with_detail("field `email` is required");
```

### 2.2 Builder 构造

```rust
use orion_error::{
    runtime::OperationContext,
    StructError,
    UvsReason,
};

let ctx = OperationContext::doing("validate request");

let err = StructError::builder(UvsReason::validation_error())
    .detail("field `email` is required")
    .context_ref(&ctx)
    .finish();
```

### 2.3 挂载 source

已有 `StructError` 时：

```rust
let err = StructError::from(UvsReason::system_error())
    .with_detail("read config failed")
    .with_std_source(std::io::Error::other("disk offline"));
```

Builder 时：

```rust
let err = StructError::builder(UvsReason::system_error())
    .detail("read config failed")
    .source_std(std::io::Error::other("disk offline"))
    .finish();
```

新代码建议优先显式写：

- `with_std_source(...)`
- `with_struct_source(...)`
- `source_std(...)`
- `source_struct(...)`

`with_source(...)` / `source(...)` 仍可用，但更适合兼容自动分流场景。

## 3. 使用上下文

`OperationContext` 是运行时上下文载体。

```rust
use orion_error::OperationContext;

let mut ctx = OperationContext::doing("place_order");
ctx.record_field("order_id", "A-1001");
ctx.record_field("user_id", "42");
ctx.record_meta("component.name", "order_service");
ctx.record_meta("tenant.id", "demo");
```

推荐区分两类写法：

- `record_field(...)`：给人看的诊断字段
- `record_meta(...)`：机器消费的结构化 metadata

### 3.1 错误侧挂载上下文

```rust
use orion_error::conversion::ErrorWith;

let result = check_inventory()
    .doing("check inventory")
    .with_context(&ctx);
```

上下文语义：

- `OperationContext::doing(...)` 写 `action`
- `OperationContext::at(...)` 写 `locator`
- `StructError::doing(...)` / `at(...)` 是对应的 error-side 语义糖衣
- 兼容投影仍然保留 `target` / `path`

常用读取方法：

- `action_main()`
- `locator_main()`
- `target_main()`
- `target_path()`

## 4. 错误进入和跨层转换

### 4.1 `into_as(...)`

`into_as(...)` 用于“普通错误第一次进入结构化体系”。

```rust
use orion_error::{IntoAs, UvsReason};

let err = std::fs::read_to_string("config.toml")
    .into_as(UvsReason::system_error(), "read config failed")
    .unwrap_err();
```

注意：当前实现没有 blanket `E: std::error::Error` 的通用实现。

当前支持的是一组受控入口：

- `std::io::Error`
- `anyhow::Error`（启用 `anyhow` feature）
- `serde_json::Error`（启用 `serde_json` feature）
- `toml::de::Error` / `toml::ser::Error`（启用 `toml` feature）
- `raw_source(...)` 包装后的下游自定义 `RawStdError`

如果你有第三方错误类型，需要显式 opt-in：

```rust
use std::fmt;
use orion_error::{IntoAs, UvsReason};
use orion_error::bridge::{raw_source, RawStdError};

#[derive(Debug)]
struct ThirdPartyError;

impl fmt::Display for ThirdPartyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "third-party failure")
    }
}

impl std::error::Error for ThirdPartyError {}
impl RawStdError for ThirdPartyError {}

let result: Result<(), ThirdPartyError> = Err(ThirdPartyError);
let err = result
    .map_err(raw_source)
    .into_as(UvsReason::system_error(), "load failed")
    .unwrap_err();
```

### 4.2 `wrap_as(...)`

当上游已经是 `StructError<_>`，而当前层要建立新的语义边界时，使用 `wrap_as(...)`：

```rust
use orion_error::ErrorWrapAs;

let wrapped = repo_call()
    .wrap_as(AppReason::from(UvsReason::system_error()), "service layer failed");
```

### 4.3 `err_conv()`

当只是把下层 reason 收敛到上层 reason，而不想新增一层 detail/source 语义时，使用 `err_conv()`：

```rust
use orion_error::conversion::ErrorConv;

let err = lower_layer_call().err_conv()?;
```

典型前提是：

- `R2: From<R1>`

### 4.4 兼容旧路径

旧代码仍然可以使用：

- `owe(...)`
- `err_wrap(...)`
- `wrap(...)`

但这些都只建议用于兼容维护，不建议作为新代码默认路径。

## 5. source、snapshot、report、bridge 的边界

### 5.1 运行时对象

运行时传播使用：

- `StructError<R>`

### 5.2 稳定导出对象

稳定机器导出使用：

- `ErrorSnapshot`
- `StableErrorSnapshot`

常用入口：

```rust
let snapshot = err.snapshot();
let stable = snapshot.stable_export();
```

### 5.3 人类诊断对象

人类诊断使用：

- `DiagnosticReport`

常用入口：

```rust
let report = err.report();
```

### 5.4 标准错误生态 bridge

`StructError<R>` 本身不再直接实现 `std::error::Error`。

需要进入标准错误生态时，使用显式 bridge：

- `as_std()`
- `into_std()`
- `into_boxed_std()`
- `into_dyn_std()`

## 6. 稳定身份和协议投影

如果 reason 实现了 `ErrorIdentityProvider`，可以直接做稳定身份和协议投影：

```rust
use orion_error::{DefaultExposurePolicy, StructError, UvsReason};
use orion_error::reason::ErrorCategory;

let err = StructError::from(UvsReason::system_error())
    .with_detail("read config failed")
    .doing("load config");

let identity = err.identity_snapshot();
let proto = err.exposure_snapshot(&DefaultExposurePolicy);
let user_debug = proto.render_user_debug();

assert_eq!(identity.code, "sys.io_error");
assert_eq!(identity.category, ErrorCategory::Sys);
assert!(user_debug.contains("sys.io_error"));
```

这几层的职责分别是：

- `identity_snapshot()`：稳定身份视图
- `exposure_snapshot(...)`：最完整的协议输入（携带 identity + decision + report）
- `proto.render_user_debug()`：用户调试摘要
- `proto.to_http_error_json()` 等：出口 JSON 投影

## 7. 测试建议

当前测试 helper：

- `assert_err_code(...)`
- `assert_err_category(...)`
- `assert_err_identity(...)`
- `assert_err_operation(...)`
- `assert_err_path(...)`

这里的 `assert_err_code(...)` 断言的是 stable code 字符串，不是数值 `error_code()`。

示例：

```rust
use orion_error::{IntoAs, UvsReason};
use orion_error::reason::ErrorCategory;
use orion_error::testcase::assert_err_identity;

let err = std::fs::read_to_string("config.toml")
    .into_as(UvsReason::system_error(), "read config failed")
    .unwrap_err();

assert_err_identity(&err, "sys.io_error", ErrorCategory::Sys);
```

如果你要断言数值码，请直接调用：

```rust
use orion_error::reason::ErrorCode;

assert_eq!(err.error_code(), 201);
```

## 8. 推荐实践

- 领域 reason 默认 derive `OrionError`
- 对外稳定协议依赖 stable code，不依赖人类文案
- 第一次进入结构化体系优先 `into_as(...)`
- 已结构化错误跨层包装优先 `wrap_as(...)`
- 只做 reason 收敛优先 `err_conv()`
- 需要稳定导出时使用 `snapshot().stable_export()`
- 需要对外协议时使用 `exposure_snapshot(...)` 或 projection API
- 需要进入标准错误生态时使用显式 bridge API
