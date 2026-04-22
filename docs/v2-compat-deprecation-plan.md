# V2 Compat Deprecation Plan

更新时间：2026-04-22

本文档用于给 `0.7.x / V2` 阶段的 compat API 收缩做分级。

目标不是立刻删除所有旧接口，而是先把以下四类分清：

1. 立即可 `#[deprecated]`
2. 继续观察 / 保留 compat
3. 只保留桥接语义
4. 对应的 V2 / V1 推荐替代项

## 1. 分级原则

只有同时满足以下条件，旧 API 才适合优先进入正式 deprecation：

- 已经存在语义更清晰的新接口
- 替代项在文档中已经稳定
- 替代项有测试锁住
- 继续保留旧名字只会制造主路径歧义

如果某个旧接口仍承载 V1/V2 过渡期的重要桥接价值，就先保留，不急着动属性。

## 2. 立即可 `#[deprecated]`

### 2.1 `with_source(...)`

原因：

- 已有明确替代项：`with_std_source(...)` / `with_struct_source(...)`
- 旧名字模糊，会掩盖普通 source 与结构化 source 的边界
- 仓库主代码已完成迁移，已具备直接移除条件

替代项：

- 普通 source -> `with_std_source(...)`
- 结构化 source -> `with_struct_source(...)`

### 2.2 `StructErrorBuilder::source(...)`

原因：

- 已有明确替代项：`source_std(...)` / `source_struct(...)`
- 与 `with_source(...)` 一样，旧名字模糊
- 仓库主代码已完成迁移，已具备直接移除条件

替代项：

- 普通 source -> `source_std(...)`
- 结构化 source -> `source_struct(...)`

### 2.3 `ErrorWrap::err_wrap(...)`

原因：

- 已有明确替代项：`wrap_as(...)`
- 新旧接口语义已经被文档收敛为“compat vs 主路径”

替代项：

- `wrap_as(reason, detail)`

注意：

- 它不是完全同签名替代，因此 note 需要明确“补上 detail”

### 2.4 `WrapStructError::wrap(...)`

原因：

- 当前主路径已是 `wrap_as(...)`
- `wrap(...)` 的语义信息不足

替代项：

- `wrap_as(reason, detail)`

### 2.5 `want(...)` / `with_want(...)`

原因：

- `doing(...)` / `at(...)` 现在已经对应真实的 `action` / `locator` 模型语义
- `want(...)` 家族继续作为主路径只会让 V2 语义边界重新变模糊
- 调用侧替换关系已经足够直接

替代项：

- `OperationContext::want(...)` -> `OperationContext::doing(...)`
- `OperationContext::with_want(...)` -> `OperationContext::with_doing(...)`
- `ErrorWith::want(...)` -> `ErrorWith::doing(...)`

## 3. 继续观察 / 保留 compat

### 3.1 `owe(...)`

原因：

- 它仍然有 `Display` only 场景下的桥接价值
- 目前没有完全等价的新名字覆盖所有 `Display` only 用法

当前策略：

- 已在 `0.7.0` 正式进入 `#[deprecated]`
- 继续保留 compat 实现，承接 legacy `Display`-only 场景
- `owe_*()` 已从主代码移除，不再继续保留

替代项：

- 普通 `StdError` -> `into_as(...)`
- `Display` only 值 -> 暂无完全等价替代；继续兼容

### 3.2 `owe_source(...)` / `owe_*_source()`

原因：

- 推荐主路径已是 `into_as(...)`
- 但旧接口仍覆盖一批 `UvsFrom` 风格快捷写法

当前策略：

- 已完成迁移并从主代码移除
- 维护旧代码时应直接改写到 `into_as(reason, detail)`

替代项：

- `into_as(reason, detail)`

### 3.3 `Serialize for StructError<R>`

原因：

- 它仍承接一批 runtime carrier 的历史 JSON 投影
- 当前 Rust 不能直接对 trait impl 本身施加有效的 `#[deprecated]`
- 但 V2 的长期主路径已经明确转向 `snapshot()` / `report()`

当前策略：

- 暂不对 `Serialize for StructError<R>` 做无效的属性标记
- 已补显式 compat 名字：`err.compat_serialize()`
- 默认 `Serialize` 目前只视为这层 compat runtime projection 的转调
- 新导出代码应优先迁到 `snapshot()` / `report()` / stable snapshot JSON

替代项：

- runtime compat JSON -> `compat_serialize()`
- 稳定导出 -> `snapshot().stable_export()` / `to_stable_snapshot_json()`
- 展示/脱敏 -> `report()` / `into_report()`

## 4. 只保留桥接语义

以下接口在 V2 阶段应明确视为 bridge/compat，而不是主路径：

- `ErrorOweBase`
- `ErrorWrap`
- `WrapStructError`

这不等于立刻删除，而是：

- 导出层继续与主路径分开
- 文档中不再作为推荐入口
- 评审时继续视作 compat 调用

### 4.1 导出面收缩

从 `0.7.0` 开始，这些 legacy trait 的 crate root re-export 也进入
`#[deprecated]`：

- `orion_error::ErrorOweBase`
- `orion_error::ErrorWrap`
- `orion_error::WrapStructError`

维护旧代码时应显式使用：

```rust,ignore
use orion_error::compat_prelude::*;
```

或更窄的：

```rust,ignore
use orion_error::compat_traits::*;
```

`compat_prelude` / `compat_traits` 本身不 deprecated，
因为它们是旧路径维护入口，不是新主路径入口。

## 5. 推荐的执行顺序

按这个顺序推进：

1. `with_source(...)`
2. `StructErrorBuilder::source(...)`
3. `err_wrap(...)`
4. `wrap(...)`
5. 观察仓库内测试、示例、文档和下游影响
6. 推进 `want(...)` / `with_want(...)` / `ErrorWith::want(...)`
7. 观察 `owe(...)` 的 compat 保留窗口
8. 再决定 compat 的最终移除窗口

## 6. 第一批实现建议

第一批代码改动建议只做：

- 给 `with_source(...)` 加 `#[deprecated]`
- 给 `StructErrorBuilder::source(...)` 加 `#[deprecated]`
- 给 `err_wrap(...)` 加 `#[deprecated]`
- 给 `wrap(...)` 加 `#[deprecated]`

并同步：

- 为仓库内部仍需调用这些接口的测试加必要的 `#[allow(deprecated)]`
- 补一条 deprecation 说明到 changelog

## 7. 当前落地状态

截至 `2026-04-22`：

- `StructError::with_source(...)` / `StructErrorBuilder::source(...)` 已从主代码移除
- `ErrorWrap::err_wrap(...)` / `WrapStructError::wrap(...)` 已从主代码移除
- `ErrorOweSourceBase::owe_source(...)` / `ErrorOweSource::owe_*_source()` 已从主代码移除
- `ErrorOweBase::owe(...)` 已实际加上 `#[deprecated(since = "0.7.0", ...)]`
- `ErrorOwe::owe_*()` 已从主代码移除
- 本文第 2 节其余列出的接口都已实际加上 `#[deprecated(since = "0.7.0", ...)]`
- `OperationContext::want(...)`、`OperationContext::with_want(...)`、`ErrorWith::want(...)` 已实际加上 `#[deprecated(since = "0.7.0", ...)]`
- `ErrorWith::with(...)` 已实际加上 `#[deprecated(since = "0.7.0", ...)]`
- crate root 上的 legacy trait re-export 已加 `#[deprecated]`
- `compat_prelude` / `compat_traits` 继续作为维护旧路径的显式入口
- 公开示例、README、教程、versioned namespace 编译测试已改到 `attach_context(...)`
- `with_std_source(...)`、`source_std(...)`、`wrap_as(...)`、`attach_context(...)` 等新主路径内部已不再转调 deprecated 旧接口
- compat 行为测试仍保留，但只在明确验证旧路径时允许 `deprecated`
- `with(...)` 已从“文档层降级”推进到“正式 deprecated + 新主路径迁移完成”

后续仍需观察的 compat 面，主要只剩：

- `owe(...)` 的最终移除窗口
- `Serialize for StructError<R>` 的最终收缩窗口
