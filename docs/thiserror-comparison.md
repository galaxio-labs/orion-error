# 与 thiserror 的差异与协作指南

## 定位

新代码默认不需要 `thiserror`。推荐直接用 `OrionError` 定义领域 reason：

```rust
use derive_more::From;
use orion_error::{prelude::*, reason::UvsReason};

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum AppError {
    #[orion_error(identity = "biz.parse_failed")]
    Parse,
    #[orion_error(transparent)]
    Uvs(UvsReason),
}
```

`OrionError` 会同时生成：

- `Display`
- 稳定协议身份
- 错误分类
- legacy numeric code

因此使用者不需要再为了文案展示去 derive `thiserror::Error`，也不需要手写稳定身份和兼容数值码的重复 `match`。

## 能力对比

| 能力 | thiserror | orion-error |
| --- | --- | --- |
| 定义标准错误类型 | 强 | 不是主要目标 |
| 定义领域 reason | 可用但需要额外实现身份 | `OrionError` 是推荐入口 |
| 统一错误分类 | 无 | `UvsReason` / `ErrorCategory` |
| 稳定协议身份 | 无 | `OrionError` 注解里的 `identity` |
| 兼容数值码 | 无 | `OrionError` 注解里的可选 `code` |
| 上下文堆栈 | 无 | `OperationContext` / `ErrorWith` |
| 非结构错误转结构错误 | 无 | `IntoAs` |
| 结构错误跨层包装 | 无 | `ErrorWrapAs` |

## 推荐模式

```rust
use derive_more::From;
use orion_error::{prelude::*, reason::UvsReason};

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum AppError {
    #[orion_error(identity = "biz.invalid_request")]
    InvalidRequest,
    #[orion_error(transparent)]
    Uvs(UvsReason),
}

fn handle() -> Result<(), StructError<AppError>> {
    std::fs::read_to_string("config.toml")
        .into_as(AppError::from(UvsReason::system_error()), "read config failed")
        .map(|_| ())?;
    Ok(())
}
```

这里不需要：

- `#[derive(thiserror::Error)]`
- 空的 `impl DomainReason for AppError {}`；新代码应 derive `OrionError`
- 手写稳定身份映射
- 手写兼容数值码映射

## 什么时候仍然用 thiserror

只有这些场景需要考虑 `thiserror`：

- 现有错误类型已经作为 `std::error::Error` 暴露给外部 crate。
- 外部 API 要求传入或返回标准错误类型。
- 你需要 `#[source]`、`#[from]` 等 `thiserror` 的标准错误生态能力。

这种情况下，可以把 `thiserror` 类型当作普通 source，通过 `into_as(...)` 或 `with_source(...)` 进入 `StructError<R>`。

## 什么时候用 `into_as(...)`，什么时候保留 `owe(...)`

- `into_as(...)`：当前默认推荐，适合 `E: std::error::Error` 第一次进入结构化体系。
- `owe(...)`：兼容路径，只保留字符串 detail，适合维护旧的 `E: Display` 场景。

如果你关心：

- `source()`
- `root_cause()`
- 下游监控的根因分类
- 更完整的错误链

默认优先使用 `into_as(...)`。

如果上游已经是 `StructError<_>`，则不要再走 `.into_as(...)` 或兼容态的 `.owe(...)`：

- 做 reason 类型转换时，优先 `err_conv()`。
- 做上层语义包装时，优先 `wrap_as(...)`。

## source 建议

`with_source(...)` 是推荐的自动 source 分流入口：

```rust
let err = StructError::from(UvsReason::system_error())
    .with_detail("write file failed")
    .with_source(std::io::Error::other("disk offline"));
```

如果调用点需要强制表达 source 分支，再使用显式 API：

- `with_std_source(...)`：普通 `std::error::Error` source。
- `with_struct_source(...)`：下层 `StructError<_>` source。

兼容说明：

- `owe_*_source()` 只用于维护已经公开过的旧语义；新代码优先使用 `into_as(reason, detail)`。
- `err_wrap(...)` / `wrap(...)` 属于 compat 层；新代码优先使用 `wrap_as(...)`。
- 如果旧代码必须继续导入这些 compat helper，请和 `prelude::*` 分开写，避免把当前主路径和兼容层混成一个默认接口。

## 实践建议

- 新领域 reason 默认 derive `OrionError`。
- 对外协议依赖 `identity`，不要默认依赖兼容数值码。
- 领域错误枚举保留少量稳定变体，底层通用错误走 `Uvs(UvsReason)`。
- 在关键链路使用 `doing(...)`、`record_field(...)` 和 `with_context(...)`。
- 在普通 `StdError` 第一次进入结构化体系时优先使用 `into_as(...)`。
