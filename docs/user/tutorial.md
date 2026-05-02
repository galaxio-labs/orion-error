# 使用教程

本文档以当前源码、测试和 `examples/` 为准，描述 `orion-error` 的主路径用法。

## 安装

```toml
[dependencies]
orion-error = "0.8.0"
```

常见可选 feature：

```toml
[dependencies]
orion-error = { version = "0.8.0", features = ["serde"] }
# 或
orion-error = { version = "0.8.0", features = ["tracing"] }
# 或
orion-error = { version = "0.8.0", features = ["serde_json"] }
```

默认 feature 包含：

- `derive`
- `log`

## 导入约定

推荐优先使用下面两种方式：

```rust
use orion_error::prelude::*;
use orion_error::reason::UnifiedReason;
use orion_error::runtime::OperationContext;
```

或：

```rust
use orion_error::{StructError, OrionError};
use orion_error::conversion::{ErrorWith, SourceErr, ConvErr};
use orion_error::protocol::DefaultExposurePolicy;
use orion_error::reason::UnifiedReason;
use orion_error::runtime::OperationContext;
```

其中：

- `prelude::*` 只导出主路径：`OrionError`、`StructError`、`SourceErr`、`ErrorWith`、`ConvErr`
- 新业务代码默认先用 `prelude::*`；只有在模块要显式表达 runtime / conversion / protocol 等边界时，再补 layered imports
- `DefaultExposurePolicy` 只从 `protocol::*` 导入，因为它只属于 exposure/projection 边界
- 需要更明确边界时，再按职责补 `runtime` / `conversion` / `report` / `bridge` / `reason` / `protocol`

## 一分钟上手

```rust
use derive_more::From;
use orion_error::{
    prelude::*,
    reason::UnifiedReason,
    runtime::OperationContext,
};

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum AppReason {
    #[orion_error(identity = "biz.invalid_request")]
    InvalidRequest,
    #[orion_error(transparent)]
    General(UnifiedReason),
}

fn load_config() -> Result<String, StructError<AppReason>> {
    let ctx = OperationContext::doing("load config")
        .with_field("path", "config.toml")
        .with_meta("component.name", "config_loader");

    std::fs::read_to_string("config.toml")
        .source_err(AppReason::system_error(), "read config failed")
        .doing("read config file")
        .with_context(&ctx)
}
```

这个例子覆盖了当前主路径的四个核心点：

- 领域 reason 用 `OrionError` 定义
- 错误进入结构化体系用 `source_err(...)`（统一入口）
- 运行时语义上下文用 `doing(...)`
- 诊断字段和 metadata 写到 `OperationContext`

## 1. 定义 reason

### 1.1 领域 reason

新代码推荐直接 derive `OrionError`：

```rust
use derive_more::From;
use orion_error::{OrionError, UnifiedReason};

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum OrderReason {
    #[orion_error(identity = "biz.order_not_found")]
    OrderNotFound,
    #[orion_error(identity = "biz.insufficient_funds")]
    InsufficientFunds,
    #[orion_error(transparent)]
    General(UnifiedReason),
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

`UnifiedReason` 是 crate 内置的通用错误分类，已经实现：

- `DomainReason`
- `ErrorCode`
- `ErrorIdentityProvider`

常用构造：

- `UnifiedReason::validation_error()`
- `UnifiedReason::business_error()`
- `UnifiedReason::system_error()`
- `UnifiedReason::network_error()`
- `UnifiedReason::timeout_error()`
- `UnifiedReason::core_conf()`
- `UnifiedReason::logic_error()`

## 2. 构造 `StructError`

### 2.1 直接构造

```rust
use orion_error::{StructError, UnifiedReason};

let err = StructError::from(UnifiedReason::validation_error())
    .with_detail("field `email` is required");
```

### 2.2 Builder 构造

```rust
use orion_error::{
    runtime::OperationContext,
    StructError,
    UnifiedReason,
};

let ctx = OperationContext::doing("validate request");

let err = StructError::builder(UnifiedReason::validation_error())
    .detail("field `email` is required")
    .context_ref(&ctx)
    .finish();
```

### 2.3 挂载 source

已有 `StructError` 时：

```rust
use orion_error::{StructError, UnifiedReason};

let err = StructError::from(UnifiedReason::system_error())
    .with_detail("read config failed")
    .with_source(std::io::Error::other("disk offline"));

assert_eq!(err.source_ref().unwrap().to_string(), "disk offline");
```

Builder 时：

```rust
use orion_error::{StructError, UnifiedReason};

let err = StructError::builder(UnifiedReason::system_error())
    .detail("read config failed")
    .source(std::io::Error::other("disk offline"))
    .finish();

assert_eq!(err.source_ref().unwrap().to_string(), "disk offline");
```

主路径建议优先使用：

- `with_source(...)`
- `source(...)`

它们会自动处理：

- 普通 `StdError`
- 已结构化的 `StructError<_>`

下面这些显式 API 属于底层/诊断/测试入口，不作为新业务代码主路径：

- `with_std_source(...)`
- `with_struct_source(...)`
- `source_std(...)`
- `source_struct(...)`

## 3. 使用上下文

`OperationContext` 是运行时上下文载体。

```rust
use orion_error::OperationContext;

let ctx = OperationContext::doing("place_order")
    .with_field("order_id", "A-1001")
    .with_field("user_id", "42")
    .with_meta("component.name", "order_service")
    .with_meta("tenant.id", "demo");
```

推荐区分两类写法：

- `with_field(...)`：给人看的诊断字段（chain 模式）
- `with_meta(...)`：机器消费的结构化 metadata（chain 模式）
- `record_field(...)` / `record_meta(...)`：当已有可变引用时使用

### 3.1 错误侧挂载上下文

```rust
use orion_error::prelude::*;
use orion_error::{OperationContext, StructError, UnifiedReason};

fn check_inventory() -> Result<(), StructError<UnifiedReason>> {
    Err(StructError::from(UnifiedReason::business_error()).with_detail("inventory unavailable"))
}

let mut ctx = OperationContext::doing("place_order");
ctx.record_field("order_id", "A-1001");

let result = check_inventory()
    .doing("check inventory")
    .with_context(&ctx);

assert!(result.is_err());
```

上下文语义：

- `OperationContext::doing(...)` 写 `action`
- `OperationContext::at(...)` 写 `locator`
- `StructError::doing(...)` / `at(...)` 是对应的 error-side 语义糖衣
- 兼容投影仍然保留 `target` / `path`

常用读取方法：

- `action_main()`
- `locator_main()`
- `target_path()`

## 4. 错误进入和跨层转换

### 4.1 `source_err(reason, detail)`

`source_err(reason, detail)` 是统一入口，同时支持原始 `std::error::Error` 和已结构化的 `StructError<_>` 源。

```rust
use orion_error::prelude::*;
use orion_error::UnifiedReason;

let err = std::fs::read_to_string("config.toml")
    .source_err(UnifiedReason::system_error(), "read config failed")
    .unwrap_err();
```

`source_err` 支持常见的标准错误类型和已结构化的 `StructError` 源。

当前支持的是一组受控入口：

- `std::io::Error`
- `anyhow::Error`（启用 `anyhow` feature）
- `serde_json::Error`（启用 `serde_json` feature）
- `toml::de::Error` / `toml::ser::Error`（启用 `toml` feature）
- `raw_source(...)` 包装后的下游自定义 `RawStdError`

如果你有第三方错误类型，需要显式 opt-in：

```rust
use std::fmt;
use orion_error::prelude::*;
use orion_error::UnifiedReason;
use orion_error::interop::{raw_source, RawStdError};

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
    .source_err(UnifiedReason::system_error(), "load failed")
    .unwrap_err();
```


### 4.2 `conv_err()`

当只是把下层 reason 收敛到上层 reason，而不想新增一层 detail/source 语义时，使用 `conv_err()`：

```rust
use derive_more::From;
use orion_error::{OrionError, StructError, UnifiedReason};
use orion_error::conversion::ConvErr;
use orion_error::conversion::ToStructError;

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum RepoReason {
    #[orion_error(transparent)]
    General(UnifiedReason),
}

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum ServiceReason {
    #[orion_error(transparent)]
    Repo(RepoReason),
}

fn lower_layer_call() -> Result<(), StructError<RepoReason>> {
    Err(RepoReason::system_error().to_err()
        .with_detail("read config failed"))
}

fn upper_layer_call() -> Result<(), StructError<ServiceReason>> {
    lower_layer_call().conv_err()?;
    Ok(())
}

let err = upper_layer_call().unwrap_err();
assert_eq!(err.detail().as_deref(), Some("read config failed"));
```

典型前提是：

- `R2: From<R1>`

## 5. source、report、bridge 的边界

### 5.1 运行时对象

运行时传播使用：

- `StructError<R>`

### 5.2 人类诊断对象

人类诊断使用：

- `DiagnosticReport`

常用入口：

```rust
use orion_error::{StructError, UnifiedReason};

let err = StructError::from(UnifiedReason::system_error())
    .with_detail("read config failed");

let report = err.report();

assert_eq!(report.reason(), "system error");
```

### 5.4 标准错误生态 bridge

`StructError<R>` 本身不再直接实现 `std::error::Error`。

需要进入标准错误生态时，使用显式 bridge：

- `as_std()`
- `into_std()`
- `into_boxed_std()`
- `into_dyn_std()`

## 6. 稳定身份和协议投影

### 6.1 稳定身份

每个错误变体都有一个**永久的机器可读名称**，不随文案或重构改变：

```rust
use orion_error::{OrionError, StructError};
use orion_error::reason::ErrorIdentityProvider;

#[derive(Debug, PartialEq, OrionError)]
enum ApiReason {
    #[orion_error(identity = "biz.invalid_input")]
    InvalidInput,
}

// 这个字符串是契约——监控、客户端、网关都依赖它：
assert_eq!(ApiReason::InvalidInput.stable_code(), "biz.invalid_input");
assert_eq!(ApiReason::InvalidInput.error_category().as_str(), "biz");
```

对比不稳定 vs 稳定：

| 不稳定 | 稳定 |
|--------|------|
| `"invalid input"`（显示文案可能改） | `"biz.invalid_input"`（永久） |
| `100`（数值码可能冲突） | `"biz.invalid_input"`（带命名空间） |
| `ApiReason::InvalidInput`（Rust 路径可能重构） | `"biz.invalid_input"`（独立于源代码） |

```text
    biz    .    invalid_input
   ────         ────────────
   category      stable code
   (conf/biz     不变的业务语义
    /logic/sys)
```

### 6.2 协议投影

同一个错误，对不同的协议边界输出**不同的 JSON 形状**，不需要手写映射：

```rust
use orion_error::{OrionError, StructError};
use orion_error::protocol::DefaultExposurePolicy;
use orion_error::UnifiedReason;

#[derive(Debug, PartialEq, OrionError)]
enum ApiReason {
    #[orion_error(identity = "biz.invalid_input")]
    InvalidInput,
    #[orion_error(transparent)]
    General(UnifiedReason),
}

let err = StructError::from(ApiReason::system_error())
    .with_detail("disk offline at /dev/sda");

let proto = err.exposure(&DefaultExposurePolicy);

// HTTP 响应——最小字段，对外安全
let http = proto.to_http_error_json().unwrap();
assert_eq!(http["status"], 500);                // 内部错误
assert_eq!(http["message"], "system error");    // 用 reason，不用 detail

// 日志输出——完整上下文，方便排查
let log = proto.to_log_error_json().unwrap();
assert_eq!(log["detail"], "disk offline at /dev/sda");  // 完整 detail
assert!(log["source_frames"].is_array());                  // source 链

// RPC 响应——隐藏内部细节
let rpc = proto.to_rpc_error_json().unwrap();
assert!(rpc["detail"].is_null()); // internal → 隐藏 detail

// CLI 输出——人类可读摘要
let cli = proto.to_cli_error_json().unwrap();
assert_eq!(cli["summary"], "system error: disk offline at /dev/sda");
```

**核心概念**：错误是一个三维物体，每个协议边界看到的是它投下的不同形状的影子。`ExposurePolicy` 决定哪一面对外可见。

```text
      错误本身（StructError<R>）
              │
    ┌─────────┼──────────┐
    │         │          │
    ▼         ▼          ▼
  HTTP      RPC        Log
  {status,  {code,     {code, detail,
   message}  detail}    source_frames}
```

### 6.3 入口选择

- `identity_snapshot()`：查看稳定身份
- `exposure(...)`：完整协议输入（identity + decision + report）
- `to_*_error_json()`：协议边界出口 JSON

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
use orion_error::prelude::*;
use orion_error::UnifiedReason;
use orion_error::reason::ErrorCategory;
use orion_error::dev::testing::assert_err_identity;

let err = std::fs::read_to_string("config.toml")
    .source_err(UnifiedReason::system_error(), "read config failed")
    .unwrap_err();

assert_err_identity(&err, "sys.io_error", ErrorCategory::Sys);
```

如果你要断言数值码，请直接调用：

```rust
use orion_error::reason::ErrorCode;
use orion_error::{StructError, UnifiedReason};

let err = StructError::from(UnifiedReason::system_error());
assert_eq!(err.reason().error_code(), 201);
```

## 8. 推荐实践

- 领域 reason 默认 derive `OrionError`
- 对外稳定协议依赖 stable code，不依赖人类文案
- 所有错误统一使用 `source_err(...)` 进入结构化体系
- 只做 reason 收敛优先 `conv_err()`
- 需要协议暴露时使用 `exposure(&policy)`
- 需要对外协议时使用 `exposure(...)` 或 projection API
- 需要进入标准错误生态时使用显式 interop API
