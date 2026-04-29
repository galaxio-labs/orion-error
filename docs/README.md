# 文档导航

这组文档只描述 `orion-error 0.8.x` 当前已经落地的设计和用法。

如果文档和实现冲突，以 `src/`、`tests/`、`examples/` 为准。

建议阅读顺序：

1. [README](../README.md)
2. [README 中文版](../README.zh-CN.md)
3. [变更记录](../CHANGELOG.md)
4. [使用教程](./tutorial.md)
5. [OrionError 与稳定身份](./reason-identity-guide.md)
6. [协议契约](./protocol-contract.md)
7. [Stable Snapshot Schema](./stable-snapshot-schema.md)
8. [Report / Exposure Boundary](./report-exposure-boundary.md)
9. [日志说明](./LOGGING.md)
10. [与 thiserror 的关系](./thiserror-comparison.md)
11. [0.8 Breaking Plan](./0.8-breaking-plan.md)
12. [0.9 Design Plan](./0.9-design-plan.md)
13. [Release Checklist](./release-checklist.md)

## 当前质量锁

当前仓库已经把下面这些边界收成固定检查：

- root surface compile-fail guard
  - 锁住 root 不再重新暴露 `DomainReason`
  - 锁住 root 不再重新暴露 trait 形态 `ErrorCode`
  - 锁住 root 不再重新暴露 trait 形态 `ErrorIdentityProvider`
- layered export regression tests
  - 锁住 root / `prelude` / `runtime` / `conversion` / `snapshot` / `report` /
    `bridge` / `reason` 的当前职责边界
- feature matrix
  - `bash scripts/check-feature-matrix.sh`
- docs code compile
  - `bash scripts/check-doc-code.sh`
- policy scan
  - `bash scripts/check-v3-policy.sh`

如果后续 public surface、feature、文档主路径发生变化，需要同步更新这些锁，
而不是只改实现或只改 README。

## crates.io 发布顺序

如果发布 `0.8.x` 当前这一组 crate：

1. 先发布 `orion-error-derive`
2. 等 crates.io 索引传播完成
3. 再发布 `orion-error`

原因是 `orion-error` 的默认 `derive` feature 依赖 `orion-error-derive`
的同版本发布包。

## 当前推荐入口

- 运行时传播：`StructError<R>`
- 领域 reason 定义：`#[derive(OrionError)]`
- 普通错误第一次进入结构化体系：`into_as(...)`
- 已结构化错误跨层包装：`wrap_as(...)`
- 完整上下文：`with_context(...)`
- 语义上下文：`doing(...)` / `at(...)`
- 稳定导出：`snapshot().stable_export()`
- 协议投影：`identity_snapshot()` / `exposure_snapshot(...)` / `.to_*_error_json()`

## 当前推荐导入方式

- 新代码的通配导入：`orion_error::prelude::*`
- 分层导入：
  - `orion_error::runtime::*`
  - `orion_error::conversion::*`
  - `orion_error::snapshot::*`
  - `orion_error::report::*`
  - `orion_error::bridge::*`
  - `orion_error::reason::*`

## 分层导入边界

- `orion_error::prelude::*`
  面向新业务代码的最小主路径，只放最常用入口。
- `orion_error::runtime::*`
  运行时传播载体与上下文，如 `StructError`、`OperationContext`。
- `orion_error::conversion::*`
  主路径转换 trait，如 `IntoAs`、`ErrorWith`、`ErrorWrapAs`。
- `orion_error::snapshot::*`
  快照与稳定 schema，如 `ErrorSnapshot`、`StableErrorSnapshot`。
- `orion_error::report::*`
  诊断、redaction、协议投影和各类 projection response。
- `orion_error::bridge::*`
  进入标准错误生态的显式 bridge 类型。
- `orion_error::reason::*`
  reason trait、`UvsReason`、category 与 stable identity 相关能力。
- `orion_error::advanced_prelude::*`
  只建议用于协议/schema 测试、迁移验证和大范围编译覆盖。
  当前主要覆盖 snapshot/report/projection 相关表面，不再承担
  bridge/reason/runtime/conversion 的宽导出。

## 设计边界

- `StructError<R>` 不再直接实现 `std::error::Error`。
- 标准错误生态边界通过显式 bridge API 进入：
  - `as_std()`
  - `into_std()`
  - `into_boxed_std()`
  - `into_dyn_std()`
- `ErrorCode` 是兼容数值码。
- 对外稳定主键是 `ErrorIdentity.code`，也就是 stable code。
