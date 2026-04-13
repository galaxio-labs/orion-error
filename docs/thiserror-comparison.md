# 与 thiserror 的差异与协作指南

## 定位

- `thiserror`：负责定义错误类型，自动生成 `Display` 和 `Error`
- `orion-error`：负责错误治理，提供分类、错误码、上下文、转换和可观测辅助

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
| 非结构错误转结构错误 | 无 | `ErrorOwe` / `ErrorOweSource` |
| 结构错误跨层转换 | 无 | `ErrorConv` |
| 重试/严重级别判断 | 无 | `is_retryable()` / `is_high_severity()` |

## 推荐模式

```rust
use derive_more::From;
use orion_error::{ErrorCode, ErrorOweSource, StructError, UvsReason};
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
        .owe_sys_source()
        .map(|_| ())?;
    Ok(())
}
```

这里不需要再写空的 `impl DomainReason for AppError {}`。

## 什么时候用 `owe_*()`，什么时候用 `owe_*_source()`

- `owe_*_source()`：默认推荐，保留真实底层 error，适合 `E: std::error::Error`
- `owe_*()`：兼容路径，只保留字符串 detail，适合 `E: Display`

如果你关心：

- `source()`
- `root_cause()`
- 下游监控的根因分类
- 更完整的错误链

默认优先使用 `owe_*_source()`。

## 实践建议

- 服务边界对外暴露 `error_code()` 和受控错误信息
- 领域错误枚举保留少量稳定变体，底层通用错误走 `Uvs(UvsReason)`
- 在关键链路使用 `want(...)` 和 `record(...)`
- 在正常 Rust 错误链路里优先使用 `owe_*_source()`
- 在需要主动包装现有错误对象时使用 `with_source(...)`
