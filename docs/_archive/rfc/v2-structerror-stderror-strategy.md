# V2 `StructError: StdError` 策略

更新时间：2026-04-22

本文档记录 `orion-error 0.7.x / V2` 在线上代码中的最终状态：

- `StructError<R>` 已经退出 `std::error::Error`
- 标准错误生态兼容只通过显式 bridge API 完成
- source 主路径已经可以重新使用统一的 `attach_source(...)`

## 1. 当前事实

当前代码里已经不存在：

```rust,ignore
impl<T> std::error::Error for StructError<T> { ... }
```

这意味着：

- `StructError<R>` 是 runtime carrier，不再直接进入标准错误生态
- `std::error::Error::source(&err)` 不再是合法写法
- 普通 `StdError` 和结构化 `StructError<R>` 在类型层已经完成分流

对应的 compile-time 约束已经由源码 doctest 锁住：

```rust,ignore
let err = StructError::from(UvsReason::system_error());
let _ = std::error::Error::source(&err); // compile_fail
```

## 2. 官方 bridge

进入标准错误生态时，必须显式桥接：

```rust,ignore
let owned = err.clone().into_std();
let borrowed = err.as_std();
let erased = err.into_dyn_std();
let boxed = err.into_boxed_std();
```

公开 bridge 类型是：

- `OwnedStdStructError<R>`
- `StdStructRef<'a, R>`
- `OwnedDynStdStructError`

从 owned bridge 回到结构化 runtime carrier 时，使用：

```rust,ignore
let err = owned.into_struct();
```

如果只是读取原始 carrier，可用：

- `OwnedStdStructError::inner()`
- `StdStructRef::inner()`

## 3. 对 source 主路径的影响

`StructError<R>` 退出 `StdError` 之后，下面这组 API 才能稳定成立：

- `IntoSourcePayload`
- `StructError::attach_source(...)`
- `StructErrorBuilder::attach_source(...)`

原因是现在不会再出现：

- `StructError<R>` 同时又掉进 `E: StdError` blanket impl
- 普通 source 和结构化 source 在统一入口里重叠

因此当前模型可以稳定收敛为：

- 普通 `StdError` -> `SourcePayloadKind::Std`
- `StructError<R>` -> `SourcePayloadKind::Struct`

## 4. 迁移口径

旧代码里如果有：

```rust,ignore
std::error::Error::source(&err)
```

现在应该改成下列其中一种：

- 只读当前 carrier 已保存的 source：`err.source_ref()`
- 进入标准错误生态后再读 source：`std::error::Error::source(&err.as_std())`
- 做结构化导出：`err.snapshot()` / `err.report()`

如果旧边界要求 owned `dyn StdError`，使用：

- `err.into_std()`
- `err.into_dyn_std()`
- `err.into_boxed_std()`

## 5. 已完成的退出条件

mini RFC 里为这条线列出的前置条件，现在已经满足：

- 官方 bridge 类型已落地
- `into_std()` / `as_std()` / `into_struct()` 已落地
- source payload 双通道主路径已公开
- runtime / snapshot / report 已有明确分层对象
- 仓库文档和测试已切到“出边界再桥接”的口径

因此这条线当前不再是“计划退出”，而是“已经退出”。

## 6. 评审规则

后续 review 应继续拒绝两类改动：

- 重新给 `StructError<R>` 加回 `impl StdError`
- 重新引入依赖 `StructError<R>: StdError` 的主路径 API

允许继续演进的方向是：

- 完善 bridge 文档和边界示例
- 继续收缩 compat API
- 继续增强 snapshot / report / source payload 的稳定导出能力

## 7. 验证口径

当前默认验证命令：

```bash
cargo test --all-features -- --test-threads=1
```

这里的 `--all-features` 已不再包含任何把 `StructError<R>` 重新变回
`StdError` 的 compat 开关。
