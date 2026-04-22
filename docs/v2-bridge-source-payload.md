# V2 Bridge / Source Payload 草案

更新时间：2026-04-22

本文档用于冻结 `orion-error 0.7.x / V2` 第一阶段里
`bridge / source payload` 的设计基线。

这一步的目标不是立刻重写 `StructError<R>` 的内部存储，
而是先回答下面三个问题：

- 当前 source 模型到底哪里还不够清晰
- V2 想收敛到什么样的双通道模型
- 在 `StructError: StdError` 退出之后，source 主路径怎样稳定收口

## 1. 当前状态

目前的 source 模型，在公开 API 上已经做了第一轮分流：

- 普通 source：`with_std_source(...)`
- 结构化 source：`with_struct_source(...)`
- 普通错误第一次进入结构化体系：`into_as(...)`
- 下游显式 raw `StdError` 逃生门：`raw_source(...)`
- 已结构化错误向上卷边界：`wrap_as(...)`

这比 V1 早期的单一 `with_source(...)` 明显清晰很多。

但在内部模型层，当前 `StructError<R>` 仍然是：

- `source_payload: Option<InternalSourcePayload>`
- `InternalSourcePayload::Std { source, frames }`
- `InternalSourcePayload::Struct { source, frames }`

也就是说：

- 模型层已经开始显式区分 `Std` source 和 `Struct` source
- `with_struct_source(...)` 的“结构化语义”不再只是额外 frame 收集逻辑
- runtime carrier 内部继续保留 bridge 对象来承接 `source()` 观察链

这就是 V2 还必须继续处理的问题。

## 2. 当前问题

### 2.1 模型层仍然是“单槽位 + 附带 frame”

公开 API 已经要求调用者显式区分：

- `with_std_source(...)`
- `with_struct_source(...)`

内部现在已经不再只是“同一个槽位 + kind 标记”，而是开始落到
`InternalSourcePayload::Std` / `InternalSourcePayload::Struct` 两个分支。

这意味着当前实现已经完成第一步“模型面分流”，
但还没有完成最终桥接模型：

- 结构化 source 分支当前仍通过内部 `StdError` bridge wrapper 支撑 `source()`
- 历史 `with_source(...)` / `builder.source(...)` 已从主代码移除

### 2.2 `StructError: StdError` 仍然压着 bridge 设计

只要 `StructError<R>` 继续直接实现 `StdError`：

- `StructError<R>` 就仍然是 `StdError`
- “普通错误 blanket impl”和“结构化错误专用 impl”天然容易重叠
- 很多看似统一的自动入口，在类型层其实并不稳定

这也是为什么 V1 里：

- `into_as(...)` 不能用 `E: StdError` blanket impl
- `raw_source(...)` 必须收紧成显式 opt-in
- `with_std_source(...)` 和 `with_struct_source(...)` 不能重新合并成一个公开主入口

### 2.3 公开 API 与内部模型还没有完全对齐

现在的 API 语义已经在告诉用户：

- `Std` source 和 `Struct` source 是两种东西

但内部实现还没有用显式类型表达这个差异。

V2 的任务不是推翻 V1，而是把这个差异继续向模型层推进。

## 3. V2 第一阶段的结论

V2 第一阶段先冻结一个目标模型：

```rust,ignore
pub enum SourcePayload {
    Std(Box<dyn std::error::Error + Send + Sync>),
    Struct(Box<dyn StructChainDyn>),
}
```

这里的核心不是具体名字，而是语义：

- `Std`
  - 表示普通非结构化 source
- `Struct`
  - 表示已经进入结构化错误体系的 source

这两个分支在模型层就是不同对象，而不是：

- 一个 `StdError` 对象
- 再外加一份“猜出来的 frames”

## 4. V2 第一阶段的硬约束

这一步只冻结边界，不做超前承诺。

### 4.1 继续承认双通道是长期方向

后续所有设计、review、文档讨论，都应以：

- `SourcePayload::Std`
- `SourcePayload::Struct`

作为目标模型。

即便当前实现还没有真的切过去，也不能再回到
“source 本质上只有一个模糊槽位”的叙事。

### 4.2 统一公开入口已正式落地

当前实现已经把以下 API 作为公开主路径：

- `IntoSourcePayload`
- `StructError::attach_source(...)`
- `StructErrorBuilder::attach_source(...)`

它们现在可以稳定成立，是因为 `StructError<R>` 已经不再直接实现
`StdError`，因此普通 `StdError` 和结构化 `StructError<R>` 可以在类型层自然分流。

### 4.3 `with_std_source(...)` / `with_struct_source(...)` 继续保留

在 V2 第一阶段里，这两个入口继续作为公开主路径：

- 它们已经明确表达调用方意图
- 它们不会重新把 source 边界揉成一团
- 它们和 V1 的迁移路径完全一致

所以 V2 第一阶段不会试图把它们重新合并成一个方法。

## 5. 对现有 V1 API 的解释口径

### 5.1 `with_std_source(...)`

解释口径：

- 这是“附加普通非结构化 source”
- 它对应未来的 `SourcePayload::Std`

### 5.2 `with_struct_source(...)`

解释口径：

- 这是“附加结构化 source”
- 它对应未来的 `SourcePayload::Struct`

### 5.3 `into_as(...)`

解释口径：

- 这是普通错误第一次进入结构化体系
- 本质上是在构造新的 runtime carrier，并把原始错误挂到 `Std` source 通道

### 5.4 `wrap_as(...)`

解释口径：

- 这是已结构化错误建立新的上层 reason 边界
- 本质上是在构造新的 runtime carrier，并把下层错误挂到 `Struct` source 通道

### 5.5 `raw_source(...)`

解释口径：

- 它只是 V1/V2 过渡期的显式 raw `StdError` opt-in 入口
- 它不是统一 source payload API 的雏形
- 它也绝不扩展到 `Display` only 值

## 6. 这一阶段明确不做的事

### 6.1 不急着重写当前 `source_frames`

当前 `source_frames` 仍然是 runtime 上的过渡能力。

V2 第一阶段先承认：

- 它服务当前导出与 report
- 它不是最终 source payload 模型本身

后续如果真正切到 `SourcePayload::Std / Struct`，
再决定 frame 的生成时机与归属。

## 7. 当前实现状态与后续顺序

`bridge / source payload` 这条线，后续建议按这个顺序推进：

1. 先冻结本文档
2. 明确 `StructError: StdError` 的退出策略与 bridge 口径
   - 见 `docs/v2-structerror-stderror-strategy.md`
3. 设计内部 `SourcePayload` 过渡存储草案
4. 引入 bridge wrapper：
   - `OwnedStdStructError<R>`
   - `StdStructRef<'a, R>`
5. 在默认构建下公开统一入口，如 `attach_source(...)`
6. 继续收缩只剩 compat 语义的旧入口

当前进度：

- `InternalSourcePayload::Std / Struct` 已经落地
- `SourcePayloadKind` / `SourcePayloadRef<'_>` 已经作为只读观察模型落地
- `OwnedStdStructError<R>` / `StdStructRef<'a, R>` 已经作为公开 bridge 类型落地
- `OwnedDynStdStructError` 已经作为 type-erased 官方 owned bridge 落地，用于 `anyhow` 这类只能按具体类型 downcast 的边界
- `StructError::into_std()` / `StructError::as_std()` 已经落地
- `StructError::into_dyn_std()` 已经落地
- `From<StructError<R>> for OwnedStdStructError<R>` 已经落地
- `From<&StructError<R>> for StdStructRef<'_, R>` 已经落地
- `OwnedStdStructError::into_struct()` / `inner()` / `StdStructRef::inner()` 已经落地
- `StructError::into_boxed_std()` / `OwnedStdStructError::into_boxed()` 已经落地
- `anyhow::Error` 的 `into_as(...)` 只识别顶层 `OwnedDynStdStructError`，不扫描 source 链，不猜第三方 wrapper
- `StructError<R>` 已不再直接实现 `std::error::Error`
- `SourcePayload` / `IntoSourcePayload` 已公开
- `StructError::attach_source(...)` / `StructErrorBuilder::attach_source(...)` 已公开
- `attach_source(...)` 可自动分流普通 `StdError` 与 `StructError<_>`

当前公开 source payload 能力分两类。

只读观察：

```rust,ignore
let payload = err.source_payload();
let kind = err.source_payload_kind();
```

它用于判断当前 source 是普通 `Std` 分支还是结构化 `Struct` 分支，
以及读取已生成的 source frames / root cause / source chain。

写入入口：

```rust,ignore
let err = StructError::from(reason).attach_source(source);
let err = StructError::builder(reason).attach_source(source).finish();
```

它把 V2 的双通道 source payload 从文档目标推进为公开主路径。

如果外部边界需要 `dyn std::error::Error`，必须显式桥接：

```rust,ignore
let borrowed = err.as_std();
let owned = err.into_std();
let boxed = err.into_boxed_std();
```

这条规则只改变标准错误生态接入方式，不改变 `source_payload()` /
`source_payload_kind()` / `source_frames()` 的只读观察语义。

顺序不能反过来。

如果还没处理 `StructError: StdError` 的类型边界，就先上统一入口，
只会重新制造一轮 V1 已经暴露过的问题。

## 8. 当前阶段完成标准

当以下条件满足时，可以认为 V2 第一阶段的
`bridge / source payload` 基线已经建立：

- 有独立文档冻结 `Std / Struct` 双通道方向
- `V2 Development Plan` 承认这份文档是下一块基线
- 导航文档能把读者带到这里
- 后续 review 可以明确拒绝“现在就重新合并 source 入口”的提案

在这一步之前，不建议直接重写 `StructError<R>` 的 source 存储。
