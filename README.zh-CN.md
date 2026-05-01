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
- 普通错误第一次进入系统时，用 `into_as(...)`
- 已经结构化的错误跨层时，用 `err_conv()` 或 `wrap_as(...)`
- 到边界时，再做 `report()` / `snapshot()` / `exposure_snapshot(...)`

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
    reason::UvsReason,
    runtime::OperationContext,
};

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum AppReason {
    #[orion_error(identity = "biz.invalid_request")]
    InvalidRequest,
    #[orion_error(transparent)]
    Uvs(UvsReason),
}

fn load_config(path: &str) -> Result<String, StructError<AppReason>> {
    let mut ctx = OperationContext::doing("load_config");
    ctx.record_field("path", path);

    std::fs::read_to_string(path)
        .into_as(AppReason::from(UvsReason::system_error()), "read config failed")
        .doing("read file")
        .with_context(&ctx)
}
```

这个例子里最重要的几件事：

- `AppReason` 是你的领域错误语义
- `StructError<AppReason>` 是统一传播载体
- `into_as(...)` 把普通 Rust 错误接进结构化体系
- `doing(...)` / `with_context(...)` 把操作上下文补进去

对新代码来说，操作语义统一使用 `doing(...)`。

## 新用户先学这 4 个 API

1. `#[derive(OrionError)]`
   定义稳定的业务 reason。
2. `into_as(reason, detail)`
   普通错误第一次进入结构化体系时使用。
3. `err_conv()`
   上游已经是 `StructError<R1>`，这里只是换 reason 类型时使用。
4. `wrap_as(reason, detail)`
   上游已经是 `StructError<_>`，但上层需要建立新的语义边界时使用。

## 一张图理解主路径

```text
std::io::Error
  -> into_as(...)
StructError<RepoReason>
  -> err_conv() 或 wrap_as(...)
StructError<ServiceReason>
  -> report() / snapshot().stable_export() / exposure_snapshot(...)
```

这张图背后的价值是：

- 下层不会各自发明一套错误输出
- 中间层不会轻易丢掉 source 和 context
- 边界层不需要重新解析字符串再猜语义
- 整个系统围绕同一套错误治理模型协作

## 到服务边界时用什么

到了 HTTP / RPC / CLI / 日志边界，主要看这些入口：

- `report()`：人看的诊断信息
- `snapshot().stable_export()`：稳定机器导出
- `exposure_snapshot(...)`

当前协议命名已经统一为 `Exposure*`，不是旧的 `ErrorPolicy*`。

这件事在大型工程里很重要，因为真正容易失控的往往就是边界层：

- 有的接口暴露过多内部细节
- 有的接口又把所有信息都抹平
- 每种协议都在自己拼一套错误结构

`orion-error` 的目标，就是让这些边界投影回到同一套治理模型下。

## 和 `std::error::Error` 的关系

`StructError<R>` 现在不再直接实现 `std::error::Error`。

如果某个边界必须进入标准错误生态，走 interop API：

```rust
use orion_error::{StructError, UvsReason};

let borrowed_err = StructError::from(UvsReason::system_error());
let owned_err = StructError::from(UvsReason::system_error());
let boxed_err = StructError::from(UvsReason::system_error());

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

- `orion_error::reason::UvsReason`
- `orion_error::runtime::OperationContext`
- `orion_error::runtime::source::*`
- `orion_error::report::*`
- `orion_error::protocol::*`
- `orion_error::snapshot::*`

这样可以把普通业务代码固定在一条可预测主路径上，同时在真正需要时仍然保留
清晰的分层边界。

## 导入策略

三类场景：

**应用主路径（默认）**
```rust
use orion_error::prelude::*;
use orion_error::reason::UvsReason;
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
use orion_error::snapshot::*;

// Interop — 必须进入 std::error::Error 生态时
use orion_error::interop::*;
```

**测试 / 迁移**
```rust
use orion_error::dev::prelude::*;
use orion_error::dev::testing::*;
```

## 错误流转路径

`StructError` 进入或穿过你的系统一共有 4 种方式：

```text
原始 std 错误 ──→ into_as(reason, detail) ──→ 首次进入结构化体系
                                                     │
                              ┌──────────────────────┼──────────────────────┐
                              ▼                      ▼                      ▼
                     err_conv()                wrap_as(reason,       as_std / into_std
                     (同语义转换，               detail)               / into_dyn_std
                      只换 reason 类型)         (新语义边界，          (边界需要
                                               包裹已有错误           std::error::Error)
                                               为 source)
```

**1. `into_as(reason, detail)`** — 入口。原始 `std::error::Error` 第一次进入结构化体系。
在每次跨越边界时调用一次（如 FFI 边界，或第三方库错误进入领域层时）。

**2. `err_conv()`** — 跨层转换，保留语义。上游已是 `StructError<R1>`，你只需要
通过 `From` 映射 reason 类型到 `StructError<R2>`。detail、context、source 和
metadata 全部保留。

**3. `wrap_as(reason, detail)`** — 跨层包裹，建立新语义边界。
上游已是 `StructError<R1>`，上层需要自己的 reason。原错误成为新错误的 *source*。

**4. `as_std() / into_std() / into_dyn_std()`** — 出口。把结构化错误桥接到
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
- [使用教程](./docs/tutorial.md)
- [OrionError 与稳定身份](./docs/reason-identity-guide.md)
- [协议契约](./docs/protocol-contract.md)
- [Stable Snapshot Schema](./docs/stable-snapshot-schema.md)
- [与 thiserror 的关系](./docs/thiserror-comparison.md)
- [orion-error-derive README](./orion-error-derive/README.md)

## 维护者说明

如果要发布这一组 crate：

1. 先发布 `orion-error-derive`
2. 等 crates.io 索引传播完成
3. 再发布 `orion-error`
