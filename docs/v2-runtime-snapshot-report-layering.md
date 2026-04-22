# V2 Runtime / Snapshot / Report 分层草案

更新时间：2026-04-21

本文档用于冻结 `orion-error 0.7.x / V2` 第一阶段里
`runtime / snapshot / report` 的职责分层草案。

这份文档的目标不是立刻重写实现，而是先回答：

- `StructError<R>` 现在到底承担了哪些不该混在一起的职责
- 哪些职责在 V2 第一阶段可以先用“边界约束”锁住
- 后续代码改造应该按什么顺序推进

## 1. 当前问题

目前的 `StructError<R>` 同时承担了至少四类职责：

1. runtime carrier
2. source bridge carrier
3. snapshot/export object
4. report/render entry

从现状看：

- runtime 传播依赖它持有 `reason`、`detail`、`context`、`source`
- source 结构化保留依赖它暴露 `source_frames()`
- `serde` 直接序列化 `StructError`
- `report()` / `report_redacted()` / `render()` / `render_redacted()` 也挂在 `StructError` 上

这会带来三个直接问题：

### 1.1 runtime carrier 被导出需求反向牵制

一旦 `StructError<R>` 继续承担稳定导出对象职责，后续内部字段、trait 约束、source 存储模型都会更难调整。

### 1.2 snapshot 与 report 混在一起

当前 `ErrorReport` 同时承担：

- 稳定导出视图
- redaction 输入
- 文本渲染输入

这比直接把所有事都塞在 `StructError` 里更好，但还没有把
“稳定 snapshot” 和 “展示 report” 的边界彻底讲清楚。

### 1.3 评审标准不稳定

如果不先冻结分层边界，后面每次改 `StructError` 都会重新掉回：

- 这个字段是不是要 serde
- 这个方法是不是该挂在 `StructError`
- redaction 是不是应该改原错误
- snapshot 到底是不是 report

V2 不能继续在这些问题上反复横跳。

## 2. V2 第一阶段的结论

V2 第一阶段先冻结以下三层语义：

### 2.1 Runtime 层

对象：

- `StructError<R>`

职责：

- 运行时传播
- reason 边界表达
- context/source 聚合
- StdError bridge
- 跨层转换与 wrap

约束：

- 优先服务运行时传播语义
- 不把“稳定导出格式”当成它的核心职责
- 允许短期内继续保留现有导出入口，但这些入口应被视为过渡层

### 2.2 Snapshot 层

对象：

- `StructErrorSnapshot`，V2 第一阶段先定义目标，不要求立即公开实现

职责：

- 稳定导出
- 测试断言
- 机器可读快照
- 为后续 tagged / untagged JSON 预留稳定载体

约束：

- 不保留 runtime error object 能力
- 不要求保留 `StdError` 语义
- 字段设计优先稳定性，而不是运行时便利性

### 2.3 Report 层

对象：

- `ErrorReport`

职责：

- 人类可读渲染
- redaction
- 日志/诊断导出视图

约束：

- report 是展示层视图，不是 runtime carrier
- redaction 只作用于 report 输出，不改变原始 `StructError`
- report 可以从 runtime 或 snapshot 派生，但不能反向定义 runtime

## 3. 当前代码如何映射到三层

按当前实现，可先把现有能力映射如下：

### 3.1 运行时字段

当前属于 runtime carrier 的核心信息：

- `reason`
- `detail`
- `position`
- `context`
- `source`

这些都仍然属于 `StructError<R>` 的正当职责。

### 3.2 过渡期附带字段

当前挂在 runtime 上，但更接近 snapshot/report 的信息：

- `source_frames`
- `serde` 序列化导出
- `report()` / `report_redacted()`
- `render()` / `render_redacted()`

V2 第一阶段不立刻删除它们，但要明确：

- 这些能力是“暂挂在 runtime 上的过渡入口”
- 不是未来继续膨胀 `StructError<R>` 职责的依据

### 3.3 现有 `ErrorReport` 的位置

当前 `ErrorReport` 更接近：

- report layer view model

它已经不再是 runtime 本体，这一步是对的。

但 V2 第一阶段需要进一步锁住：

- `ErrorReport` 继续用于 render/redaction
- 它不等于未来的稳定 snapshot object

## 4. V2 第一阶段的最小设计决策

这一阶段只锁以下决策，不超前引入大改：

### 4.1 不再给 `StructError<R>` 新增更多导出职责

后续如果再出现：

- tagged JSON
- snapshot test helper
- export schema helper
- report variant formatter

默认不直接堆到 `StructError<R>` 上。

优先顺序应该是：

1. 先判断它属于 snapshot 还是 report
2. 再决定挂到独立对象，还是由 `StructError` 仅提供一个转换入口

### 4.2 `ErrorReport` 继续视为 report 层，不升级为“统一导出真身”

这意味着：

- 允许继续扩展 `ErrorReport` 的 render / redaction 能力
- 但不把它等同为未来稳定快照对象

### 4.3 `StructErrorSnapshot` 已有第一批最小骨架

第一阶段现在已经有一个最小可用的 `StructErrorSnapshot`：

- `StructError::snapshot()`
- `StructError::into_snapshot()`
- `StructErrorSnapshot::report()`
- `StructErrorSnapshot::into_report()`
- `StructError::into_report()`

这一步的意义是：

- runtime 到 report 之间已经有了明确中间层
- `snapshot` 不再只是纯文档记账名词
- 后续稳定导出能力可以围绕同一个对象继续收敛

但当前仍然不能把它描述成最终完成态。

现阶段它只是第一批最小骨架，还没有承诺：

- 稳定导出
- 测试快照
- tagged / snapshot-friendly 输出

当前还需要明确一个现实约束：

- `StructErrorSnapshot` 已经是独立对象
- 当前代码已经开始拆出 snapshot 自己的只读 frame：
  - `SnapshotContextFrame`
  - `SnapshotSourceFrame`
- `SnapshotContextFrame` / `StableSnapshotContextFrame` 已携带 V2 上下文语义：
  - `action`
  - `locator`
- 但 `report()` 仍会把这些 snapshot frame 回投成现有
  `OperationContext` / `SourceFrame` 形状，继续复用当前 report 层协议

当前字段口径也开始分层：

- 稳定 snapshot 字段优先看：
  - `target`
  - `action`
  - `locator`
  - `path`
  - `metadata`
  - `message`
  - `error_code`
  - `reason`
  - `want`
  - `detail`
  - `is_root_cause`
- 兼容投影字段当前仍保留：
  - `fields`
  - `result`
  - `display`
  - `type_name`

当前代码里已经开始体现为两条显式导出入口：

- `snapshot.stable_export()`
  - 面向后续稳定 snapshot schema 收敛
- `snapshot.compat_export()`
  - 面向当前兼容投影和过渡期序列化观察
  - 当前已进入 deprecated migration-only 路径
- `snapshot.to_stable_snapshot_json()`
  - 在 `serde_json` feature 下提供稳定 JSON 输出入口
- `err.into_snapshot()`
  - 消费 runtime carrier，生成 owned snapshot
- `snapshot.into_report()` / `err.into_report()`
  - 消费上游对象，生成 report 层 owned 视图
- `stable.compat_export()` / `stable.report()`
  - 从稳定 snapshot 生成兼容观察或 report 视图
  - 这是有损投影，不还原 runtime source object 或 compat-only 字段
  - 其中 `stable.report()` 是主路径；`stable.compat_export()` 当前已进入 deprecated migration-only 路径
- `STABLE_SNAPSHOT_SCHEMA_VERSION`
  - 当前值为 `orion-error.snapshot.v2`
  - `stable_export()` / `to_stable_snapshot_json()` 输出会携带该版本

稳定 JSON 字段契约见：

- `docs/v2-stable-snapshot-schema.md`

当前默认 `serde Serialize for StructErrorSnapshot` 已切到稳定 schema。
如果仍需要旧 compat 投影，需要显式使用：

- `snapshot.to_compat_snapshot_json()`
- `serde_json::to_value(snapshot.compat_serialize())`

这些 compat snapshot 导出入口当前也都已进入 deprecated migration-only 路径。

也就是说，当前已经完成的是“对象分层主路径可用”，
并且 V2 上下文语义已经能从 runtime 保留到 snapshot/report。
仍未最终冻结的是：旧兼容投影字段的移除窗口，以及是否继续保留
`StructError` 本体上的过渡导出入口。

后续相关设计、review、文档都必须继续使用这个名字来承接这部分能力，
避免以后又把需求重新塞回 `StructError` 或 `ErrorReport`。

## 5. 对现有 API 的影响

V2 第一阶段先不给用户造成大面积破坏，但需要调整解释口径：

### 5.1 `StructError::report(...)`

当前保留。

解释口径调整为：

- 这是从 runtime carrier 到 report layer 的转换入口
- 不代表 report 是 runtime 本体的一部分

### 5.2 `render(...)` / `render_redacted(...)`

当前保留。

解释口径调整为：

- 这是 runtime 上挂的便捷入口
- 底层语义仍然是先生成 report，再做渲染或 redaction

### 5.3 `serde Serialize for StructError<R>`

当前保留。

解释口径调整为：

- 这是 V1/V2 过渡期兼容能力
- 不是 V2 长期分层的最终形态
- 当前已经提供显式名字：`err.compat_serialize()`
- 默认 `Serialize for StructError<R>` 只是转调这层 compat runtime projection

当前开始引入 snapshot 路径后，对外应优先鼓励：

- `err.snapshot()`
- `err.into_snapshot()`
- `err.report()`
- `err.into_report()`

而不是继续直接依赖 `StructError` 本体序列化契约。

## 6. 后续实现顺序

V2 后续代码改造建议按这个顺序推进：

1. 冻结本分层文档
2. 明确 bridge/source payload 草案
3. 在现有 `StructErrorSnapshot` 最小骨架上继续补稳定字段边界
4. 继续收敛 stable snapshot / report 的转换与导出边界
5. 最后再评估 `StructError` 上哪些导出能力可以收缩

顺序不能反过来。

如果先动实现、后补边界，极容易再次出现：

- API 先改了一半
- 文档跟不上
- compat 口径又漂

## 7. 第一阶段完成标准

当以下条件成立时，可以认为 V2 第一阶段的“分层草案”完成：

- 有独立文档冻结 runtime / snapshot / report 边界
- `V2 Development Plan` 引用并承认这份草案
- 文档索引能把读者带到这份草案
- 后续 review 可以据此判断某个新能力该落在哪一层

这一步完成前，不建议直接大改 `StructError` 内部结构。
