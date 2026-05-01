# Orion Error Diagnostic Protocol Evolution

本文档用于重构 `SourceFrame metadata` PR 文档中第 21 节“后续需求与建议”，给出一份边界更清晰、可以独立推进的后续设计。

核心判断只有一条：

> `orion-error` 后续最该补的应是稳定的结构化诊断协议，其次才是安全展示协议；业务 helper 与业务分类不应进入基础错误 crate。

## 1. 设计目标

`SourceFrame.metadata` 已经让 `orion-error` 具备了基础的机器可读诊断载体能力，但后续演进不能继续把“核心协议”“展示层能力”“业务扩展”“API 糖”混在同一个需求列表里。

后续设计应拆成三层：

1. 核心协议层
2. 展示与安全层
3. 业务扩展层

这三层必须分别演进、分别验收。

## 2. 层次划分

### 2.1 核心协议层

属于 `orion-error` 本体，负责稳定的结构化诊断协议。

这一层只回答四个问题：

- 错误对象中哪些结构化字段是稳定的
- metadata 如何记录、传播、合并
- 哪些类型具备稳定的序列化契约
- 上层如何稳定读取 root error 和 source chain 的机器可读信息

这一层不负责：

- 敏感信息清洗
- verbose 文本输出
- CLI hint
- 业务 helper
- 业务分类

### 2.2 展示与安全层

这一层负责：

- redaction
- verbose formatter
- snapshot-friendly 输出
- 可选 tagged JSON 模式

它依赖核心协议层，但不应反过来污染 `StructError` 的基础模型。

### 2.3 业务扩展层

这一层属于业务 crate，例如 `wp-config`、`wp-error`、`wp-motor`。

它负责：

- 业务 metadata key 常量
- typed helper
- 业务分类
- CLI hint 规则

业务扩展层可以建立在 `orion-error` 协议之上，但不应把业务语义塞回 `orion-error`。

## 3. 核心协议层设计

### 3.1 已确定的核心能力

以下能力应视为核心协议层的第一阶段：

1. `StructError::context_metadata()`
2. metadata merge contract
3. `SourceFrame.metadata`
4. serde / schema 契约

### 3.2 `context_metadata()`

root error 自身需要统一 metadata 读取入口，不能只依赖 `source_frames()`。

建议保留如下 API：

```rust,ignore
impl<T: DomainReason> StructError<T> {
    pub fn context_metadata(&self) -> ErrorMetadata;
}
```

语义要求：

- 返回聚合后的 metadata 副本
- 调用方修改返回值不会反写 `StructError`
- 上层可以同时读取：
  - root error 自身 metadata
  - source chain 每一帧 metadata

### 3.3 metadata merge contract

metadata merge contract 必须是统一 helper 和统一测试契约，而不是散落在各 crate 中的约定。

唯一允许的语义：

- 更具体 context 优先
- 外层 context 只补缺
- 已存在 key 不被外层覆盖

建议 helper 形态：

```rust,ignore
impl ErrorMetadata {
    pub(crate) fn merge_missing(&mut self, other: &ErrorMetadata);
}
```

以及：

```rust,ignore
fn merged_context_metadata(contexts: &[OperationContext]) -> ErrorMetadata;
```

### 3.4 serde / schema 契约

这一项是核心协议层的收口点，必须文档化。

建议明确如下契约：

- `OperationContext`: `Serialize + Deserialize`
- `ErrorMetadata`: `Serialize + Deserialize`
- `MetadataValue`: `Serialize + Deserialize`
- `SourceFrame`: 当前保证 `Serialize`
- `StructError`: 当前保证 `Serialize`

同时明确 schema 边界：

- 稳定字段：
  - `reason`
  - `detail`
  - `position`
  - `context`
  - `want`
  - `path`
  - `source_frames`
  - `source_message`
  - `source_chain`
  - `metadata`
- 可扩展字段：
  - 后续新增的可选字段
- 向后兼容要求：
  - 新增字段必须通过 `serde(default)` 兼容旧 payload
  - 空 metadata 必须跳过序列化

### 3.5 不应进入核心协议层的内容

以下内容不应混入本层 roadmap：

- redaction
- verbose formatter
- query helper 泛滥
- 业务分类
- 业务 metadata helper

## 4. 展示与安全层设计

### 4.1 为什么不应直接堆在 `StructError` 上

`Display` 的职责是默认简洁的人类可读输出，不是万能调试界面。

如果未来直接在 `StructError` 上堆：

- `display_chain_verbose()`
- `redact()`
- `to_snapshot_json()`
- `to_tagged_json()`

那么基础错误类型会很快膨胀成 renderer 容器。

更合理的设计是：先定义稳定的 report/view model，再在这个视图上做渲染与清洗。

### 4.2 推荐的 report model

建议引入独立的导出视图：

```rust,ignore
pub struct ErrorReport {
    pub reason: String,
    pub detail: Option<String>,
    pub position: Option<String>,
    pub want: Option<String>,
    pub path: Option<String>,
    pub context: Vec<OperationContext>,
    pub root_metadata: ErrorMetadata,
    pub source_frames: Vec<SourceFrame>,
}
```

建议 API：

```rust,ignore
pub enum RenderMode {
    Compact,
    Verbose,
}

pub trait RedactPolicy {
    fn redact_key(&self, key: &str) -> bool;
    fn redact_value(&self, key: Option<&str>, value: &str) -> Option<String>;
}

impl<T: DomainReason> StructError<T> {
    pub fn report(&self) -> ErrorReport;
    pub fn render(&self, mode: RenderMode) -> String;
    pub fn render_redacted(&self, mode: RenderMode, policy: &impl RedactPolicy) -> String;
}
```

### 4.3 redaction

`detail`、`context`、`metadata` 都可能携带敏感信息，redaction 是合理需求，但它属于展示与导出层，而不是基础错误协议层。

建议目标：

- redaction 不改变原始 `StructError`
- redaction 只作用于 report/render 输出
- policy 由上层注入

### 4.4 verbose formatter

verbose formatter 也是合理需求，但应建立在 `ErrorReport` 之上。

最低建议：

```rust,ignore
err.render(RenderMode::Verbose)
```

不建议直接在默认 `Display` 上扩展逻辑。

### 4.5 tagged JSON 与 snapshot-friendly 输出

这两项也属于导出层：

- `untagged` 继续保留为默认 JSON 模式
- 如果未来需要严格类型 roundtrip，再引入 tagged JSON
- snapshot-friendly 输出应是稳定的导出策略，而不是普通 `Display`

## 5. 业务扩展层设计

### 5.1 typed helper

typed helper 是合理需求，但不属于 `orion-error` 本体。

错误做法：

```rust,ignore
ctx.wp_config_kind(WpConfigKind::SinkDefaults)
```

把这类 API 放进基础 crate，会把业务语义带入通用错误库。

正确做法是由业务 crate 自己定义扩展 trait：

```rust,ignore
pub trait WpConfigMetadataExt {
    fn wp_config_kind(self, kind: WpConfigKind) -> Self;
    fn wp_config_scope(self, scope: WpConfigScope) -> Self;
}
```

### 5.2 业务分类

业务分类也不应进入 `orion-error`。

以下能力应继续留在业务侧：

- `sink_defaults` / `wpsrc` / `sink_route` 等业务 kind
- CLI hint 规则
- 文件角色判断
- WP metadata key 常量

### 5.3 `UvsReason` 范围内的通用辅助

如果确实需要“基础分类辅助”，建议把范围收窄到 `UvsReason` 这一类已经稳定的通用 taxonomy，而不是对所有 `DomainReason` 做推断。

例如：

```rust,ignore
impl UvsReason {
    pub fn is_config(&self) -> bool;
    pub fn is_validation(&self) -> bool;
    pub fn is_resource(&self) -> bool;
    pub fn is_timeout(&self) -> bool;
}
```

这属于“通用 reason taxonomy 辅助”，不是“任意错误分类”。

## 6. 关于查询与 helper API 的约束

`first_path()`、`first_want()`、`find_meta()`、frame filter 等能力不是不能做，但不应在使用模式尚未稳定时大面积铺开。

更合理的策略是：

- 先观察真实诊断代码的重复模式
- 只收敛最小、最稳定的一组 helper

如果必须提供，建议最小化为：

```rust,ignore
impl<T: DomainReason> StructError<T> {
    pub fn root_cause_frame(&self) -> Option<&SourceFrame>;
    pub fn first_frame_with_meta(&self, key: &str) -> Option<&SourceFrame>;
}
```

不建议在当前阶段引入大量：

- `first_path()`
- `first_want()`
- `find_meta()`
- `frames_with_path()`
- `frames_with_metadata()`
- 其他组合型 filter API

## 7. 修正后的优先级

### 7.1 P0：核心协议收口

1. `context_metadata()`
2. metadata merge contract
3. serde / schema 契约文档化

### 7.2 P1：展示与安全视图

4. `ErrorReport` 视图模型
5. redaction
6. verbose formatter

### 7.3 P2：导出策略

7. snapshot-friendly 输出
8. tagged JSON 模式

### 7.4 P3：谨慎引入最小 helper

9. 最小 query helper
10. 最小 builder/helper 一致化

### 7.5 Out of Scope

11. typed helper 扩展点
12. 业务分类
13. CLI hint 规则
14. 业务 metadata key 常量

这些全部放到业务 crate。

## 8. 推荐推进顺序

建议按如下顺序演进：

1. `context_metadata()` 与 merge contract
2. schema / serde 契约
3. `ErrorReport` 视图模型
4. redaction 与 verbose renderer
5. snapshot / tagged JSON 输出
6. 基于真实使用模式补最小 query helper

## 9. 结论

第 21 节原始思路的方向是对的，但问题在于把四类不同性质的需求混进了一个 roadmap：

- 核心协议
- 展示安全
- 业务扩展
- API 糖

正确设计应当是：

- `orion-error` 负责稳定的结构化诊断协议
- report/render 层负责安全展示协议
- 业务 crate 负责 typed helper 与业务分类

一句话总结：

> `orion-error` 后续最该补的不是更多错误文本能力，也不是更多业务 helper，而是稳定的结构化诊断协议，以及建立在其上的安全展示协议。
