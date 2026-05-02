# OrionError 与稳定身份

本文解释当前实现里的四个概念：

- `DomainReason`
- `OrionError`
- `ErrorCode`
- `ErrorIdentityProvider`

核心结论先放在前面：

- `DomainReason` 解决“这个 reason 能不能作为 `StructError<R>` 的运行时语义载体”
- `OrionError` 解决“这个 reason 能不能低成本地同时获得显示文案、兼容数值码和稳定身份”
- `ErrorCode` 是兼容数值码
- `ErrorIdentityProvider` 才是对外稳定协议的机器主键来源

## 1. 当前 trait 约束

当前 `DomainReason` 很薄，只要求：

```rust,ignore
pub trait DomainReason: PartialEq + Display + Debug + Send + Sync + 'static {}
```

它只负责说明：

- `StructError<R>` 的 reason 类型是显式的
- reason 本身有可显示文本

`DomainReason` 本身不包含：

- 稳定 code
- category
- exposure 决策
- 对外协议投影

## 2. `OrionError` 做了什么

推荐写法：

```rust
use derive_more::From;
use orion_error::{OrionError, UnifiedReason};

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum AppReason {
    #[orion_error(identity = "biz.invalid_request")]
    InvalidRequest,
    #[orion_error(transparent)]
    General(UnifiedReason),
}
```

`OrionError` 会同时生成：

- `Display`
- `DomainReason`
- `ErrorCode`
- `ErrorIdentityProvider`

因此它是当前定义领域 reason 的推荐入口。

## 3. `ErrorCode` 与 `ErrorIdentityProvider` 的区别

### 3.1 `ErrorCode`

`ErrorCode::error_code()` 返回兼容数值码：

- `UnifiedReason::system_error()` 是 `201`
- `OrionError` 未显式声明 `code = ...` 时默认是 `500`

它的定位是：

- 兼容旧系统
- 兼容已有数值码测试
- 保留传统集成接口

### 3.2 `ErrorIdentityProvider`

`ErrorIdentityProvider` 提供两个稳定协议字段：

- `stable_code()`
- `error_category()`

例如：

- `sys.io_error`
- `biz.invalid_request`
- `logic.internal_invariant_broken`

它们用于：

- 稳定断言
- exposure 决策
- HTTP / CLI / log / RPC 投影
- 指标、聚合、告警、跨服务协作

如果系统要依赖稳定机器主键，应该依赖这里，而不是 `Display` 文案。

## 4. `identity` 注解规则

### 4.1 最常用写法

```rust
use orion_error::OrionError;

#[derive(Debug, Clone, PartialEq, OrionError)]
enum AppReason {
    #[orion_error(identity = "biz.order_not_found")]
    OrderNotFound,
}
```

这会产生：

- stable code：`biz.order_not_found`
- 默认显示文案：`order not found`
- category：`Biz`
- 默认兼容数值码：`500`

### 4.2 显式指定 message / code / category

```rust
use orion_error::OrionError;

#[derive(Debug, Clone, PartialEq, OrionError)]
enum AppReason {
    #[orion_error(
        message = "storage temporarily unavailable",
        code = 2000,
        identity = "sys.storage_unavailable",
        category = Sys
    )]
    StorageUnavailable,
}
```

这里：

- `message` 控制 `Display`
- `code` 控制兼容数值码
- `identity` 控制 stable code
- `category` 可显式覆盖，也可由 identity 前缀推导

### 4.3 `transparent`

```rust
use orion_error::{OrionError, UnifiedReason};

#[derive(Debug, Clone, PartialEq, OrionError)]
enum AppReason {
    #[orion_error(transparent)]
    General(UnifiedReason),
}
```

`transparent` 会把以下能力委托给内部单字段类型：

- `Display`
- `ErrorCode`
- `ErrorIdentityProvider`

这适合保留一个 `General(UnifiedReason)` 兜底通道。

## 5. 为什么不能只靠 `Display`

`Display` 适合给人看，不适合当协议主键。

原因：

- 文案可能优化
- 文案可能国际化
- 文案可能带动态值
- 不同调用方很难稳定依赖自然语言文本

例如：

- `read config failed`
- `failed to read config`
- `读取配置失败`

这些都可能描述同一类错误，但它们不适合做稳定协议主键。

相比之下：

- `sys.io_error`
- `biz.order_not_found`

才适合作为跨边界约定。

## 6. `UnifiedReason` 的角色

`UnifiedReason` 是内置的通用错误分类，已经实现：

- `DomainReason`
- `ErrorCode`
- `ErrorIdentityProvider`

这意味着它可以直接：

- 作为 `StructError<UnifiedReason>` 的 reason
- 作为 `#[orion_error(transparent)]` 的底层 reason
- 进入稳定身份和协议投影路径

例如：

- `UnifiedReason::system_error()` -> `sys.io_error`
- `UnifiedReason::network_error()` -> `sys.network_error`
- `UnifiedReason::core_conf()` -> `conf.core_invalid`
- `UnifiedReason::logic_error()` -> `logic.internal_invariant_broken`

## 7. 什么时候必须提供稳定身份

下面这些情况不要只停在 `DomainReason`：

- 要使用 `identity_snapshot()`
- 要用 `assert_err_identity(...)`
- 要输出 HTTP / CLI / log / RPC 响应
- 要做统一 exposure 决策
- 要把错误接入指标、聚合、告警
- 要形成跨 crate 或跨服务的错误契约

这时 reason 应该实现 `ErrorIdentityProvider`，而最省事的做法通常就是 derive `OrionError`。

## 8. 推荐模式

推荐让领域 reason 只有少量真正稳定的业务语义，并保留一个通用兜底分支：

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

这样做的好处是：

- 业务错误有稳定领域语义
- 通用系统错误不需要重复造轮子
- HTTP / RPC / log 等投影可以直接复用 `UnifiedReason` 的稳定身份

## 9. 设计建议

设计 stable code 时，建议遵守：

- 不把动态值写进 code
- 不把实现细节写进 code
- code 尽量表达语义，不表达调用点
- category 保持粗粒度

推荐：

- `biz.invalid_request`
- `biz.order_not_found`
- `conf.feature_invalid`
- `sys.io_error`

不推荐：

- `read_config_failed_for_user_42`
- `primary_db_timeout_us_east_1`
- `error_1`
