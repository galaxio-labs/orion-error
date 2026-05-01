# 与 thiserror 的关系

`orion-error` 和 `thiserror` 不是互斥关系，但它们的定位不同。

## 1. 定位差异

### `thiserror`

更偏向：

- 定义标准 Rust error 类型
- 服务于 `std::error::Error` 生态
- 为 `Display`、`source`、`from` 等标准错误能力提供 derive 支持

### `orion-error`

更偏向：

- 定义运行时结构化错误载体
- 管理上下文、source frame、快照、协议投影
- 为领域 reason 提供稳定身份

## 2. 当前推荐路径

新代码里的领域 reason，默认不需要 `thiserror`。

推荐直接使用 `OrionError`：

```rust
use derive_more::From;
use orion_error::{prelude::*, reason::UvsReason};

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum AppReason {
    #[orion_error(identity = "biz.invalid_request")]
    InvalidRequest,
    #[orion_error(transparent)]
    Uvs(UvsReason),
}
```

这样就能同时获得：

- `Display`
- `DomainReason`
- 稳定身份
- 分类
- 兼容数值码

## 3. 能力对比

| 能力 | thiserror | orion-error |
| --- | --- | --- |
| 定义标准错误类型 | 强 | 不是主要目标 |
| 领域 reason derive | 需要额外补稳定身份 | `OrionError` 是推荐入口 |
| 运行时结构化上下文 | 无 | 有 |
| source frame 追踪 | 无 | 有 |
| stable code / category | 无 | 有 |
| snapshot / report / projection | 无 | 有 |

## 4. 什么时候仍然适合 `thiserror`

下面这些场景仍然适合保留 `thiserror`：

- 你对外公开的就是标准 `std::error::Error` 类型
- 外部库 API 要求你传递标准 error 类型
- 你需要 `#[from]`、`#[source]` 这类标准错误生态能力

这种情况下，`thiserror` 类型可以作为 source 进入 `StructError<R>`。

## 5. 如何和 `orion-error` 配合

最常见的配合方式是：

- 边界外还是标准 error
- 进入你的业务边界后，再转成 `StructError<R>`

例如：

```rust
use orion_error::{IntoAs, UvsReason};

let err = std::fs::read_to_string("config.toml")
    .into_as(UvsReason::system_error(), "read config failed")
    .unwrap_err();
```

注意：当前 `into_as(...)` 不是 blanket `E: std::error::Error` 实现。

如果你的第三方错误类型不在内建支持列表里，需要显式 `raw_source(...)` opt-in。

## 6. `into_as(...)`、`wrap_as(...)`、`upcast()`

当前推荐分工：

- `into_as(...)`
  - 普通错误第一次进入结构化体系
- `wrap_as(...)`
  - 上游已经是 `StructError<_>`，当前层要建立新语义边界
- `upcast()`
  - 上游已经是 `StructError<_>`，当前层只做 reason 收敛

如果上游已经是 `StructError<_>`，不要再回退到：

- `.into_as(...)`

## 7. source 建议

如果你已经在 `StructError` 世界里，推荐优先使用当前 source API：

- `with_source(...)`
- `builder.source(...)`

下面这些显式 API 只在维护旧代码、测试 source 分类或调试 auto-routing 时使用：

- `with_std_source(...)`
- `with_struct_source(...)`
- `builder.source_std(...)`
- `builder.source_struct(...)`

## 8. 实践建议

- 领域 reason 默认 derive `OrionError`
- 标准错误类型继续用 `thiserror` 时，把它们视为边界外或边界前对象
- 稳定协议依赖 stable code，不依赖自然语言文案
- 结构化错误跨层传播时优先使用 `StructError<R>` 体系
