# V1 Migration Checklist

本文档记录 `orion-error` 在 `0.6.x` 进入 `V1 API` 后的迁移主路径。

如需调整这份 checklist 的主路径、边界约束或评审标准，先以 [V1 修复与评审基线](./v1-fix-and-review-plan.md) 为准。

## 新代码 review checklist

新代码默认按下面规则 review：

- 默认使用 `orion_error::prelude::*` 作为 V1 主路径通配导入
- 如果只想按 trait 分组导入 V1 主路径，可使用 `orion_error::traits_ext::*`
- 旧的 `owe_*()` / `err_wrap(...)` 兼容导入要显式写成 `orion_error::compat_prelude::*` 或 `orion_error::compat_traits::*`
- 普通错误第一次进入结构化体系，优先 `into_as(...)`
- 已结构化错误向上层建立新边界，优先 `wrap_as(...)`
- 普通 source 使用 `with_std_source(...)`
- 结构化 source 使用 `with_struct_source(...)`
- 上下文主命名使用 `at(...)` / `doing(...)`
  - `at(...)` 在 V1 中只是 `with(...)` 的命名糖衣
  - `doing(...)` 在 V1 中只是 `want(...)` 的命名糖衣
- 只有在需要显式声明“这是显式实现了 `RawStdError` 的 raw StdError 类型”时，才使用 `raw_source(...)`
  - 不要重新引入 `E: StdError` blanket 风格入口
  - 不要试图让 `StructError<_>` 走 `raw_source(...)`

review 时如果看到把 `prelude::*` 与 compat 导入混用，应视为需要收敛接口叙事的信号。

## 不建议新增的旧入口

- `with_source(...)`
- `want(...)`
- `owe(...)`
- `owe_source(...)`
- `owe_*()`
- `owe_*_source()`
- `err_wrap(...)`

这些 API 在 `0.6.x` 仍保留兼容，但已经进入 deprecated path。

## 建议替换关系

- `with_source(...)` -> `with_std_source(...)` / `with_struct_source(...)`
- `builder.source(...)` -> `builder.source_std(...)` / `builder.source_struct(...)`
- `want(...)` -> `doing(...)`
- `err_wrap(...)` -> `wrap_as(...)`
- `owe_*_source()` -> `into_as(...)`

## 暂不建议机械替换

- `err_conv()`
  - 它保留独立语义，不等于 `wrap_as(...)`
- `err_wrap(...)`
  - 保留兼容，但不属于 V1 推荐主路径
- `with(...)`
  - 在 V1 中仍承载混合上下文语义，不能简单替换成 `at(...)`
- `owe_*()`
  - 仍可用于兼容 `Display` only 场景；`raw_source(...)` 不扩展到这类值

## 旧代码迁移说明

这一节用于回答一个更具体的问题：

> 看到旧代码里全是 `owe_*()` / `owe_*_source()` / `err_wrap(...)` / `want(...)` / `with_source(...)`，现在到底应该怎么改？

先记一个最小判断顺序：

1. 上游错误是不是第一次进入结构化体系？
2. 上游是不是已经是 `StructError<_>`？
3. 当前附加的是普通 `StdError` source，还是结构化 source？
4. 当前只是命名调整，还是底层语义真的变了？

按这个顺序判断，基本就不会改偏。

### 1. `into_as(...)`

适用场景：

- 上游是普通 `Result<T, E>`
- `E` 是真正的 `std::error::Error`
- 这是第一次进入 `StructError<_>` 体系

推荐写法：

```rust
use orion_error::IntoAs;

std::fs::read_to_string("config.toml")
    .into_as(UvsReason::system_error(), "read config failed")?;
```

旧写法对照：

- `owe_source(reason)` -> `into_as(reason, detail)`
- `owe_sys_source()` / `owe_net_source()` / `owe_validation_source()` -> `into_as(UvsReason::xxx_error(), detail)`

迁移要点：

- `into_as(...)` 要显式给 `reason`
- `into_as(...)` 也要显式给 `detail`
- 不要再期待 blanket `E: StdError` 风格自动兜底

### 2. `raw_source(...)` / `RawStdError`

适用场景：

- 上游错误是你自己的本地 raw `StdError`
- 它没有被库直接 allowlist 到 `UnstructuredSource`
- 你确认它只是普通原始错误，不是结构化错误包装器

推荐写法：

```rust
use std::fmt;

use orion_error::{raw_source, IntoAs, RawStdError, UvsReason};

#[derive(Debug)]
struct MyError;

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "my error")
    }
}

impl std::error::Error for MyError {}
impl RawStdError for MyError {}

call_raw()
    .map_err(raw_source)
    .into_as(UvsReason::system_error(), "call raw failed")?;
```

迁移要点：

- `raw_source(...)` 只用于显式 raw `StdError` 包装
- 不要把它扩展到 `Display` only 值
- 不要试图让 `StructError<_>` 走这条路

### 3. `wrap_as(...)`

适用场景：

- 上游已经是 `Result<T, StructError<R1>>`
- 你不是做简单 reason 映射
- 你要在上层建立一个新的语义边界，并把下层结构化错误完整保留下来

推荐写法：

```rust
use orion_error::ErrorWrapAs;

repo_call()
    .wrap_as(UvsReason::system_error(), "service layer failed")?;
```

旧写法对照：

- `err_wrap(reason)` -> `wrap_as(reason, detail)`

迁移要点：

- `wrap_as(...)` 会新增一层外部 `detail`
- 下层 `StructError` 会保留为结构化 source
- 如果你只是做 reason 类型映射，不要误用 `wrap_as(...)`

### 4. `err_conv()`

适用场景：

- 上游已经是 `StructError<R1>`
- 你只想把 reason 转成 `R2`
- 不是要建立新的上层语义边界

推荐写法：

```rust
repo_call().err_conv()?;
```

迁移要点：

- `err_conv()` 不等于 `wrap_as(...)`
- `err_conv()` 保留的是“同一层错误语义的 reason 类型转换”
- `wrap_as(...)` 保留的是“跨层新边界”

### 5. `with_std_source(...)`

适用场景：

- 你手里已经有一个 `StructError<_>`
- 现在只是要给它附加普通 `StdError` source

推荐写法：

```rust
let err = StructError::from(UvsReason::system_error())
    .with_detail("load config failed")
    .with_std_source(std::io::Error::other("disk offline"));
```

旧写法对照：

- `with_source(err)` -> `with_std_source(err)`
- `builder.source(err)` -> `builder.source_std(err)`

迁移要点：

- `with_std_source(...)` 只用于普通非结构化 source
- 如果 source 本身是 `StructError<_>`，要走 `with_struct_source(...)`

### 6. `with_struct_source(...)`

适用场景：

- 你要把下层 `StructError<_>` 直接挂成 source
- 你希望保留下层的 `reason / detail / metadata / source_frames`

推荐写法：

```rust
let lower = StructError::from(UvsReason::config_error())
    .with_detail("invalid sink defaults");

let err = StructError::from(UvsReason::system_error())
    .with_struct_source(lower);
```

旧写法对照：

- 原来容易误写成 `with_source(lower_err)`
- 现在应明确改成 `with_struct_source(lower_err)`

### 7. `doing(...)` / `at(...)`

适用场景：

- 你只是把旧命名迁移到 V1 推荐主命名
- 不是在修改底层 `OperationContext` 模型

推荐写法：

```rust
let mut ctx = OperationContext::doing("load_config");
ctx.record("path", "config.toml");

read_file()
    .into_as(UvsReason::system_error(), "read config failed")?
    .doing("read file")
    .at(&ctx);
```

旧写法对照：

- `OperationContext::want("op")` -> `OperationContext::doing("op")`
- `.want("step")` -> `.doing("step")`
- `.with(&ctx)` -> `.at(&ctx)` 可以作为命名糖衣使用

迁移要点：

- `doing(...)` 在 `0.6.x` 里只是 `want(...)` 的别名
- `at(...)` 在 `0.6.x` 里只是 `with(...)` 的别名
- V1 不承诺这一步会带来新的底层 target/path 语义

## 常见迁移示例

### 示例一：普通 IO 错误

旧代码：

```rust
std::fs::read_to_string("config.toml").owe_sys_source()?;
```

新代码：

```rust
std::fs::read_to_string("config.toml")
    .into_as(UvsReason::system_error(), "read config failed")?;
```

### 示例二：结构化错误上卷

旧代码：

```rust
repo_call().err_wrap(UvsReason::system_error())?;
```

新代码：

```rust
repo_call().wrap_as(UvsReason::system_error(), "service layer failed")?;
```

### 示例三：上下文主命名

旧代码：

```rust
let mut ctx = OperationContext::want("load_config");
call().want("read file").with(&ctx)?;
```

新代码：

```rust
let mut ctx = OperationContext::doing("load_config");
call().doing("read file").at(&ctx)?;
```

### 示例四：普通 source 与结构化 source 分流

旧代码：

```rust
let err = StructError::from(UvsReason::system_error())
    .with_source(io_err);
```

新代码：

```rust
let err = StructError::from(UvsReason::system_error())
    .with_std_source(io_err);
```

旧代码：

```rust
let err = StructError::from(UvsReason::system_error())
    .with_source(lower_struct_err);
```

新代码：

```rust
let err = StructError::from(UvsReason::system_error())
    .with_struct_source(lower_struct_err);
```
