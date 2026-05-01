# Stable Snapshot Schema

更新时间：2026-04-23

本文档描述当前稳定 snapshot 导出协议。

## 1. 三层对象

先区分三个对象：

- `StructError<R>`：运行时传播对象
- `ErrorSnapshot`：运行时冻结出的富快照
- `StableErrorSnapshot`：稳定机器导出对象

当前稳定 schema version：

```text
orion-error.snapshot.v3
```

对应常量：

```rust,ignore
orion_error::snapshot::STABLE_SNAPSHOT_SCHEMA_VERSION
```

## 2. 入口

最直接的入口：

```rust,ignore
let snapshot = err.snapshot();
let stable = snapshot.stable_export();
```

等价入口：

```rust,ignore
let stable = err.snapshot().into_stable_export();
let stable = StableErrorSnapshot::from(err.snapshot());
let stable = StableErrorSnapshot::from(&err);
```

当前实现也支持：

```rust,ignore
let snapshot = ErrorSnapshot::from(&err);
let stable = StableErrorSnapshot::from(&err);
```

## 3. `ErrorSnapshot`

`ErrorSnapshot` 是中间层富快照。

字段：

- `reason`
- `detail`
- `position`
- `path`
- `context`
- `root_metadata`
- `source_frames`

它保留的信息比稳定 schema 更多：

- `SnapshotContextFrame.fields`
- `SnapshotContextFrame.result`
- `SnapshotSourceFrame.display`
- `SnapshotSourceFrame.type_name`

这些字段主要用于：

- 调试
- 兼容观察
- 中间层转换

说明：

- `path` 是稳定导出的路径投影
- 当前运行时主语义和新代码文档应优先使用 `doing(...)` / `action`

## 4. `StableErrorSnapshot`

`StableErrorSnapshot` 是稳定机器导出对象。

顶层字段：

- `schema_version`
- `reason`
- `detail`
- `position`
- `path`
- `context`
- `root_metadata`
- `source_frames`

## 5. Stable Context Shape

`StableErrorSnapshot.context[]` 的稳定导出字段：

- `target`
- `action`
- `locator`
- `path`
- `metadata`

其中：

- `target` 是兼容 root target 投影
- `action` / `locator` / `path` 更接近当前 runtime 主语义的稳定导出

不属于稳定 schema 的 context 字段：

- `fields`
- `result`

它们仍存在于 `SnapshotContextFrame`，但不会进入稳定导出。

## 6. Stable Source Shape

`StableErrorSnapshot.source_frames[]` 的稳定导出字段：

- `index`
- `message`
- `error_code`
- `reason`
- `path`
- `detail`
- `metadata`
- `is_root_cause`

其中 `path` 是稳定导出的路径投影。

不属于稳定 schema 的 source frame 字段：

- `display`
- `type_name`
- `debug`

这些字段只在富快照或 runtime 辅助路径里使用。

## 7. `serde` 与 `serde_json`

### 7.1 `serde`

启用 `serde` feature 后：

- `StableErrorSnapshot` 可序列化
- `ErrorSnapshot` 的 `Serialize` 会直接输出稳定 schema

也就是说：

```rust,ignore
serde_json::to_value(err.snapshot())
```

输出形状和：

```rust,ignore
serde_json::to_value(err.snapshot().stable_export())
```

是一致的。

### 7.2 `serde_json`

启用 `serde_json` feature 后，可以直接生成 JSON 值：

```rust,ignore
err.snapshot().to_stable_snapshot_json()
err.into_snapshot().to_stable_snapshot_json()
```

## 8. 从 stable snapshot 到 report

`StableErrorSnapshot` 可以投影成 `DiagnosticReport`：

```rust,ignore
let report = stable.report();
```

这是有损投影。

当前行为：

- `context.fields` 会为空
- `context.result` 会退回兼容默认值 `Fail`
- `source_frames.display` 会为空
- `source_frames.type_name` 会为空

因此这条路径适合：

- 渲染
- 诊断
- 文本观察

不适合：

- 还原完整 runtime carrier
- 还原原始 source 持有关系

## 9. 设计约束

当前版本规则：

- 增删稳定字段时，需要评估是否升级 `STABLE_SNAPSHOT_SCHEMA_VERSION`
- 富快照非稳定字段变化，不自动触发 stable schema version 变化

## 10. 当前非目标

当前稳定 snapshot 不承诺：

- 从 JSON 反序列化回 `StructError<R>`
- typed roundtrip
- tagged union schema
- 通过 stable JSON 完整还原 runtime source 对象
