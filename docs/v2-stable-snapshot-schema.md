# V2 Stable Snapshot Schema

更新时间：2026-04-22

本文档冻结 `orion-error 0.7.x / V2` 当前稳定 snapshot JSON 的第二版协议。

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
serde_json::to_value(StableStructErrorSnapshot::from(err.snapshot()))
serde_json::to_value(StableStructErrorSnapshot::from(&err))
```

`StructErrorSnapshot` 自身的默认 `serde Serialize` 现在直接输出稳定 schema。

稳定 snapshot 也可以显式降级成兼容投影或 report：

```rust,ignore
let compat = stable.compat_export();
let report = stable.report();
```

其中：

- `stable.report()` 仍是当前主路径
- `stable.compat_export()` 已进入 deprecated migration-only 路径

如果仍需要旧 compat JSON 投影，可显式使用：

```rust,ignore
snapshot.to_compat_snapshot_json()
serde_json::to_value(snapshot.compat_serialize())
```

这些 compat snapshot JSON 入口也都已进入 deprecated migration-only 路径。

这是一种有损投影：

- `context.fields` 会为空
- `context.result` 会使用兼容默认值 `Fail`
- `source_frames.display` 会为空
- `source_frames.type_name` 会为空

它只用于渲染、诊断和兼容观察，不表示可以从稳定 snapshot 还原 runtime
`StructError<_>`。

## 2. 顶层字段

`StableStructErrorSnapshot` 当前字段：

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

这些字段只保留在 deprecated 的 `compat_export()` / 显式 compat JSON 投影里。

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
- 兼容投影字段变化不自动改变稳定 schema version
- compat JSON 入口变化不自动改变稳定 schema version

## 6. 当前非目标

当前版本不承诺：

- typed roundtrip
- tagged JSON
- 从 JSON 反序列化回 runtime error
- 从 JSON 反序列化回 runtime error
