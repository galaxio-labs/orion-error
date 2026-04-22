# 更新日志 (CHANGELOG)

## [v0.7.0] - 2026-04-21

### ✨ V2 启动
- **版本切换到 `0.7.0`**：以 `0.6.3` 的 V1 收口结果为基线，正式进入 V2 第一阶段开发。
- **V2 计划落档**：新增 `docs/v2-development-plan.md`，冻结 V2 的起点、执行顺序与第一阶段范围。
- **compat 收缩分级落档**：新增 `docs/v2-compat-deprecation-plan.md`，明确哪些旧 API 立即进入 `#[deprecated]`，哪些仍只保留 compat / bridge 语义。

### 🔄 兼容接口收缩
- 为以下旧接口添加 `#[deprecated(since = "0.7.0", ...)]`：
  - `StructError::with_source(...)`
  - `StructErrorBuilder::source(...)`
  - `ErrorWrap::err_wrap(...)`
  - `WrapStructError::wrap(...)`
  - `ErrorOweSourceBase::owe_source(...)`
  - `ErrorOweSource::{owe_logic_source, owe_biz_source, owe_rule_source, owe_validation_source, owe_data_source, owe_conf_source, owe_res_source, owe_net_source, owe_timeout_source, owe_sys_source}(...)`
- `with_std_source(...)`、`source_std(...)`、`wrap_as(...)` 等新主路径内部不再转调 deprecated 旧接口，避免库内部告警噪音。
- crate root 上 legacy trait re-export 进入 `#[deprecated]`；维护旧路径请显式使用 `compat_prelude::*` / `compat_traits::*`。
- 仓库内测试已切到新命名；只有明确验证 compat 行为的测试保留 `#[allow(deprecated)]`。

### 🧱 Runtime / Snapshot / Report 分层
- 新增 `StructErrorSnapshot` 第一阶段能力：
  - `StructError::snapshot()`
  - `StructError::into_snapshot()`
  - `StructErrorSnapshot::stable_export()`
  - `StructErrorSnapshot::into_stable_export()`
  - `StructErrorSnapshot::compat_export()`
  - `StructErrorSnapshot::report()`
  - `StructErrorSnapshot::into_report()`
- 新增 stable snapshot schema：
  - `StableStructErrorSnapshot`
  - `StableSnapshotContextFrame`
  - `StableSnapshotSourceFrame`
  - `STABLE_SNAPSHOT_SCHEMA_VERSION = "orion-error.snapshot.v1"`
  - `snapshot().to_stable_snapshot_json()`（`serde_json` feature 下）
- 补齐 runtime / snapshot / stable / report 的只读转换路径：
  - `From<&StructError<_>>` / `From<StructError<_>>` 到 `StructErrorSnapshot`
  - `From<&StructError<_>>` / `From<StructError<_>>` 到 `StableStructErrorSnapshot`
  - `From<&StructError<_>>` / `From<StructError<_>>` 到 `ErrorReport`
  - `From<&StructErrorSnapshot>` / `From<StructErrorSnapshot>` 到 `StableStructErrorSnapshot`
  - `From<&StructErrorSnapshot>` / `From<StructErrorSnapshot>` 到 `ErrorReport`
  - `StableStructErrorSnapshot::compat_export()` / `report()`，作为有损兼容投影

### 🌉 Bridge / Source Payload
- 内部 source 存储拆成 `InternalSourcePayload::Std / Struct` 双通道，避免继续用单一 source 概念混淆普通 source 与结构化 source。
- 新增默认关闭的 `struct-error-std` feature，作为 V2 过渡兼容开关：
  - 默认构建下，`StructError<R>` 本体不再直接进入 `StdError` 生态。
  - 需要兼容旧边界时，可显式开启该 feature。
  - 标准错误生态兼容应优先改用 `into_std()` / `as_std()` / `into_boxed_std()` / `into_dyn_std()` 等显式 bridge。
- 新增公开 bridge 类型：
  - `OwnedStdStructError<R>`
  - `StdStructRef<'a, R>`
  - `OwnedDynStdStructError`
- 新增 bridge 方法：
  - `StructError::into_std()`
  - `StructError::as_std()`
  - `StructError::into_boxed_std()`
  - `StructError::into_dyn_std()`
- `anyhow::Error` 的 `into_as(...)` 只识别顶层官方 `OwnedDynStdStructError`；普通 `anyhow` 仍按未结构化错误处理，不扫描 source 链，不猜第三方 wrapper。
- 新增只读 source payload 观察 API：
  - `SourcePayloadKind`
  - `SourcePayloadRef<'_>`
  - `StructError::source_payload()`
  - `StructError::source_payload_kind()`
- 仍不公开 `attach_source(...)` / `IntoSourcePayload`，也不提供 `E: StdError` blanket。

### 📚 文档同步
- 顶层 `README.md`、`docs/README.md` 与 V2 计划文档已同步到 `0.7.0` 口径。
- 明确区分：
  - 已正式 deprecated：`with_source(...)`、`builder.source(...)`、`err_wrap(...)`、`wrap(...)`、`owe_source(...)`、`owe_*_source()`
  - 仍为 compat / 文档层降级：`want(...)`、`with(...)`、`owe_*()`
- 新增并持续更新：
  - `docs/v2-runtime-snapshot-report-layering.md`
  - `docs/v2-stable-snapshot-schema.md`
  - `docs/v2-bridge-source-payload.md`
  - `docs/v2-structerror-stderror-strategy.md`

### 🧪 验证
- 通过：
  - `cargo fmt --all -- --check`
  - `cargo test -- --test-threads=1`
  - `cargo test --all-features -- --test-threads=1`
  - `cargo test --no-default-features --features serde,serde_json,anyhow,toml -- --test-threads=1`

## [v0.6.3] - 2026-04-21

### ✨ 能力更新
- **V1 主路径正式落地**：普通错误第一次进入结构化体系，统一推荐使用 `into_as(...)`；已结构化错误跨层建立新语义边界，统一推荐使用 `wrap_as(...)`。
- **`into_as(...)` 入口收紧**：不再依赖 `E: StdError` blanket 风格入口，改为封闭的 `UnstructuredSource` 通道，避免误吞 `StructError<_>`。
- **显式 raw source 逃生门**：新增 `RawStdError`、`RawSource<E>` 与 `raw_source(...)`，只允许下游本地 raw `StdError` 类型显式 opt-in。
- **source 接口分流补齐**：新增 `with_std_source(...)` 与 `builder.source_std(...)`，与 `with_struct_source(...)` / `source_struct(...)` 形成明确分工。
- **上下文命名糖衣落地**：新增 `OperationContext::doing(...)`、`with_doing(...)` 以及错误链上的 `doing(...)` / `at(...)` 命名糖衣；在 `0.6.x` 中它们仍保持 V1 约定下的别名语义。
- **结构化上卷新接口**：新增 `ErrorWrapAs` / `WrapStructErrorAs`，支持 `wrap_as(reason, detail)` 作为新的公开主路径。

### 🔄 行为与导出面调整
- **导出层分流**：`prelude::*` / `traits_ext::*` 现在面向 V1 主路径；新增 `compat_prelude::*` / `compat_traits::*` 承载旧的 `owe_*()` / `err_wrap(...)` 路径。
- **兼容路径内部对齐**：`owenance` 内部改用 `with_std_source(...)`，减少普通 source 与结构化 source 通道混用。

### 📚 文档收口
- **主文档统一到 V1 口径**：更新 `README.md`、`docs/README.md`、`docs/tutorial.md`、`docs/thiserror-comparison.md`、`docs/LOGGING.md`、`docs/v1-migration-checklist.md`。
- **迁移说明细化**：`docs/v1-migration-checklist.md` 现已补充新接口说明、旧代码替换规则和典型迁移示例，可直接指导 `0.6.x` 旧代码改造。
- **修复与评审基线落档**：新增 `docs/v1-fix-and-review-plan.md`，冻结 V1 正确解法与评审顺序。
- **V1 结案说明落档**：新增 `docs/v1-closure-summary.md`，记录实现层、主文档层和历史设计文档的最终收口结论。
- **历史设计文档降级**：`docs/error-handling/01-08` 统一补充“历史设计说明”与 V1 对照，避免再被误读为当前 API 手册。

### 🧪 验证
- 新增 `wrap_as(...)` 直接测试，锁住 `detail`、source chain 与 metadata 保留行为。
- 通过：
  - `cargo fmt --all -- --check`
  - `cargo test --all-features -- --test-threads=1`

## [v0.6.2] - 2026-04-20

### ✨ 能力更新
- **结构化诊断 metadata**：错误上下文与 source frame 现在都可以携带稳定、机器可读的 metadata，便于上层做分类、聚合与诊断。
- **structured source 保留能力增强**：跨层包装 `StructError` 时，可以显式保留 source 的 `reason / want / path / detail / metadata`，避免降级为纯文本 source。
- **统一诊断报告接口**：新增 `ErrorReport`、`report()`、`report_redacted()`、`render()`、`render_redacted()`，同时支持结构化查看和文本输出。
- **安全展示协议落地**：新增统一 redaction 扩展点 `RedactPolicy`，可以在日志或外发前对诊断信息做集中脱敏。
- **结构化导出更完整**：启用 `serde` feature 后，可直接导出 `ErrorReport` 与带 metadata 的 source frame 数据。

### 📚 文档与示例
- README、`order_case` 示例、诊断协议设计文档已同步更新到上述能力。

### 🧪 验证
- `cargo test --all-features -- --test-threads=1`

## [v0.6.1] - 2026-04-13

### ✨ 新增
- **source-chain 支持**：`StructError` 现在可保存真实底层错误 source，并实现标准 `std::error::Error::source()`。
- **source 辅助接口**：新增 `with_source(...)`、`source_ref()`、`root_cause()`、`source_chain()`、`display_chain()`。
- **source 结构化快照**：新增 `SourceFrame`、`source_frames()`、`root_cause_frame()`，用于把 source chain 输出为稳定结构化数据；`err_wrap()` 现在会把下层 `StructError` 的 `error_code`、`reason`、`want`、`path`、`detail` 写入 source frame。
- **source-aware 转换**：新增 `ErrorOweSourceBase` / `ErrorOweSource`，支持 `.owe_source(...)`、`.owe_sys_source()`、`.owe_validation_source()` 等保留真实 source 的转换接口。
- **跨层包装接口**：新增 `WrapStructError` / `ErrorWrap`，支持 `wrap(reason)` 与 `err_wrap(reason)`，用于 service/repository 等分层场景中把下层 `StructError` 作为上层错误的 source 保留下来。

### 🔄 行为改进
- **`err_conv()` 保留 source**：`StructError<R1>` 转为 `StructError<R2>` 时，不再丢失原有 source。
- **Display 补充 source 摘要**：结构化错误的显示输出增加 `Source` 行，便于快速定位底层错误。
- **`serde` 输出补充 source 结构化字段**：启用 `serde` feature 时，序列化结果新增 `source_frames`，并继续保留 `source_message` 与 `source_chain` 兼容字段；底层 trait object 本体不直接序列化，`debug` 字段默认也不序列化。
- **`Want` / `Path` 语义收敛**：`OperationContext::want(...)` 固定表示最外层调用目标；链式 `.want(...)` 只追加内部路径，展示与序列化同步输出 `want` / `path`。
- **目标读取 API 收敛**：新增 `target_main()` / `target_path()` 作为推荐读取接口，并移除尚未稳定的 `operation_want()`。
- **source frame 消息稳定化**：`StructError` source frame 的 `message` 收敛为 reason 文本，完整格式化输出放入 `display`。

### 📚 文档对齐
- **顶层文档重写**：更新 `README.md` 到当前 `0.6.x` API。
- **教程重写**：更新 `docs/tutorial.md`，统一使用 `record(...)`、无参 `UvsReason::*_error()` 构造器、`owe_*_source()` 等当前接口。
- **日志文档修正**：更新 `docs/LOGGING.md`，统一到 `with_auto_log()` / `scoped_success()`。
- **协作文档修正**：更新 `docs/thiserror-comparison.md` 与 `docs/error-handling/README.md`。
- **文档导航修正**：更新 `docs/README.md`，明确旧版本写法已过期。

### 🧪 验证
- 通过：
  - `cargo test --all-features -- --test-threads=1`

### 使用提示
- 旧的 `.owe_*()` 仍可用，但只会把上游错误文本写入 `detail`。
- 这是 `0.6.1` 当时的历史建议；从 `0.7.0` 起，真实 `StdError` 第一次进入结构化体系优先使用 `into_as(reason, detail)`，普通 source 使用 `with_std_source(...)`，结构化上卷使用 `wrap_as(reason, detail)`。

## [v0.6.0] - 2026-02-22

### 🚨 Breaking Changes
- **`DomainReason` 去掉 `Serialize` 约束**：从 `PartialEq + Display + Serialize` 调整为 `PartialEq + Display`。
- **`serde` 改为可选特性**：默认不启用；需要序列化时请启用 `serde` feature。
- **移除 `UvsXxxFrom` 旧 trait 族**：统一使用 `UvsFrom`。
- **`UvsReason` 结构简化**：除 `ConfigError(ConfErrReason)` 外，其余分类不再携带消息字符串。
  - 例如：`UvsReason::system_error(\"msg\")` -> `UvsReason::system_error()`。
- **`ErrorOwe` 拆分**：`.owe(reason)` 归属 `ErrorOweBase`；`.owe_sys()` 等快捷方法归属 `ErrorOwe`。

### ✨ 新增与优化
- **统一转换接口 `UvsFrom`**：所有 `from_*` 构造收敛到单一 trait。
- **`owe_xxx` 消息去重**：错误消息仅落在 `detail`，不再在 `reason` 中重复存储。
- **新增 `op_context!` 宏**：在调用处展开 `module_path!()`，避免日志模块路径固定在库内部。
- **tracing 实际生效**：`OperationContext` 的 Drop 与日志方法支持 tracing；tracing target 统一为 `domain`。
- **移除 `derive-getters` 依赖**：改为手写 getter，减少依赖体积。
- **`derive_more` 精简**：仅保留 `from` 功能。

### 🧪 验证
- 通过：
  - `cargo test --no-default-features`
  - `cargo test --features tracing --no-default-features`
  - `cargo test --features serde --no-default-features`
  - `cargo test --all-features`

### 迁移提示
- `.owe(...)` 记得引入 `ErrorOweBase`。
- 旧 `UvsReason::*_error(\"...\")` / `UvsReason::core_conf(\"...\")` 改为无参版本。
- 需要序列化时启用：`orion-error = { version = \"0.6\", features = [\"serde\"] }`。

## 版本 0.5.5 (2024-9-20)

### ✨ 新增与优化
- **结构化错误构建器**：`StructError::builder` 支持链式设置 detail、position、context，并与 `context_ref` 共享上下文栈，避免重复分配。
- **上下文作用域 Guard**：新增 `OperationContext::scope()` / `scoped_success()` 与 `OperationScope`，在成功路径自动标记 `mark_suc()`，降低遗漏风险。
- **错误转换性能**：`ErrorOwe::owe_*` 系列仅序列化一次底层错误消息，减少 `to_string()` 开销；上下文内部改用 `Arc<Vec<_>>` 共用堆栈。
- **示例更新**：`examples/order_case.rs`、`logging_example.rs` 演示 builder 与作用域 guard 的推荐写法。

### 📚 文档更新
- `docs/tutorial.md` 增补 builder/OperationScope 的实践示例。
- `docs/error-handling/05-logging-standards.md`、`06-best-practices.md` 补充 guard 与构建器的使用建议。
- `docs/README.md` 在亮点更新中突出上述能力。

### 🧪 兼容性
- 接口向后兼容；若手动调用 `mark_suc()`，仍可与作用域 guard 共存。

## [v0.5.0] - 2025-08-25

### 新增功能
- **日志支持**: 为错误上下文添加了完整的日志记录功能，包括 `info`、`debug`、`warn`、`error`、`trace` 方法
- **自动日志记录**: 新增 `with_exit_log` 和 `mark_suc` 方法，支持在对象销毁时自动记录日志
- **PathContext包装器**: 添加了 `PathContext<V: AsRef<Path>` 包装类型，用于区分路径和字符串类型

### 重大变更
- **结构体重命名**: 将内部结构体从 `WithContext` 重命名为 `OperationContext`，提高代码清晰度
- **ContextTake trait重构**: 解决了 trait 实现冲突问题，移除了 `&PathBuf` 的特定实现，改为使用 `PathContext` 包装器

### 依赖更新
- 升级 `thiserror` 从 2.0.12 到 2.0.16
- 升级 `serde_json` 从 1.0.140 到 1.0.143
- 添加 `log` 和 `env_logger` 依赖以支持日志功能

### 文档和示例
- 新增 `LOGGING.md` 文档，详细说明日志功能的使用方法
- 添加 `examples/logging_example.rs` 示例文件
- 更新现有示例以适配新的 API
- 完善了错误处理的最佳实践文档

### 测试改进
- 新增了完整的 `ContextRecord` trait 测试用例，覆盖字符串类型、数字类型、路径类型等各种场景
- 改进了测试覆盖率，确保所有新功能的稳定性

### CI/CD 优化
- 升级 GitHub Actions 从 `actions/checkout@v4` 到 `v5`
- 升级 `rustsec/audit-check` 从 1.4.1 到 2.0.0
- 改进了 CI 流程中的覆盖率处理

### 错误修复
- 修复了 ContextTake trait 的实现冲突问题 (E0119)
- 修复了路径类型处理的编译错误
- 改进了错误消息的格式化和显示

## [v0.4.0] - 2025-08-24

### 初始发布
- 基础错误处理框架
- 支持结构化错误类型
- 提供错误上下文管理
- 基本的序列化和反序列化支持
