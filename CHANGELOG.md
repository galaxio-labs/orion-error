# Changelog

## 0.8.0

本版是 API 收口版本：删除所有 `0.7.x` 已废弃（deprecated）的兼容路径，缩减根导出，清理公开模块。

### Removed

- **完整删除 `compat_prelude` / `compat_traits` 模块**
- **完整删除 ErrorOwe 系列 trait**：
  - `ErrorOwe` / `ErrorOweBase` / `ErrorOweSource` / `ErrorOweSourceBase`
  - `.owe()` / `.owe_source()` / `.owe_logic()` / `.owe_biz()` / `.owe_rule()` / `.owe_validation()` / `.owe_data()` / `.owe_conf()` / `.owe_res()` / `.owe_net()` / `.owe_timeout()` / `.owe_sys()` 及对应的 `*_source()` 变体
- **删除 `ErrorWith` 上的废弃方法**：
  - `.want()` / `.attach_context()` / `.with()`
- **删除 `OperationContext::with_want()`**
- **删除根 `#[doc(hidden)]` 导出**：
  - `UvsFrom` / `Visibility` / `ErrorConv`（derive 不依赖这三个名字）
- **删除测试文件 `test_error_owe.rs`**

### Changed

- **版本升级**：`0.7.2` → `0.8.0`（breaking change）
- `examples/order_case.rs` 改用 `conversion::ErrorConv` 代替根导入

## 0.7.2

本版是 identity-first 架构收口：将 identity 从诊断层解耦到 exposure 层，清除 `ErrorCode` 在主路径上的残留 bound。

### Added

- `ErrorProtocolSnapshot::from_report(report, identity, policy)` — 从 `DiagnosticReport` 进入 protocol 层的 canonical 入口
- `Visibility::as_str()` — 稳定 lowercase 输出

### Changed

- **`DiagnosticReport` 移除 identity 数据**：
  - 删除 `category` / `code` 字段（只保留诊断字段）
  - 删除整组 exposure bridge 方法（`exposure_identity` / `http_status` / `visibility` / `default_hints` / `decision` / `exposure_snapshot` / `to_exposure_snapshot_json`）
  - `render()` / `render_compact()` / `redacted()` / `render_redacted()` 保留
- **`StructError<T>::report()` / `into_report() `** — 只要求 `DomainReason`（不再需要 `ErrorIdentityProvider`）
- **`From<StructError<T>> for DiagnosticReport`** — 降为 `DomainReason`-only
- **`StableErrorSnapshot.category` / `.code`** — 添加 `#[serde(skip)]`，不在 v2 稳定导出中出现
- **四个 JSON projection** — `category` / `visibility` 改用 `as_str()` 输出稳定小写字符串（`"biz"` / `"sys"` / `"public"` / `"internal"`），不再使用 Rust Debug 格式
- **`exposure_snapshot().to_json()`** — 改用手动 JSON 构造，与其余四个 projection 一致

### Removed

- `DiagnosticReport::category` / `DiagnosticReport::code` 字段
- `DiagnosticReport` 上的 exposure bridge 方法
- `ErrorCode` bound 从以下路径移除（`DomainReason` 已足够）：
  - `IntoSourcePayload for StructError<R>`
  - `with_struct_source()`
  - `wrap()` / `WrapStructErrorAs` / `ErrorWrapAs`
  - `into_std()` / `into_boxed_std()` / `as_std()`
  - `OwnedStdStructError::into_boxed()`

### Fixed

- `DiagnosticReport::exposure_identity()` 不再构造启发式 `"report.unclassified"`，改用真正的 `stable_code()`
- `display_chain()` 不再要求 `ErrorCode`
- `collect_struct_error_source_frames()` 不再要求 `ErrorCode`（compat `error_code` 字段只在 `OwnedDynStdStructError::from` 显式设置）
- `print_error()` 不再要求 `ErrorCode` bound，只需 `DomainReason`

### Docs

- 新增 crate 级决策流程图（`src/lib.rs`），从"我有一个错误"到"我该怎么处理"的 5 条路径
- `report()`、`render()`（`StructError`）、`exposure_snapshot()`、`render()`（`DiagnosticReport`）各增加可运行的 doc-example
- `cli.rs::print_error` 的 doc-test 从 `ignore` 改为可运行

## 0.7.1

依赖更新与文档清理。

### Added

- 文档补充：`doing` 作为主操作 API 的定位说明

### Changed

- README 简化，面向新用户
- 代码格式化

### Docs

- 组织引用更新到 `galaxio-labs`

### Dependencies

- `codecov/codecov-action` 5 → 6
- `softprops/action-gh-release` 2 → 3
- 常规依赖升级

## 0.7.0

相对 `0.6.3`，`0.7.0` 是一次公开 API 口径收敛和协议命名统一。

### Added

- 分层公开模块正式稳定：
  - `reason`
  - `runtime`
  - `bridge`
  - `snapshot`
  - `report`
  - `conversion`
  - `testcase`
- 稳定身份与暴露协议相关公开类型：
  - `ErrorIdentity`
  - `ExposureView`
  - `ExposureDecision`
  - `ErrorProtocolSnapshot`
- 标准错误生态的显式 bridge 能力：
  - `as_std()`
  - `into_std()`
  - `into_boxed_std()`
  - `into_dyn_std()`
  - `OwnedStdStructError`
  - `OwnedDynStdStructError`
  - `StdStructRef`
- `testcase::*` 黑盒断言辅助
- `orion-error-derive` 补齐独立发布到 crates.io 所需的包元数据、README 与发布顺序约束

### Changed

- `StructError<R>` 不再直接实现 `std::error::Error`
- 标准错误生态兼容统一改为显式 bridge API，而不是直接把 `StructError<R>` 当成标准错误类型使用
- 公共协议命名统一到 `Exposure*`：
  - `DefaultExposurePolicy`
  - `ExposurePolicy`
  - `ExposureDecision`
  - `ExposureView`
- 推荐导入方式统一为：
  - `prelude::*` 作为新代码的最小主路径
  - 分层模块导入作为显式边界
- `report()` 的主返回类型明确为 `DiagnosticReport`
- 对外稳定身份明确为 `ErrorIdentity.code`，而不是兼容数值码 `ErrorCode`
- `snapshot().stable_export()`、`identity_snapshot()`、`exposure_snapshot(...)` 成为当前稳定导出/协议投影主路径
- 文档、教程、示例统一收敛到当前主路径：
  - `doing(...)` / `at(...)`
  - `with_context(...)`
  - `into_as(...)`
  - `wrap_as(...)`
  - `with_std_source(...)` / `with_struct_source(...)`
  - `source_std(...)` / `source_struct(...)`

### Deprecated

- 以下兼容路径在 `0.7.x` 中只保留兼容语义，不再作为推荐写法：
  - `want(...)`
  - `with_want(...)`
  - `with(...)`
  - `owe(...)`
  - `err_wrap(...)`
  - `wrap(...)`
  - `with_source(...)`
  - `StructErrorBuilder::source(...)`

新代码应改用当前主路径 API。

### Removed

- 旧协议命名兼容别名：
  - `ErrorPolicy*`
  - `policy_*`
- report/snapshot shape 的公开字段名常量
- 不属于当前 `0.7` 公开设计的旧 root helper / alias

### Migration Notes

- 领域 reason enum 优先使用 `#[derive(OrionError)]`
- 优先使用 `reason::*`、`runtime::*`、`report::*`、`snapshot::*`、`interop::*`、`conversion::*`、`dev::testing::*`，避免继续扩大 root import
- 维护旧代码时才使用 `compat_prelude::*` / `compat_traits::*`
- `0.8` 的 breaking 收口计划见 [docs/0.8-breaking-plan.md](./docs/0.8-breaking-plan.md)

## 0.6.3 - 2026-04-21

这一版的主题是把 V1 主路径正式定型。

### Added
- V1 主路径：
  - `into_as(...)`
  - `wrap_as(...)`
- 受控的一次入场 source 通道：
  - `UnstructuredSource`
  - `RawStdError`
  - `RawSource<E>`
  - `raw_source(...)`
- source 分流接口：
  - `with_std_source(...)`
  - `builder.source_std(...)`
  - `with_struct_source(...)`
  - `source_struct(...)`
- 上下文命名糖衣：
  - `OperationContext::doing(...)`
  - `with_doing(...)`
  - `doing(...)`
  - `at(...)`
- 结构化上卷主路径：
  - `ErrorWrapAs`
  - `WrapStructErrorAs`

### Changed
- `prelude::*` 面向 V1 主路径
- `compat_prelude::*` / `compat_traits::*` 专门承接旧的 `owe_*()` / `err_wrap(...)` 兼容路径
- `into_as(...)` 不再依赖 `E: StdError` blanket 风格入口，避免误吞 `StructError<_>`
- 兼容路径内部也改用 source 分流接口，减少普通 source 和结构化 source 的混用

### Docs
- 主文档和教程统一到 V1 主路径口径
- 历史设计文档补充“历史说明”标记，避免与当前 API 手册混淆

## 0.6.2 - 2026-04-20

这一版的主题是把诊断与安全展示协议补完整。

### Added
- 结构化诊断 metadata
- structured source 的更多保留能力，跨层包装时可保留 `reason / want / path / detail / metadata`
- 统一诊断报告接口：
  - `DiagnosticReport`
  - `report()`
  - `report_redacted()`
  - `render()`
  - `render_redacted()`
- redaction 扩展点 `RedactPolicy`
- `serde` 下更完整的结构化导出，包括带 metadata 的 source frame

### Docs
- README、示例和诊断协议文档同步到新的诊断/脱敏能力

## 0.6.1 - 2026-04-13

这一版的主题是补齐真实 source-chain 与跨层保留。

### Added
- 真实 source-chain 支持，`StructError` 可保留底层 source
- source 相关接口：
  - `with_source(...)`
  - `source_ref()`
  - `root_cause()`
  - `source_chain()`
  - `display_chain()`
- source 的结构化快照能力：
  - `SourceFrame`
  - `source_frames()`
  - `root_cause_frame()`
- source-aware 转换：
  - `ErrorOweSourceBase`
  - `ErrorOweSource`
  - `.owe_source(...)`
  - `.owe_sys_source()`
  - `.owe_validation_source()`
- 跨层包装接口：
  - `WrapStructError`
  - `ErrorWrap`
  - `wrap(reason)`
  - `err_wrap(reason)`

### Changed
- `err_conv()` 在 reason 转换时保留已有 source
- Display 输出增加 source 摘要
- `serde` 输出增加 `source_frames`，同时保留兼容字段 `source_message` / `source_chain`
- `Want` / `Path` 语义收敛，展示与序列化统一输出 `want` / `path`
- 推荐读取接口统一为 `action_main()` / `target_path()`
- source frame 的 `message` 收敛为 reason 文本，完整格式化结果放到 `display`

### Docs
- README、教程、日志文档与协作文档同步到 source-chain 能力

### Notes
- 旧的 `.owe_*()` 仍可用，但只会把上游错误文本写入 `detail`。
- 这是 `0.6.1` 当时的历史建议；从 `0.7.0` 起，真实 `StdError` 第一次进入结构化体系优先使用 `into_as(reason, detail)`，普通 source 使用 `with_std_source(...)`，结构化上卷使用 `wrap_as(reason, detail)`。

## 0.6.0 - 2026-02-22

### Breaking Changes
- `DomainReason` 去掉 `Serialize` 约束：
  - `PartialEq + Display + Serialize`
  - -> `PartialEq + Display`
- `serde` 改为可选 feature，默认不启用
- 移除 `UvsXxxFrom` 旧 trait 族，统一使用 `UvsFrom`
- `UvsReason` 结构简化，除 `ConfigError(ConfErrReason)` 外不再内嵌消息字符串
  - 例如：`UvsReason::system_error("msg") -> UvsReason::system_error()`
- `ErrorOwe` 拆分：
  - `.owe(reason)` 归属 `ErrorOweBase`
  - `.owe_sys()` 等快捷方法归属 `ErrorOwe`

### Added
- 统一转换接口 `UvsFrom`
- `owe_xxx` 消息去重，消息只放在 `detail`
- `op_context!` 宏
- tracing 支持实际接入 `OperationContext`
- 减少依赖：
  - 移除 `derive-getters`
  - `derive_more` 仅保留 `from`

### Migration Notes
- `.owe(...)` 记得引入 `ErrorOweBase`。
- 旧 `UvsReason::*_error("...")` / `UvsReason::core_conf("...")` 改为无参版本。
- 需要序列化时启用：`orion-error = { version = "0.6", features = ["serde"] }`。

## 0.5.5 - 2024-09-20

### Added
- **结构化错误构建器**：`StructError::builder` 支持链式设置 detail、position、context，并与 `context_ref` 共享上下文栈，避免重复分配。
- **上下文作用域 Guard**：新增 `OperationContext::scope()` / `scoped_success()` 与 `OperationScope`，在成功路径自动标记 `mark_suc()`，降低遗漏风险。
- **错误转换性能**：`ErrorOwe::owe_*` 系列仅序列化一次底层错误消息，减少 `to_string()` 开销；上下文内部改用 `Arc<Vec<_>>` 共用堆栈。
- **示例更新**：`examples/order_case.rs`、`logging_example.rs` 演示 builder 与作用域 guard 的推荐写法。

### Docs
- `docs/tutorial.md` 增补 builder/OperationScope 的实践示例。
- `docs/error-handling/05-logging-standards.md`、`06-best-practices.md` 补充 guard 与构建器的使用建议。
- `docs/README.md` 在亮点更新中突出上述能力。

### Compatibility
- 接口向后兼容；若手动调用 `mark_suc()`，仍可与作用域 guard 共存。

## 0.5.0 - 2025-08-25

### Added
- **日志支持**: 为错误上下文添加了完整的日志记录功能，包括 `info`、`debug`、`warn`、`error`、`trace` 方法
- **自动日志记录**: 新增 `with_exit_log` 和 `mark_suc` 方法，支持在对象销毁时自动记录日志
- **PathContext包装器**: 添加了 `PathContext<V: AsRef<Path>` 包装类型，用于区分路径和字符串类型

### Changed
- **结构体重命名**: 将内部结构体从 `WithContext` 重命名为 `OperationContext`，提高代码清晰度
- **ContextTake trait重构**: 解决了 trait 实现冲突问题，移除了 `&PathBuf` 的特定实现，改为使用 `PathContext` 包装器

### Dependencies
- 升级 `thiserror` 从 2.0.12 到 2.0.16
- 升级 `serde_json` 从 1.0.140 到 1.0.143
- 添加 `log` 和 `env_logger` 依赖以支持日志功能

### Docs
- 新增 `LOGGING.md` 文档，详细说明日志功能的使用方法
- 添加 `examples/logging_example.rs` 示例文件
