# V1 Fix And Review Plan

更新时间：2026-04-21

本文档用于冻结 `orion-error` 在 `0.6.x / V1 API` 阶段的修复方案与评审顺序。

目的只有一个：

- 先把正确解法写死
- 再按同一套标准评审
- 不再边改边试、反复摇摆

## 1. 适用范围

本文档只覆盖 `V1` 范围内可立即修复并可立即锁住的问题。

不覆盖：

- `V2` 破坏性重构
- `StructError: StdError` 的根架构调整
- bridge 类型体系的最终形态

## 2. V1 验收标准

后续所有实现与评审，只接受同时满足以下 6 条的方案：

1. `into_as(...)` 不提供 `E: StdError` blanket impl。
2. `StructError<_>` 不能走 `into_as(...)` / `raw_source(...)`。
3. 下游自定义 raw `StdError` 仍保留显式 opt-in 入口。
4. `wrap_as(...)` 继续作为结构化错误上卷主路径。
5. README / tutorial / migration checklist / RFC 口径一致。
6. 关键边界必须由测试、doctest 或 `compile_fail` 锁住。

如果任意一条不满足，则方案直接判定为不通过。

## 3. V1 正确解法

### 3.1 主入口分流

- `IntoAs` 只对封闭的 `UnstructuredSource` 开放。
- 不为 `Result<T, E> where E: StdError` 提供 blanket impl。
- 内置 allowlist raw error 直接实现 `UnstructuredSource`。

例如：

- `std::io::Error`
- `anyhow::Error`
- `serde_json::Error`
- `toml::de::Error`
- `toml::ser::Error`

### 3.2 显式 raw 逃生门

- `raw_source(...)` 只接受 `E: RawStdError`。
- `RawStdError` 是公开 marker trait。
- `RawStdError` 不提供 blanket impl。

这意味着：

- 下游可以为“自己的本地 raw `StdError` 类型”显式实现 `RawStdError`
- 下游不能为 `StructError<_>` 实现 `RawStdError`
  - 因为 trait 与 type 都是外部项
  - 这里依赖 orphan rule 保住 V1 边界

- `RawSource<E>` 再由库内实现 `UnstructuredSource`

### 3.3 结构化错误上卷

- `Result<T, StructError<_>>` 只走：
  - `wrap_as(...)`
  - `err_conv()`

- `wrap_as(...)` 作为公开主路径
- `err_wrap(...)` 只保留 compat/bridge 语义，不混入新体系主叙事

### 3.4 source 分流

- 普通 source：`with_std_source(...)`
- 结构化 source：`with_struct_source(...)`
- `with_source(...)` 只保留兼容，不再作为 V1 主路径

### 3.5 上下文命名

- `doing(...)`：V1 推荐主命名
- `at(...)`：V1 推荐辅助命名

但 V1 中：

- `doing(...)` 只是 `want(...)` 的命名糖衣
- `at(...)` 只是 `with(...)` 的命名糖衣

V1 不调整底层 `OperationContext` 模型语义。

## 4. 明确禁止的错误修法

以下做法一律判定为错误方向：

- 重新引入 `E: StdError` blanket 风格的 `into_as(...)`
- 让 `StructError<_>` 能经由 `raw_source(...)` 重新进入 `into_as(...)`
- 把 `RawStdError` 做成外部不可实现，从而删除下游显式逃生门
- 把 `err_wrap(...)` 再写成 V1 推荐主路径
- 把 `want(...)` / `with_source(...)` 继续写成主文档首选写法
- 只靠运行时 panic 兜底，而不通过 API 形状和编译期约束收紧边界

## 5. 评审顺序

后续评审必须按以下顺序进行，不再边改边试：

### 5.1 API 形状评审

只看：

- trait 边界
- 公开入口
- blanket impl 风险
- 下游 opt-in 是否仍存在

通过条件：

- 不误吞 `StructError<_>`
- 不删除显式 raw 逃生门
- 不退回 blanket `StdError`

### 5.2 契约评审

必须确认以下约束已被可执行锁住：

- 正向 doctest
  - 下游本地 raw error 可实现 `RawStdError`
  - `raw_source(...) + into_as(...)` 可用

- 反向 `compile_fail`
  - `StructError<_>` 不能进入 `raw_source(...)`

### 5.3 文档评审

必须检查以下文档是否全部同口径：

- `README.md`
- `docs/tutorial.md`
- `docs/v1-migration-checklist.md`
- `docs/orion-error-mini-rfc.md`

要求：

- 主路径只讲 `into_as(...) / wrap_as(...) / with_std_source(...) / doing(...)`
- compat API 只能明确标记为兼容层

### 5.4 compat 评审

兼容 API 可以保留，但必须满足：

- `err_wrap(...)` 只作为 compat/bridge
- `owe(...)` 只作为 compat
- `owe_*()` / `owe_*_source()` 已从当前主代码移除
- `want(...)` / `with_source(...)` 不再占据主叙事位

## 6. 评审输出格式

在未批准改动前，评审只输出两类信息：

1. `通过` / `不通过`
2. `不通过项`
   - 严重级别
   - 违反哪条验收标准
   - 建议如何修改

不再采用“先动手、再回头修”的工作方式。

## 7. V1 与 V2 边界

本文档只处理：

- V1 范围内能修的
- V1 范围内能锁住的

以下问题不在 V1 中假装彻底解决：

- `StructError: StdError` 的根冲突
- 标准错误生态与结构化错误体系的最终桥接模型
- `StructError` 退出 `StdError` 后的终局 API

这些问题必须继续记账到 `V2`。

## 8. 结论

V1 的工作原则固定为：

- V1 范围内能修的，立即修并锁住
- V1 范围内修不干净的，明确记账到 V2

不再接受：

- 便利性先行
- 文档与实现不同口径
- 依赖运行时 panic 作为主要防线
