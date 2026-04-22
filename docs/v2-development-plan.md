# V2 Development Plan

更新时间：2026-04-22

本文档用于冻结 `orion-error 0.7.x / V2` 的开发起点与执行顺序。

V2 的目标不是继续给 V1 打补丁，而是开始处理 V1 明确记账但没有假装解决的问题。

## 1. V2 要解决什么

V1 已经把以下边界锁住，但没有彻底解决：

- `StructError: StdError` 的根冲突
- 标准错误生态与结构化错误体系的终局桥接模型
- `StructError` 同时承担 runtime carrier / snapshot / report 的职责过多
- compat API 仍然大量保留在运行时表面

因此 V2 的核心问题只有一句话：

> 运行时结构化错误、标准错误桥接、稳定导出对象，必须开始拆层。

## 2. V2 开发原则

V2 先遵守以下原则：

1. 先落设计与分层，再做大面积行为迁移。
2. 先引入新层，不先粗暴删除旧层。
3. 新增 V2 能力时，优先做可并存的过渡结构。
4. 每一步都必须有测试或编译期约束锁住。
5. 不再让文档口径领先实现太多。

## 3. 第一阶段范围

V2 第一阶段不直接改完所有接口，而是先做三件事：

### 3.1 明确 runtime / snapshot / report 分层

目标：

- `StructError<R>` 优先服务运行时传播
- 快照/导出能力逐步独立为明确对象
- 减少后续继续把所有职责塞回 `StructError` 本体

第一阶段至少要回答：

- 哪些字段属于 runtime carrier
- 哪些字段属于 snapshot / report
- 哪些 trait 约束应该从 runtime carrier 上剥离

### 3.2 明确 bridge 边界

目标：

- 普通 `StdError` 进入结构化体系的桥接路径继续保留
- 已结构化错误上卷路径继续保留
- 但两者的模型层分工开始向 `SourcePayload::Std` / `SourcePayload::Struct` 收敛

第一阶段至少要回答：

- 现在的 source 存储模型怎样平滑迁到更清晰的双通道模型
- `StructError: StdError` 在 V2 中的短期策略和长期策略分别是什么

### 3.3 compat 策略进入真实收缩阶段

目标：

- `want(...)`、`with_source(...)`、`err_wrap(...)`、`owe_*()` 不再只在文档层降级
- 开始明确哪些进入正式 deprecation，哪些只保留桥接价值

第一阶段至少要回答：

- 0.7.x 里哪些旧 API 应先加 `#[deprecated]`
- 哪些旧 API 暂时不能动，因为仍缺完全等价替代

## 4. 第一批可落地任务

按顺序执行：

1. 建立 V2 设计基线文档
2. 明确 runtime / snapshot / report 草案
3. 给 compat API 做分级清单
4. 选一条最小实现切口开始落地

当前建议的最小实现切口是：

- 先补 compat API 分级与 deprecation 计划

原因：

- 这一步不要求立刻拆 `StructError`
- 但能把 V2 的边界管理从“文档说说而已”推进到“代码属性和迁移节奏”

在此之后，V2 第一阶段的下一步基线文档是：

- `docs/v2-runtime-snapshot-report-layering.md`
- `docs/v2-bridge-source-payload.md`
- `docs/v2-structerror-stderror-strategy.md`

## 5. 第一批不做的事

这一阶段先不直接做：

- 直接移除 `StructError: StdError`
- 一次性重写 `StructError` 内部存储
- 一次性删除全部 compat API
- 同时推进多套替代接口

这些动作都太大，容易再次回到边改边漂。

## 6. 当前建议的第一步

V2 现在立刻可做的第一步是：

- 新增一份 compat API 分级文档
- 列清：
  - 立即 `#[deprecated]`
  - 暂缓 `#[deprecated]`
  - 只保留桥接语义
  - V2 预期替代项

然后基于这份清单，先把第一批真正可以安全标记的旧 API 加上 deprecation。

## 7. 当前进度

截至 `2026-04-22`，V2 第一阶段已经完成以下动作：

- `docs/v2-compat-deprecation-plan.md` 已落档
- `docs/v2-runtime-snapshot-report-layering.md` 已落档
- `docs/v2-bridge-source-payload.md` 已落档
- `docs/v2-structerror-stderror-strategy.md` 已落档
- `SourcePayload` / `IntoSourcePayload` / `attach_source(...)` 已落地为公开双通道 source 主路径
- `OperationContext::doing(...)` / `at(...)` 已升级为真实 `action` / `locator` 语义，不再只是命名糖衣
- snapshot 默认 `serde` 已切到 `orion-error.snapshot.v2` 稳定 schema，并补了显式 compat 导出入口
- `OperationContext::want(...)`、`OperationContext::with_want(...)`、`ErrorWith::want(...)` 已在 `0.7.0` 正式进入 `#[deprecated]`
- `ErrorWith::with(...)` 已在 `0.7.0` 正式进入 `#[deprecated]`，主路径迁移到 `attach_context(...)`
- `StructError<R>` 已退出 `StdError`，标准错误生态改由 `into_std()` / `as_std()` /
  `into_dyn_std()` / `into_boxed_std()` 等官方 bridge 承接
- `ErrorWrap::err_wrap(...)` / `WrapStructError::wrap(...)` 已从主代码移除
- `StructError::with_source(...)` / `StructErrorBuilder::source(...)` 已从主代码移除
- `ErrorOweSourceBase::owe_source(...)` / `ErrorOweSource::owe_*_source()` 已从主代码移除
- `ErrorOweBase::owe(...)` 已在 `0.7.0` 正式进入 `#[deprecated]`
- `ErrorOwe::owe_*()` 已从主代码移除
- 新主路径内部已避免再通过 deprecated 旧名自调用
- 仓库内相关测试已对齐到新命名；仅明确验证 compat 行为的测试保留 `#[allow(deprecated)]`
- 公开 README / tutorial / examples 已不再把 `with(...)` 作为上下文主路径
- `SourcePayload` / `IntoSourcePayload` / `attach_source(...)` 已作为 V2 双通道 source 主路径落地
- `OperationContext::doing(...)` / `OperationContext::at(...)` 已写入独立 `action` / `locator` 字段，同时保留 `target` / `path` 兼容投影
- `SnapshotContextFrame` / `StableSnapshotContextFrame` 已携带 `action` / `locator`，runtime -> snapshot -> report 转换链路不再丢失 V2 上下文语义
- `StructError` 的 runtime JSON 历史投影已显式命名为 `compat_serialize()`；默认 `Serialize` 目前只转调这层 compat wrapper
- 已验证：
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all-features -- --test-threads=1`

因此，V2 第一阶段已经从“起步”推进到“主路径可用”状态。后续如果继续推进，应聚焦旧 compat API 的移除窗口、默认稳定 JSON 形状是否切到 `stable_export()`，以及是否在下一破坏性版本里彻底移除 runtime carrier 上的兼容导出入口。
