# Stable Snapshot Schema

更新时间：2026-04-22

本文档描述当前稳定 snapshot JSON 协议。

当前稳定协议版本：

```text
orion-error.snapshot.v2
```

代码常量：

```rust,ignore
orion_error::STABLE_SNAPSHOT_SCHEMA_VERSION
```

## 1. 入口

稳定 snapshot JSON 只从以下入口产生：

```rust,ignore
err.snapshot().to_stable_snapshot_json()
err.into_snapshot().to_stable_snapshot_json()
err.snapshot().into_stable_export()
```

或等价地：

```rust,ignore
serde_json::to_value(err.snapshot().stable_export())
serde_json::to_value(err.into_snapshot().stable_export())
serde_json::to_value(StableErrorSnapshot::from(err.snapshot()))
serde_json::to_value(StableErrorSnapshot::from(&err))
```

`ErrorSnapshot` 自身的默认 `serde Serialize` 现在直接输出稳定 schema。

稳定 snapshot 可以转成 report：

```rust,ignore
let report = stable.report();
```

这是从稳定导出对象到报告对象的有损投影：

- `context.fields` 会为空
- `context.result` 会使用兼容默认值 `Fail`
- `source_frames.display` 会为空
- `source_frames.type_name` 会为空

它只用于渲染、诊断和兼容观察，不表示可以从稳定 snapshot 还原 runtime
`StructError<_>`。

## 2. 顶层字段

`StableErrorSnapshot` 当前字段：

- `schema_version`
- `reason`
- `detail`
- `position`
- `want`
- `path`
- `context`
- `root_metadata`
- `source_frames`

## 3. Context Frame 字段

`StableSnapshotContextFrame` 当前字段：

- `target`
- `action`
- `locator`
- `path`
- `metadata`

以下字段不属于稳定 context schema：

- `fields`
- `result`

这些字段不再通过 snapshot 的兼容投影入口导出；如果需要 `0.6.3` runtime JSON 形状，使用 `StructError` 默认 serde 输出。

## 4. Source Frame 字段

`StableSnapshotSourceFrame` 当前字段：

- `index`
- `message`
- `error_code`
- `reason`
- `want`
- `path`
- `detail`
- `metadata`
- `is_root_cause`

以下字段不属于稳定 source frame schema：

- `display`
- `type_name`
- `debug`

`debug` 本身也不会进入当前 `SourceFrame` 的 serde 输出。

## 5. 版本规则

当前规则：

- 增加或删除稳定字段，必须评估是否升级 `STABLE_SNAPSHOT_SCHEMA_VERSION`
- 非稳定字段变化不自动改变稳定 schema version

## 6. 当前非目标

当前版本不承诺：

- typed roundtrip
- tagged JSON
- 从 JSON 反序列化回 runtime error
- 从 JSON 反序列化回 runtime error
