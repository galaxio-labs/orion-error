# 更新日志 (CHANGELOG)

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
- 如果你需要保留真实底层错误链，请优先使用 `.owe_*_source()`、`with_source(...)` 或 `err_wrap(...)`。

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
