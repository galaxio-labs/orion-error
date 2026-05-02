# orion-error

[English](./README.md) | [简体中文](./README.zh-CN.md)

面向大型 Rust 工程的结构化错误治理体系。

`orion-error` 不只是一个“定义错误类型”的库。

它更像是一套面向大型服务和分层系统的错误治理框架，用一套统一模型去治理：

- 错误语义
- 运行时传播
- 上下文附着
- 跨层转换
- HTTP / RPC / CLI / 日志边界输出

它解决的是这类问题：

- 业务错误到处是字符串，无法稳定识别
- 每一层都在用不同的错误写法
- 到 HTTP / RPC / CLI / 日志边界时，很难统一输出
- 既想保留底层 source，又不想把错误链搞乱

它的核心思路很简单：

- 用 `#[derive(OrionError)]` 定义稳定的业务 reason
- 用 `StructError<R>` 作为统一运行时载体
- 错误进入系统用 `source_err(...)`（统一入口，支持 std 和 StructError 源）
- 统一用 `source_err(...)` 进入系统，跨层转换用 `conv_err()`
- 到边界时，再做 `report()` / `exposure()`

[![CI](https://github.com/galaxio-labs/orion-error/workflows/CI/badge.svg)](https://github.com/galaxio-labs/orion-error/actions)
[![Coverage Status](https://codecov.io/gh/galaxio-labs/orion-error/branch/main/graph/badge.svg)](https://codecov.io/gh/galaxio-labs/orion-error)
[![crates.io](https://img.shields.io/crates/v/orion-error.svg)](https://crates.io/crates/orion-error)

## 为什么值得用

如果你的项目已经有下面这些需求，这个 crate 会比“手写字符串 + thiserror 零散拼装”更顺手：

- 想让 service / repo / adapter / protocol 层共享同一套错误语言
- 想给业务错误一个稳定 identity
- 想把 detail、context、source 一起保留下来
- 想在 service / repo / adapter 之间清楚地区分错误语义
- 想在外部协议层输出统一结构，而不是每层自己拼 JSON
- 想让错误处理方式能够随着工程规模增长，而不是越写越散

如果你只是写一个很小的本地 enum，`thiserror` 往往就够了。  
如果你是一个分层服务，或者已经有对外协议和诊断输出需求，`orion-error`
会更合适。

可以简单理解成：

- `thiserror` 更像本地建模工具
- `orion-error` 更像全工程的错误治理方案

## 安装

```toml
[dependencies]
orion-error = "0.8"
```

默认 feature 包含 `derive` 和 `log`。

常见可选 feature：

```toml
[dependencies]
orion-error = { version = "0.8", features = ["serde"] }
orion-error = { version = "0.8", features = ["serde_json"] }
orion-error = { version = "0.8", features = ["tracing"] }
orion-error = { version = "0.8", features = ["anyhow"] }
orion-error = { version = "0.8", features = ["toml"] }
```

## 5 分钟上手

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

fn load_config(path: &str) -> Result<String, StructError<AppReason>> {
    let mut ctx = OperationContext::doing("load_config");
    ctx.record_field("path", path);

    std::fs::read_to_string(path)
        .source_err(AppReason::system_error(), "read config failed")
        .doing("read file")
        .with_context(&ctx)
}
```

这个例子里最重要的几件事：

- `AppReason` 是你的领域错误语义
- `StructError<AppReason>` 是统一传播载体
- `source_err(...)` 把错误接进结构化体系（统一入口）
- `doing(...)` / `with_context(...)` 把操作上下文补进去

对新代码来说，操作语义统一使用 `doing(...)`。

## 新用户先学这 4 个 API

1. `#[derive(OrionError)]`
   定义稳定的业务 reason。
2. .source_err(reason, detail)`
   普通错误第一次进入结构化体系时使用。
3. `conv_err()`
   上游已经是 `StructError<R1>`，这里只是换 reason 类型时使用。
4. `exposure(&policy)`
   到服务边界时，投影为 HTTP/RPC/CLI/log 输出。

## 一张图理解主路径

```text
std::io::Error
  ->.source_err(...)
StructError<RepoReason>
  ->.source_err() + conv_err() (错误进入 + 跨层转换)
StructError<ServiceReason>
  -> report() / exposure(...)
```

这张图背后的价值是：

- 下层不会各自发明一套错误输出
- 中间层不会轻易丢掉 source 和 context
- 边界层不需要重新解析字符串再猜语义
- 整个系统围绕同一套错误治理模型协作

## 到服务边界时用什么

到了 HTTP / RPC / CLI / 日志边界，主要看这些入口：

- `report()`：人看的诊断信息
- `identity_snapshot()`：稳定身份识别
- `exposure(...)`

当前协议命名已经统一为 `Exposure*`，不是旧的 `ErrorPolicy*`。

这件事在大型工程里很重要，因为真正容易失控的往往就是边界层：

- 有的接口暴露过多内部细节
- 有的接口又把所有信息都抹平
- 每种协议都在自己拼一套错误结构

`orion-error` 的目标，就是让这些边界投影回到同一套治理模型下。

## 第三方错误类型适配

`source_err` 内置支持 (`io::Error`、`serde_json::Error`、`anyhow::Error`、`toml::Error`)。
自定义类型通过显式 opt-in 接入：

```rust
use orion_error::interop::{raw_source, RawStdError};
use orion_error::prelude::*;
use orion_error::UnifiedReason;

#[derive(Debug)]
struct MyError;

impl std::fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "my custom error")
    }
}

impl std::error::Error for MyError {}

impl RawStdError for MyError {}

let result: Result<(), MyError> = Err(MyError);
let err = result
    .map_err(raw_source)
    .source_err(UnifiedReason::system_error(), "my operation failed")
    .unwrap_err();

assert_eq!(err.source_ref().unwrap().to_string(), "my custom error");
```

> **为什么是 opt-in 而不是 blanket `E: StdError`？** blanket impl 会静默地把
> `StructError<_>` 值也吞为无结构 source，丢失结构化身份和上下文。显式 opt-in
> 确保你明确选择哪些类型进入无结构路径。

**newtype 包装外来类型。** 如果错误类型来自依赖，无法直接实现 `RawStdError`
（orphan rule），使用 newtype：

```rust
use orion_error::interop::{raw_source, RawStdError};
use orion_error::prelude::*;
use orion_error::UnifiedReason;

#[derive(Debug)]
struct ForeignError;

impl std::fmt::Display for ForeignError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "foreign failure")
    }
}
impl std::error::Error for ForeignError {}

#[derive(Debug)]
struct WrappedError(ForeignError);

impl std::fmt::Display for WrappedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}
impl std::error::Error for WrappedError {}
impl RawStdError for WrappedError {}

let result: Result<(), WrappedError> = Err(WrappedError(ForeignError));
let err = result
    .map_err(raw_source)
    .source_err(UnifiedReason::system_error(), "api call failed")
    .unwrap_err();

assert_eq!(err.source_ref().unwrap().to_string(), "foreign failure");
```

## 和 `std::error::Error` 的关系

`StructError<R>` 现在不再直接实现 `std::error::Error`。

如果某个边界必须进入标准错误生态，走 interop API：

```rust
use orion_error::{StructError, UnifiedReason};

let borrowed_err = StructError::from(UnifiedReason::system_error());
let owned_err = StructError::from(UnifiedReason::system_error());
let boxed_err = StructError::from(UnifiedReason::system_error());

let borrowed_std = borrowed_err.as_std();
let owned_std = owned_err.into_std();
let boxed_std = boxed_err.into_boxed_std();

assert!(std::error::Error::source(&borrowed_std).is_none());
assert!(std::error::Error::source(&owned_std).is_none());
assert!(std::error::Error::source(boxed_std.as_ref()).is_none());
```

这样做的好处是：边界更清楚，不会在业务层里无意间退化成普通错误链。

## 推荐导入方式

新代码先从这句开始：

```rust
use orion_error::prelude::*;
```

把它当成业务代码默认入口。只有模块本身在表达架构边界、协议适配层，
或者测试 / schema 校验时，才切到分层导入。

然后按需补少量分层导入，例如：

- `orion_error::reason::UnifiedReason`
- `orion_error::runtime::OperationContext`
- `orion_error::runtime::source::*`
- `orion_error::report::*`
- `orion_error::protocol::*`

这样可以把普通业务代码固定在一条可预测主路径上，同时在真正需要时仍然保留
清晰的分层边界。

## 导入策略

三类场景：

**应用主路径（默认）**
```rust
use orion_error::prelude::*;
use orion_error::reason::UnifiedReason;
use orion_error::runtime::OperationContext;
```

**架构边界** — 分层导入让模块耦合关系显式化。
```rust
// 领域层
use orion_error::prelude::*;
use orion_error::reason::{ErrorCategory, ErrorIdentityProvider};

// 服务 / 适配器层 — StructError 是你的错误载体
use orion_error::{prelude::*, conversion::*};

// 协议 / 边界层 — 只用到投影输出
use orion_error::protocol::*;
use orion_error::report::{DiagnosticReport, RedactPolicy};
use orion_error::protocol::*;

// Interop — 必须进入 std::error::Error 生态时
use orion_error::interop::*;
```

**测试 / 迁移**
```rust
use orion_error::dev::prelude::*;
use orion_error::dev::testing::*;
```

## 错误流转路径

```text
raw std error / StructError ──→.source_err(reason, detail) ──→ 首次进入
                                                                  │
                                                            conv_err()
                                                        (reason 转换)
                                                                  │
                                    report / exposure
```

**1. .source_err(reason, detail)`** — 统一入口。同时支持原始 `std::error::Error`
和已结构化的 `StructError` 源。在每次错误进入系统时使用。

**2. `conv_err()`** — 跨层转换，保留语义。上游已是 `StructError<R1>`，你只需要
通过 `From` 映射 reason 类型到 `StructError<R2>`。detail、context、source 和
metadata 全部保留。

**3. `as_std() / into_std() / into_dyn_std()`** — 出口。把结构化错误桥接到
`std::error::Error` 生态。这些调用是显式的；`StructError<T>` 不直接实现
`StdError`。

## 直接试一下

```bash
cargo test --all-features -- --test-threads=1
cargo run --example order_case
cargo run --example logging_example --features log
```

## 继续阅读

- [English README](./README.md)
- [变更记录](./CHANGELOG.md)
- [文档导航](./docs/README.md)
- [使用教程](./docs/user/tutorial.md)
- [OrionError 与稳定身份](./docs/user/reason-identity-guide.md)
- [协议契约](./docs/user/protocol-contract.md)
- [Protocol Contract](./docs/user/protocol-contract.md)
- [与 thiserror 的关系](./docs/user/thiserror-comparison.md)
- [orion-error-derive README](./orion-error-derive/README.md)

## 维护者说明

如果要发布这一组 crate：

1. 先发布 `orion-error-derive`
2. 等 crates.io 索引传播完成
3. 再发布 `orion-error`
