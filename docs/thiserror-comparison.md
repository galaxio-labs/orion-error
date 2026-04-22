# 与 thiserror 的差异与协作指南

## 定位

- `thiserror`：负责定义错误类型，自动生成 `Display` 和 `Error`
- `orion-error`：负责错误治理，提供分类、错误码、上下文、转换和可观测辅助

导入约定：

- 新代码优先 `use orion_error::v2::*;` 或 `use orion_error::v2::prelude::*;`
- 维护 V1 风格代码时，优先 `use orion_error::v1::*;`
- 只按 trait 分组导入时可用 `use orion_error::traits_ext::*;`
- 旧的 `owe(...)` / `err_wrap(...)` 兼容导入请显式从 compat prelude / compat traits 模块进入

推荐组合方式是：

- 用 `thiserror` 定义领域错误枚举
- 用 `orion-error` 负责 `UvsReason`、`StructError`、上下文和转换

## 能力对比

| 能力 | thiserror | orion-error |
|---|---|---|
| 定义领域错误 | 强 | 一般 |
| 统一错误分类 | 无 | `UvsReason` |
| 错误码 | 无 | `ErrorCode` |
| 上下文堆栈 | 无 | `OperationContext` |
| 非结构错误转结构错误 | 无 | `IntoAs` |
| 结构错误跨层转换 | 无 | `ErrorConv` / `ErrorWrapAs` |
| 重试/严重级别判断 | 无 | `is_retryable()` / `is_high_severity()` |

## 推荐模式

```rust
use derive_more::From;
use orion_error::{
    conversion::IntoAs,
    reason::{ErrorCode, UvsReason},
    runtime::StructError,
};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, From)]
enum AppError {
    #[error("parse failed")]
    Parse,
    #[error("{0}")]
    Uvs(UvsReason),
}

impl ErrorCode for AppError {
    fn error_code(&self) -> i32 {
        match self {
            Self::Parse => 1000,
            Self::Uvs(reason) => reason.error_code(),
        }
    }
}

fn handle() -> Result<(), StructError<AppError>> {
    std::fs::read_to_string("config.toml")
        .into_as(AppError::from(UvsReason::system_error()), "read config failed")
        .map(|_| ())?;
    Ok(())
}
```

这里不需要再写空的 `impl DomainReason for AppError {}`。

## 什么时候用 `into_as(...)`，什么时候保留 `owe(...)`

- `into_as(...)`：V1 默认推荐，适合 `E: std::error::Error` 第一次进入结构化体系
- `owe(...)`：兼容路径，只保留字符串 detail，适合 `E: Display`

如果你关心：

- `source()`
- `root_cause()`
- 下游监控的根因分类
- 更完整的错误链

默认优先使用 `into_as(...)`。

如果上游已经是 `StructError<_>`，则不要再走 `.into_as(...)` 或兼容态的 `.owe(...)`：

- 做 reason 类型转换时，优先 `err_conv()`
- 做上层语义包装时，优先 `wrap_as(...)`

兼容说明：

- `owe_*_source()` 已从当前主代码移除；新代码优先使用 `into_as(reason, detail)`
- `err_wrap(...)` 仍然保留，但已进入 `0.7.0` deprecated path，属于 compat/bridge 层
- `with_source(...)` 建议改成 `with_std_source(...)` 或 `with_struct_source(...)`
- 如果旧代码必须继续导入这些 compat helper，请和 `prelude::*` 分开写，避免把 V1 主路径和兼容层混成一个默认接口

## 实践建议

- 服务边界对外暴露 `error_code()` 和受控错误信息
- 领域错误枚举保留少量稳定变体，底层通用错误走 `Uvs(UvsReason)`
- 在关键链路使用 `doing(...)` 和 `record(...)`
- 在普通 `StdError` 第一次进入结构化体系时优先使用 `into_as(...)`
- 在需要主动包装现有错误对象时优先使用 `with_std_source(...)`
