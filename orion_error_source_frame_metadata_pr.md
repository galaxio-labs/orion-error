# Orion Error SourceFrame Metadata PR 设计

本文档整理一个面向 `orion-error` 的独立 PR 需求与设计方案。目标是为 `SourceFrame` 增加机器可读的 typed metadata，使上层诊断分类不再依赖 `Display` 文本、文件名或错误字符串启发式匹配。

## 1. 背景

`orion-error 0.6` 已经提供结构化错误链能力，`SourceFrame` 当前包含：

```rust,ignore
pub struct SourceFrame {
    pub index: usize,
    pub message: String,
    pub display: Option<String>,
    pub debug: String,
    pub type_name: Option<String>,
    pub error_code: Option<i32>,
    pub reason: Option<String>,
    pub want: Option<String>,
    pub path: Option<String>,
    pub detail: Option<String>,
    pub is_root_cause: bool,
}
```

这些字段可以表达错误链中的 reason、operation、path、detail，但不能稳定表达业务诊断分类。

例如 CLI 想区分以下配置错误：

- `topology/sources/wpsrc.toml` 解析失败
- `topology/sinks/defaults.toml` 解析失败
- `topology/sinks/infra.d/*.toml` route 解析失败
- `topology/sinks/business.d/*.toml` route 解析失败

目前上层只能基于字符串判断：

```rust,ignore
contains("defaults.toml")
contains("load sink defaults")
contains("expected `defaults`")
contains("wpsrc.toml")
```

这会带来两个问题：

- 错误展示层与 `Display` 输出格式强耦合。
- hint 分类容易误判，例如把 `sinks/defaults.toml` 的错误误提示为 sink route 或 `wpsrc.toml`。

## 2. PR 目标

本 PR 的目标是给 `orion-error` 增加通用 metadata 承载能力：

- `OperationContext` 可以记录机器可读 metadata。
- `StructError` 生成 `SourceFrame` 时携带合并后的 metadata。
- `SourceFrame` 对外暴露 metadata，供上层分类器使用。
- 默认 `Display` 不打印 metadata，避免 CLI 输出变长。
- `serde` 下空 metadata 不输出，保持向后兼容。
- `orion-error` 不引入任何 WP 领域概念。

一句话概括：

> `orion-error` 只提供结构化 metadata 协议，业务 crate 决定 metadata key 和 value。

## 3. 非目标

本 PR 不负责：

- 定义 WP 领域枚举，如 `sink_defaults`、`sink_route`、`wpsrc`。
- 修改 `wp-motor` CLI 展示逻辑。
- 替代 `reason`、`detail`、`source` 或 `context`。
- 默认在错误 `Display` 中打印 metadata。
- 从普通 `StdError` 的文本中反向解析 metadata。

## 4. 核心类型

建议新增通用 metadata 类型：

```rust,ignore
pub struct ErrorMetadata {
    fields: BTreeMap<String, MetadataValue>,
}

pub enum MetadataValue {
    String(String),
    Bool(bool),
    I64(i64),
    U64(u64),
}
```

不建议只暴露裸 `BTreeMap<String, String>` 作为唯一 API。内部可以使用 map，但外部应提供 typed value，避免后续需要表达行号、列号、布尔开关时继续字符串化。

## 5. ErrorMetadata API

建议提供最小 API：

```rust,ignore
impl ErrorMetadata {
    pub fn new() -> Self;

    pub fn is_empty(&self) -> bool;

    pub fn as_map(&self) -> &BTreeMap<String, MetadataValue>;

    pub fn insert<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<MetadataValue>;

    pub fn get(&self, key: &str) -> Option<&MetadataValue>;

    pub fn get_str(&self, key: &str) -> Option<&str>;

    pub fn iter(&self) -> impl Iterator<Item = (&String, &MetadataValue)>;
}
```

`as_map()` 用于测试、调试和少量需要整体只读视图的调用方。它只返回不可变引用，不暴露可变 map，避免绕过 key 校验。

`insert` / `record_meta` / `with_meta` 必须禁止空 key。这里的“禁止”不是指返回错误或 panic，而是定义为以下固定契约，避免错误构造路径引入新的失败分支：

- 空 key 不写入 metadata。
- debug build 触发 `debug_assert!`，便于开发期发现。
- 文档明确空 key 非法，调用方不得依赖空 key 行为。

`MetadataValue` 建议提供常用转换：

```rust,ignore
impl From<String> for MetadataValue;
impl From<&str> for MetadataValue;
impl From<bool> for MetadataValue;
impl From<i64> for MetadataValue;
impl From<i32> for MetadataValue;
impl From<u64> for MetadataValue;
impl From<u32> for MetadataValue;
impl From<usize> for MetadataValue;
```

`MetadataValue` 不提供浮点类型。诊断 metadata 应保持短小、稳定、可比较；浮点值容易引入精度和展示差异。如果确实需要表达比例或耗时，建议由业务层先格式化为字符串，或使用整数单位，例如 `duration.ms = 120u64`。

## 6. OperationContext 扩展

`OperationContext` 增加字段：

```rust,ignore
metadata: ErrorMetadata
```

建议新增 API：

```rust,ignore
impl OperationContext {
    pub fn metadata(&self) -> &ErrorMetadata;

    pub fn record_meta<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<MetadataValue>;

    pub fn with_meta<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<MetadataValue>;
}
```

`record_meta` / `with_meta` 同样必须遵守“禁止空 key”的规则。

使用示例：

```rust,ignore
let ctx = OperationContext::want("load sink defaults")
    .with_meta("config.kind", "sink_defaults")
    .with_meta("config.scope", "sink")
    .with_meta("config.format", "toml")
    .with_meta("file.path", path.display().to_string());
```

## 7. SourceFrame 扩展

`SourceFrame` 增加字段：

```rust,ignore
pub metadata: ErrorMetadata
```

serde 处理建议：

```rust,ignore
#[cfg_attr(feature = "serde", serde(default))]
#[cfg_attr(feature = "serde", serde(skip_serializing_if = "ErrorMetadata::is_empty"))]
pub metadata: ErrorMetadata,
```

这样旧调用方不使用 metadata 时行为保持不变，序列化输出不会多出空字段。

本 PR 要求 `ErrorMetadata` / `MetadataValue` 在 `serde` feature 下同时支持 `Serialize` 与 `Deserialize`。`SourceFrame` 当前是否支持 `Deserialize` 不作为本 PR 的强制目标；如果 `SourceFrame` 后续增加 `Deserialize`，`metadata` 字段必须使用 `serde(default)` 维持向后兼容。

## 8. Metadata 合并规则

这是 PR 中最重要的行为，需要写入测试。

一个错误可能有多层 `OperationContext`：

```text
context 0: load object from toml file with env
context 1: load sink defaults
context 2: load infra sink routes for clean
```

建议 `SourceFrame.metadata` 采用“更具体 context 优先，外层 context 只补缺”的合并策略。

原则：

- 最靠近 root cause / loader 的 context 优先。
- 如果 key 已存在，外层 context 不覆盖。
- 外层 context 可以补充更高层语义，例如 `config.group=infra`。
- `file.path`、`config.kind` 等更具体信息不应被外层操作覆盖。

示例：

```text
inner:
  config.kind = sink_defaults
  file.path = /.../topology/sinks/defaults.toml

outer:
  config.kind = sink_route
  config.group = infra

merged:
  config.kind = sink_defaults
  file.path = /.../topology/sinks/defaults.toml
  config.group = infra
```

伪代码：

```rust,ignore
fn merged_metadata(contexts: &[OperationContext]) -> ErrorMetadata {
    let mut merged = ErrorMetadata::new();

    for ctx in contexts.iter().from_inner_to_outer() {
        merged.merge_missing(ctx.metadata());
    }

    merged
}
```

实际遍历顺序需要结合 `orion-error` 当前 context stack 的存储顺序确定。PR 必须新增内部 helper，例如：

```rust,ignore
impl<T: DomainReason> StructError<T> {
    pub fn context_metadata(&self) -> ErrorMetadata;
}
```

该 helper 是合并规则的唯一实现入口，`SourceFrame` 收集、诊断分类和测试都应复用它。PR 可以不在文档中假设当前 context vec 的物理顺序，但必须用 `.with(...)` / `.want(...)` 的真实调用行为写测试，锁定“具体 context 优先，外层 context 只补缺”的最终语义。

## 9. 普通 StdError 的行为

普通 `StdError` 没有 metadata，因此收集 source frames 时使用空 metadata：

```rust,ignore
SourceFrame {
    metadata: ErrorMetadata::default(),
    ...
}
```

不要从 `Display`、`Debug` 或错误字符串中反向解析 metadata。

## 10. StructError Source 的行为

当 source 是另一个 `StructError` 时，root source frame 应携带该 source 自身 context 合并得到的 metadata。

示意：

```rust,ignore
frames.push(SourceFrame {
    message: source.reason().to_string(),
    want: source.target_main(),
    path: source.target_path(),
    detail: source.detail().clone(),
    metadata: source.merged_metadata(),
    ...
});
```

已有 `source.source_frames()` 应继续 clone，并调整 `index`。clone 过程中不能丢失原 frame 的 metadata。

同时需要区分两个层次：

- root error 自身的 metadata：通过 `err.context_metadata()` 读取。
- source chain 的 metadata：通过 `err.source_frames()[n].metadata` 读取。

上层分类器不应只看 `source_frames()`；对于 wrapper 层补充的 operation / component metadata，应同时读取 root error 的 `context_metadata()`。

## 11. Display 策略

默认 `Display for StructError` 不打印 metadata。

原因：

- metadata 是给分类器和机器消费者使用的。
- 默认 CLI 错误输出应保持简洁。
- `reason/detail/source/context` 已经足够支持人类阅读。

如果后续需要展示 metadata，应在 debug 或 verbose renderer 中显式输出，而不是改变默认 `Display`。

## 12. Serde 策略

在 `serde` feature 下：

- `ErrorMetadata` 支持 serialize / deserialize。
- `MetadataValue` 使用 `#[serde(untagged)]`，让 JSON 更自然。
- 空 metadata 不输出。

`#[serde(untagged)]` 优先保证 JSON 可读性，不保证 `I64` / `U64` 在反序列化后保持原 variant。诊断 metadata 当前只要求值可读、可比较、可用于分类；如果未来需要严格数值类型 roundtrip，再引入 tagged metadata encoding。

期望 JSON：

```json
{
  "metadata": {
    "config.kind": "sink_defaults",
    "config.scope": "sink",
    "parse.line": 1,
    "parse.column": 1
  }
}
```

不建议输出成：

```json
{
  "metadata": {
    "config.kind": {
      "String": "sink_defaults"
    }
  }
}
```

## 13. Re-export

建议从 crate root 暴露：

```rust,ignore
pub use core::{ErrorMetadata, MetadataValue};
```

并加入：

```rust,ignore
pub mod prelude {
    pub use crate::{ErrorMetadata, MetadataValue};
}

pub mod types {
    pub use crate::{ErrorMetadata, MetadataValue};
}
```

## 14. 推荐 Metadata Key 规范

`orion-error` 不强制定义业务 key，但 PR 文档应给出推荐 namespace，避免生态中出现 `kind`、`config_kind`、`config.kind` 混用。

配置类：

```text
config.kind
config.scope
config.group
config.file_role
config.format
```

文件类：

```text
file.path
file.name
file.ext
```

解析类：

```text
parse.format
parse.line
parse.column
parse.expected
parse.found
```

运行时类：

```text
error.domain
component.name
operation.name
```

连接器类：

```text
connector.id
connector.scope
connector.kind
```

## 15. 测试要求

### 15.1 OperationContext metadata

覆盖：

- `with_meta` 可以构造 metadata。
- `record_meta` 可以追加 metadata。
- string / bool / integer value 都能保存。
- `as_map()` 可以返回只读 map 视图。
- 空 key 不会写入 metadata，并在 debug build 中暴露为开发期问题。
- 不支持浮点 metadata value。

示例：

```rust,ignore
let ctx = OperationContext::want("load")
    .with_meta("config.kind", "wpsrc")
    .with_meta("parse.line", 1u32);

assert_eq!(ctx.metadata().get_str("config.kind"), Some("wpsrc"));
assert!(ctx.metadata().as_map().contains_key("parse.line"));
```

### 15.2 StructError source frame 携带 metadata

覆盖：

- source 是 `StructError` 时，`source_frames()[0].metadata` 包含 source context metadata。

示例：

```rust,ignore
let ctx = OperationContext::want("load sink defaults")
    .with_meta("config.kind", "sink_defaults");

let source = StructError::from(TestReason::Config).with(ctx);
let err = StructError::from(TestReason::Runtime).with_source(source);

assert_eq!(
    err.source_frames()[0].metadata.get_str("config.kind"),
    Some("sink_defaults")
);
```

### 15.3 wrap 后不丢 metadata

覆盖：

- `with_source(StructError)` 不丢 metadata。
- `wrap()` / `with_struct_error_source()` 不丢 metadata。
- clone source frames 时 index 调整后 metadata 仍保留。

### 15.4 多层 context 合并

覆盖“具体 context 优先，外层补缺”：

```text
inner: config.kind = sink_defaults
outer: config.kind = sink_route
outer: config.group = infra
```

期望：

```text
config.kind = sink_defaults
config.group = infra
```

### 15.5 serde

覆盖：

- 空 metadata 不输出。
- 非空 metadata 输出为自然 JSON。
- `MetadataValue` 的 string / bool / integer 序列化符合预期。

### 15.6 Display

覆盖：

- 默认 `Display` 不包含 metadata key。
- 默认 `Display` 不包含 metadata value。

## 16. 后续 WP 侧消费方式

该 PR 合并后，`wp-motor` 可以从文本启发式：

```rust,ignore
let hints = collect_hints(&e.display_chain());
```

逐步迁移为 metadata 优先：

```rust,ignore
let kind = classify_by_source_metadata(e.source_frames());
let hints = collect_hints_for_kind(kind);
```

分类示例：

```rust,ignore
match frame.metadata.get_str("config.kind") {
    Some("sink_defaults") => DiagnosticKind::SinkDefaultsToml,
    Some("sink_route") => DiagnosticKind::SinkRouteToml,
    Some("wpsrc") => DiagnosticKind::WpSrcToml,
    _ => DiagnosticKind::Unknown,
}
```

旧文本匹配可以保留为 fallback，等 `wp-config` loader 全面补齐 metadata 后再收敛。

分类器应同时读取两类 metadata：

- `err.context_metadata()`：当前 wrapper/root error 自身的上下文 metadata。
- `err.source_frames()`：上游 source chain 每一帧携带的 metadata。

这样可以同时利用外层 operation/component 信息和内层 root cause/config kind 信息。

## 17. 迁移计划

推荐分阶段推进：

1. `orion-error` 增加 metadata 基础能力。
2. `wp-config` loader 在关键入口补 metadata。
3. `wp-motor` diagnostics 优先使用 metadata 分类。
4. 保留旧字符串 hint 作为 fallback。
5. 基于真实 `wp-example` 错误样例验证分类准确性。
6. 逐步收缩旧字符串启发式判断。

优先补 metadata 的配置入口：

- engine config
- `wpsrc.toml`
- `sinks/defaults.toml`
- `sinks/infra.d/*.toml`
- `sinks/business.d/*.toml`
- connector definition
- wpgen config

## 18. 风险与约束

主要风险：

- metadata key 没有规范，导致生态混乱。
- metadata 被拿来塞长文本，退化成另一个 `detail`。
- 空 metadata key 进入错误链，导致分类器无法建立稳定约定。
- 外层 context 覆盖内层具体 metadata，导致分类仍然错误。
- `orion-error` 引入 WP 领域概念，破坏通用性。
- 默认 `Display` 打印 metadata，导致 CLI 输出变长。

约束：

- `orion-error` 只提供通用容器和传播机制。
- 领域 key 应由领域 crate 通过常量或 helper 统一定义。
- metadata 应短小、稳定、机器可读。
- metadata key 禁止为空。
- metadata value 不支持浮点。
- metadata 不替代 `source`，也不替代 `detail`。
- metadata 不默认展示。

## 19. PR 描述草稿

```markdown
## Summary

Add typed diagnostic metadata to `OperationContext` and `SourceFrame`.

This allows downstream crates to classify structured errors using stable machine-readable fields instead of parsing `Display` output or matching file names.

## Motivation

`SourceFrame` already exposes reason/want/path/detail, but it cannot represent domain-neutral diagnostic attributes such as `config.kind`, `config.scope`, `parse.line`, or `file.path`.

Downstream CLI renderers currently need fragile string heuristics to distinguish errors like source config parse failures, sink defaults parse failures, and sink route parse failures.

## Design

- Add `ErrorMetadata` as a small typed map.
- Add `MetadataValue` for string/bool/signed/unsigned values.
- Add metadata storage and builder methods to `OperationContext`.
- Add `metadata` to `SourceFrame`.
- Propagate metadata when collecting frames from `StructError` sources.
- Keep metadata out of default `Display`.
- Serialize non-empty metadata under the `serde` feature.

## Compatibility

Existing code does not need changes. Empty metadata is omitted during serialization. Default display output is unchanged.
```

## 20. 建议结论

建议将该能力作为 `orion-error` 的独立 PR 推进，并保持边界清晰：

- `orion-error`：通用 metadata 协议。
- `wp-config` / `wp-error`：领域 key 常量和 helper。
- `wp-motor`：基于 metadata 的诊断分类和 hint 输出。

这能把 CLI 错误提示从“文本猜测”推进到“结构化分类”，同时避免把 WP 业务概念污染到基础错误 crate。

## 21. 后续需求与建议

### 21.0 当前实现状态

截至当前实现，本节需求状态如下：

- 已完成：
  - `21.1.1 StructError::context_metadata()`
  - `21.1.2 metadata merge contract 固化`
- 部分完成：
  - `21.1.3 serde / schema 契约文档化`
  - `21.3.13 文档与 examples 补齐`
- 未完成：
  - `21.1.4 redaction 能力`
  - `21.1.5 verbose formatter`
  - `21.2.6 typed helper 扩展点`
  - `21.2.7 source frame filter / query API`
  - `21.2.8 context/source 查询辅助`
  - `21.2.9 通用错误分类辅助`
  - `21.2.10 builder / helper 一致化`
  - `21.3.11 tagged JSON 输出模式`
  - `21.3.12 snapshot-friendly 输出`

更具体地说：

- `context_metadata()` 已经提供 root error 自身 metadata 的统一读取入口。
- metadata merge contract 已沉淀为统一 helper 与测试契约，当前行为是“内层优先，外层补缺”。
- serde 相关代码契约已部分落地，但仍缺系统化 schema 文档，尚未清晰定义哪些字段属于稳定协议、哪些字段允许扩展。
- README 与 examples 已补 metadata、`with_struct_source()` / `source_struct()`、root/source metadata 读取示例；但 `verbose formatter` 与 `redaction` 的文档和示例仍未补齐，因为功能本身尚未实现。

如果按第 21 节的推荐推进顺序验收，当前进度可以概括为：

> 第一阶段的核心能力已落地；第二阶段开始部分推进；第三阶段及之后的大多数建议项尚未启动。

`SourceFrame metadata` 只是 `orion-error` 诊断能力演进的第一步。除了本 PR，本工程还建议 `orion-error` 后续按优先级补充以下能力。

### 21.1 高优先级

#### 1. `StructError::context_metadata()`

root error 自身需要统一 metadata 读取入口，不能只依赖 `source_frames()`。

建议：

```rust,ignore
impl<T: DomainReason> StructError<T> {
    pub fn context_metadata(&self) -> ErrorMetadata;
}
```

这样上层诊断分类器可以同时读取：

- root error 自身 metadata
- source chain 每一帧 metadata

#### 2. metadata merge contract 固化

“具体 context 优先，外层 context 补缺”不应只停留在文档中，应沉淀为统一 helper 和测试契约，避免各 crate 自己实现一套 merge 逻辑。

#### 3. serde / schema 契约文档化

需要明确：

- 哪些类型保证 `Serialize`
- 哪些类型保证 `Deserialize`
- 哪些字段是稳定 schema
- 哪些字段允许后续扩展

否则下游 JSON 消费方会在没有显式协议的情况下与实现细节耦合。

#### 4. redaction 能力

未来 `detail`、`context`、`metadata` 中都可能携带：

- token
- password
- key path
- endpoint
- 请求头

建议 `orion-error` 预留统一 redaction 能力，例如：

```rust,ignore
pub trait RedactPolicy { ... }
pub fn redact(&self, policy: &impl RedactPolicy) -> RedactedStructError<_>;
```

至少要有明确演进方向，避免每个上层项目自己做一套敏感信息清洗。

#### 5. verbose formatter

默认 `Display` 应保持简洁，但需要提供“结构化详细输出”能力，避免每个项目自己拼装 debug 输出。

例如：

```rust,ignore
err.display_chain_verbose()
err.render(RenderMode::Verbose)
```

### 21.2 中优先级

#### 6. typed helper 扩展点

`orion-error` 本身应保持通用，但应允许业务 crate 低成本封装领域 helper，减少散落的字符串 key。

例如业务侧可以基于统一扩展 trait 写：

```rust,ignore
ctx.wp_config_kind(WpConfigKind::SinkDefaults)
```

而不是全仓散落：

```rust,ignore
ctx.with_meta("config.kind", "sink_defaults")
```

#### 7. source frame filter / query API

建议增加辅助接口，减少上层重复遍历和模式化筛选。例如：

- 只取 root cause frame
- 只取有 path 的 frame
- 只取带 metadata 的 frame
- 查找第一个匹配 metadata key 的 frame

这类能力适合收敛到 `orion-error`，避免每个项目手写遍历。

#### 8. context/source 查询辅助

例如：

```rust,ignore
err.first_path()
err.first_want()
err.find_meta("config.kind")
```

这些辅助方法不引入业务语义，但能显著降低诊断层样板代码。

#### 9. 通用错误分类辅助

不是业务分类，而是基于 `reason` / `error_code` 的稳定通用能力，例如：

- 是否配置错误
- 是否校验错误
- 是否资源错误
- 是否超时错误

这样上层可以在不解析文本的前提下做基础分支判断。

#### 10. builder / helper 一致化

当前 `to_err()`、`with_detail()`、`with_source()`、`with(...)`、`want(...)` 的组合能力较强，但实际工程中仍容易出现多种手写包装风格。

建议后续进一步统一 builder / helper 体验，减少“功能都有但写法不统一”的情况。

### 21.3 低优先级

#### 11. tagged JSON 输出模式

当前 `MetadataValue` 使用 `untagged`，优先照顾 JSON 可读性。如果未来需要严格类型 roundtrip，可以增加 tagged 模式作为可选输出策略，而不必立即改变默认 schema。

#### 12. snapshot-friendly 输出

建议提供更稳定、适合测试断言的文本或 JSON 输出格式，减少 snapshot test 因上下文顺序、显示格式小变化而脆弱。

#### 13. 文档与 examples 补齐

建议后续补完整示例，覆盖：

- metadata
- wrap / with_source
- serde 输出
- verbose formatter
- redaction

### 21.4 不建议放入 `orion-error` 的内容

以下内容不建议进入 `orion-error`：

- 业务枚举，如 `sink_defaults`、`wpsrc`、`sink_route`
- CLI hint 规则
- 文件类型/配置类型判断逻辑
- WP 专属 metadata key 常量

这些应由业务 crate 负责。

### 21.5 推荐推进顺序

建议后续按如下顺序演进：

1. `context_metadata()` 与 metadata merge contract
2. serde / schema 契约文档化
3. redaction 与 verbose formatter
4. typed helper 扩展点
5. 查询 / 过滤辅助 API

一句话总结：

> `orion-error` 后续最该补的不是更多“错误文本能力”，而是更稳定的“结构化诊断协议”和“安全展示协议”。
