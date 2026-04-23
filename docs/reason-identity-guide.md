# OrionError 与稳定身份使用指南

本文面向 `orion-error 0.7.0`，说明几个容易混淆的概念：

- `OrionError`：领域 reason 的推荐 derive 入口
- `DomainReason`：错误的运行时语义载体
- 稳定身份：错误对外暴露时的机器主键

新代码推荐用 `OrionError` derive 注解同时提供 `Display`、兼容数值码和稳定身份，不要手写重复的 `match`。

## 1. 先看当前源码里的实际约束

`DomainReason` 只要求：

```rust
pub trait DomainReason: PartialEq + Display {}
```

`DomainReason` 现在是显式 marker。新代码通过 derive `OrionError` 自动获得实现，不再依赖“满足若干 trait 即自动成为 reason”的 blanket impl。

`ErrorIdentityProvider` 是 `OrionError` 生成的底层能力 trait。正常业务代码不需要手写它；只有高级集成、宏实现或迁移旧类型时才需要直接接触。

## 2. 这两个东西分别负责什么

`DomainReason` 负责：

- 让 `StructError<R>` 有一个明确的 reason 类型
- 支撑 `reason().to_string()` 这样的可读文本
- 支撑普通错误传播、上下文挂载、source 挂载、基础 report/snapshot

稳定身份负责：

- 给错误一个稳定的 `code`，例如 `sys.io_error`
- 给错误一个稳定的 `category`，例如 `Sys`
- 支撑稳定断言
- 支撑策略判断
- 支撑统一出口投影，例如 HTTP / RPC / CLI / log

一句话区分：

- `Display` / `reason text` 是给人看的
- `identity` / `category` 是给机器和边界协议看的

## 3. 推荐入口：OrionError 一次补齐身份

推荐写法：

```rust
use derive_more::From;
use orion_error::{
    prelude::*,
    reason::UvsReason,
    report::DefaultErrorPolicy,
};

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum AppError {
    #[orion_error(identity = "biz.invalid_request")]
    InvalidRequest,
    #[orion_error(transparent)]
    Uvs(UvsReason),
}

fn load_config() -> Result<String, StructError<AppError>> {
    std::fs::read_to_string("config.toml")
        .into_as(AppError::from(UvsReason::system_error()), "read config failed")
}

let err = load_config().unwrap_err();
let http = err.http_response(&DefaultErrorPolicy);

assert_eq!(http.code, "sys.io_error");
```

此时，错误就可以进入统一出口协议了。

`OrionError` 会生成协议消费接口需要的稳定身份能力。下面这些 API 因此可以直接使用：

- `assert_err_code(...)`
- `assert_err_category(...)`
- `assert_err_identity(...)`
- `StructError::identity_snapshot()`
- `StructError::policy_report()`
- `StructError::into_policy_report()`
- `StructError::policy_snapshot(...)`
- `StructError::http_response(...)`
- `StructError::cli_response(...)`
- `StructError::log_response(...)`
- `StructError::rpc_response(...)`
- `StructError::render_user_debug(...)`
- `StructError::render_user_debug_redacted(...)`

## 4. 为什么只靠 Display 不够

假设你的错误文本是：

```text
read config failed
```

它对人类很清楚，但对机器并不稳定：

- 文案可能改成 `failed to read config`
- 国际化后可能变成中文
- 文本里可能混入路径、参数、资源名等动态内容

如果前端、CLI 脚本、日志平台、指标系统直接依赖这些文案，就会很脆弱。

所以出口协议通常更需要这种稳定字段：

- `code = "sys.io_error"`
- `category = Sys`

注解规则：

- `identity = "biz.invalid_request"` 生成协议稳定身份码。
- `category` 默认从 `identity` 前缀推导，支持 `biz` / `conf` / `logic` / `sys`。
- `message` 默认从 `identity` 的最后一段推导，例如 `biz.invalid_request` 会生成 `invalid request`。
- `code = 1000` 只生成 legacy numeric code；新业务默认不需要写。
- `code` 未写时默认是兼容用数值码 `500`。
- 如果需要更自然的人类文案，可以显式写 `message = "..."` 覆盖默认推导。
- `#[orion_error(transparent)]` 用于单字段包装变体，直接委托给内部 reason，例如 `Uvs(UvsReason)`。

## 5. “要进 HTTP / RPC / CLI / log 契约”到底是什么意思

这里的“契约”指的是：错误不再只在 Rust 内部流转，而是会被投影为对外稳定的响应结构。

当前 crate 已定义这些响应：

- `ErrorHttpResponse`
- `ErrorCliResponse`
- `ErrorLogResponse`
- `ErrorRpcResponse`

这些结构里都有稳定字段：

- `code`
- `category`

典型使用场景：

- 前端按 `code` 决定错误提示或交互行为
- 网关按 `category` 记录业务错误和系统错误
- CLI 脚本按 `code` 判断是否需要特殊退出路径
- 日志平台按 `code` 聚合异常
- RPC 调用方按 `code` 和 `retryable` 做重试或降级

如果没有稳定身份，这些外部系统就只能依赖错误文本，成本高且不稳定。

## 6. 什么时候可以先不用 OrionError

下面这些情况，可以先不接入稳定身份：

- 错误只在模块内部传播
- 只是为了少量业务逻辑分支而定义本地 reason
- 当前阶段还没有对外协议、稳定测试或统一治理需求
- 你还在快速重构错误分类，不想过早冻结 code

这种情况属于高级/临时场景。主路径仍建议从 `OrionError` 开始，因为它的注解成本已经足够低。

## 7. 什么时候应该尽快提供稳定身份

下面这些情况，建议尽快实现：

- 要使用 `assert_err_identity(...)`
- 要导出 HTTP / RPC / CLI / log 响应
- 要根据错误做统一策略判断
- 要把错误接入指标、告警、聚合分析
- 要形成跨 crate、跨服务的稳定错误协议

这时 `identity` 应该被视为公开契约的一部分，而不是随手命名的内部文案。

## 8. 设计建议

为自定义 reason 设计 `identity` 时，建议遵守这些约束：

- code 要稳定，不要把动态信息编码进去
- code 要表达语义，而不是实现细节
- category 要保持粗粒度，避免把它当成完整 reason
- 领域变体可以自定义 code
- 通用底层错误优先透传 `UvsReason`

例如：

- `biz.invalid_request`
- `biz.order_not_found`
- `conf.feature_invalid`
- `sys.io_error`

不建议：

- `read_config_failed_for_user_42`
- `error_1`
- `db_timeout_in_us_east_1_primary`

## 9. 推荐做法

推荐把领域错误分成两类：

- 少量真正需要业务语义的领域变体
- 一个 `Uvs(UvsReason)` 兜底通道

例如：

```rust
#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum OrderError {
    #[orion_error(identity = "biz.insufficient_funds")]
    InsufficientFunds,
    #[orion_error(identity = "biz.order_not_found")]
    OrderNotFound,
    #[orion_error(transparent)]
    Uvs(UvsReason),
}
```

然后：

- 业务特有错误自己定义 `identity`
- 通用系统类错误直接复用 `UvsReason` 的稳定身份

这样通常是最省成本、也最容易长期维护的方案。

## 10. 使用者的决策规则

可以直接用下面这条规则：

1. 新 reason 默认 derive `OrionError`
2. 用 `identity = "biz.xxx"` 作为协议主键
3. 只有需要兼容旧数值码时才写 `code = ...`
4. 如果错误要进入测试断言、策略系统、导出协议、日志聚合或跨服务边界，必须保证 identity 稳定

换句话说：

- `DomainReason` 解决“这个错误能不能在运行时结构化传播”
- `OrionError` 解决“这个错误能不能被系统稳定识别和消费”

## 11. 为什么需要 snapshot

引入 `snapshot` 的核心原因是：

- `StructError<R>` 是运行时对象
- 但导出、序列化、测试断言、稳定协议消费需要的是只读快照对象

如果没有 `snapshot` 这一层，`StructError<R>` 很容易同时承担过多职责：

- runtime carrier
- source bridge carrier
- snapshot/export object
- report/render entry

这会让运行时对象被导出需求反向牵制，后续内部字段、source 存储模型和导出 schema 更难演进。

### 11.1 `StructError`、`snapshot`、`report` 各自解决什么问题

可以先用一句话区分：

- `StructError<R>` 解决“程序里怎么传”
- `ErrorSnapshot` 解决“导出时保存什么结构”
- `DiagnosticReport` 解决“给人怎么展示”

也可以把它类比成：

- `StructError<R>`：进程内运行时对象
- `ErrorSnapshot`：机器可读 DTO / export record
- `DiagnosticReport`：面向人类的 view model

### 11.2 为什么不能直接拿 `StructError` 做导出对象

`StructError<R>` 的目标是：

- 好传播
- 好挂 context
- 好保留 source
- 好做运行时转换

而导出对象的目标是：

- 字段稳定
- 结构清晰
- 适合序列化
- 适合测试断言
- 适合后续 policy / projection 消费

这两类目标并不一致。

如果把 `StructError<R>` 直接当导出对象，就会出现几个问题：

- runtime 内部实现容易被导出协议绑死
- 每次改字段都要担心 JSON / 测试兼容性
- render / report / snapshot 职责会继续缠在一起
- `StructError` 会不断膨胀

### 11.3 snapshot 的具体价值

当前 `StructError::snapshot()` 会把运行时错误冻结成只读结构，包含：

- `reason`
- `detail`
- `position`
- `want`
- `path`
- `context`
- `root_metadata`
- `source_frames`

有了这个中间层，后续能力就可以围绕 snapshot 收敛：

- 稳定导出
- JSON schema
- 测试断言
- report 构造
- identity / policy / projection 消费

这意味着：

- runtime 负责采集
- snapshot 负责冻结
- report 负责展示

### 11.4 snapshot 为什么不是 report

`report` 的核心职责是：

- 人类可读渲染
- redaction
- 诊断展示

它本质上是展示层视图。

而 `snapshot` 的核心职责是：

- 机器可读结构
- 稳定字段承载
- 中间导出对象

所以：

- `snapshot` 不该被渲染需求反向定义
- `report` 也不该成为稳定导出真身

当前实现里，`ErrorSnapshot` 可以继续转成 `DiagnosticReport`，正是因为它被设计成 runtime 和 report 之间的中间层。

### 11.5 为什么 snapshot 对测试和协议更重要

如果直接对 `Display` 文本或最终渲染结果做断言，会很脆弱：

- 文案调整会导致测试失效
- 渲染格式变化会导致测试失效
- 本地化或 detail 微调会带来大量噪音

而 `snapshot` 更适合断言结构字段，例如：

- `reason`
- `detail`
- `position`
- `context`
- `source_frames`
- stable export schema

所以从工程上看，snapshot 是把“运行时错误对象”变成“稳定消费输入”的第一步。

### 11.6 一句话总结

如果只记一句，可以记这个：

- `StructError` 是活的运行时 carrier
- `ErrorSnapshot` 是静态的只读快照
- `DiagnosticReport` 是给人看的展示视图

## 12. 当前概念关系对照

当前模型分成两条主线：

- 内部结构分层：runtime / snapshot / report / bridge
- 外部消费协议：stable identity / policy / projection

可以先记一句话：

- runtime / snapshot / report / bridge 回答“错误对象怎么建模和传播”
- identity / policy / projection 回答“错误对象怎么被稳定消费”

### 12.1 命名约定

后续如果继续收敛公开 API 命名，建议遵守以下规则：

- 能力 trait 使用 `*Provider` 后缀
- 数据对象使用名词，不额外带 `Trait` / `Able`
- 最终出口对象继续使用 `*Response`
- policy 的计算结果继续使用 `*Decision`
- 中间输入对象优先使用 `*Input`，少用语义模糊的 `View`

按这个规则，当前底层能力 trait 仍使用 `ErrorIdentityProvider`：

```rust
trait ErrorIdentityProvider {
    fn stable_code(&self) -> &'static str;
    fn error_category(&self) -> ErrorCategory;
}
```

这组命名的目标是让角色更清楚：

- `ErrorIdentityProvider`：类型提供稳定错误身份的底层能力 trait，通常由 `OrionError` 生成
- `ErrorIdentity`：稳定错误身份数据对象
- `ErrorPolicy`：根据稳定身份做出口行为决策
- `ErrorPolicyDecision`：policy 的计算结果
- `ErrorHttpResponse` / `ErrorRpcResponse`：最终出口投影

后续仍可考虑的命名清理：

| 当前命名 | 可选推荐命名 | 理由 |
| --- | --- | --- |
| `SnapshotContextFrame` | `ContextSnapshotFrame` | 词序更自然，表达 context 的 snapshot frame |
| `SnapshotSourceFrame` | `SourceSnapshotFrame` | 词序更自然，表达 source 的 snapshot frame |
| `StableSnapshotContextFrame` | `StableContextSnapshotFrame` | 与 `ContextSnapshotFrame` 配套 |
| `StableSnapshotSourceFrame` | `StableSourceSnapshotFrame` | 与 `SourceSnapshotFrame` 配套 |

建议保持不动的命名：

| 当前命名 | 理由 |
| --- | --- |
| `StructError<R>` | 已是库核心运行时类型，改名迁移成本高 |
| `DomainReason` | 语义清楚，表达领域 reason 的最小约束 |
| `ErrorPolicy` | 简洁且表达策略层职责 |
| `ErrorPolicyDecision` | 表达 policy 的计算结果，当前已经清楚 |
| `ErrorHttpResponse` / `ErrorCliResponse` / `ErrorLogResponse` / `ErrorRpcResponse` | 最终出口对象，`*Response` 后缀清楚 |

其他 frame / renderer 命名可以作为后续清理项。

### 12.2 它们各自关心什么

内部结构分层关心的是：

- runtime
- snapshot
- report
- bridge

它要解决的问题是：

- `StructError<R>` 在运行时怎么传播
- source 怎么桥接标准错误和结构化错误
- snapshot 和 report 怎么拆层
- `StructError` 和 `StdError` 的边界怎么处理

外部消费协议关心的是：

- stable identity
- policy
- projection

它要解决的问题是：

- 错误的稳定主键是什么
- 该错误默认应该公开还是隐藏
- HTTP status / retryable / hints 应该如何决定
- HTTP / CLI / log / RPC 应该导出成什么结构
- 测试应该断言哪些稳定字段

### 12.3 概念对照表

这张对照表不应该被理解成“所有使用者都必须掌握这些概念”。

更合适的读法是：

- 第一张表是大多数使用者真正需要知道的概念
- 第二张表是实现层和进阶扩展时才需要进入的概念

#### 面向大多数使用者的最小概念集

| 概念 | 所属层 | 主要作用 |
| --- | --- | --- |
| `OrionError` | derive | 推荐的领域 reason 入口，一次生成展示文案、稳定身份、分类和兼容数值码 |
| `StructError<R>` | runtime | 运行时错误载体，负责 reason、detail、context、source 的传播 |
| `IntoAs` | conversion | 普通 `std::error::Error` 第一次进入结构化体系 |
| `ErrorWith` | context | 给错误追加 `doing(...)` / `at(...)` / `with_context(...)` |
| `ErrorWrapAs` | conversion | 上层为下层 `StructError<_>` 建立新的语义边界 |
| `DefaultErrorPolicy` | policy | 默认出口策略，决定 HTTP status、visibility、hints、retryable |

如果你只是业务侧接入，一般优先理解这张表即可。

#### 面向实现层和进阶扩展的概念

| 概念 | 所属层 | 主要作用 |
| --- | --- | --- |
| `ErrorIdentityProvider` | stable identity | 给错误提供稳定 `code` 和 `category`；通常由 `OrionError` 自动生成 |
| `ErrorPolicy` | policy | 根据稳定身份决定 `http_status`、`visibility`、`hints`、`retryable` |
| `ErrorHttpResponse` / `ErrorCliResponse` / `ErrorLogResponse` / `ErrorRpcResponse` | projection | HTTP / CLI / log / RPC 出口投影 |
| `assert_err_code(...)` / `assert_err_category(...)` / `assert_err_identity(...)` | test helper | 用稳定字段做测试断言 |
| `SourcePayload` / `IntoSourcePayload` | bridge | 承接普通 `StdError` 和结构化 source 的桥接 |
| `OwnedStdStructError` / `StdStructRef` / `into_std()` / `as_std()` | bridge | 显式进入标准错误生态 |
| `ErrorSnapshot` | snapshot | 机器可读快照，中间导出对象 |
| `SnapshotContextFrame` / `SnapshotSourceFrame` | snapshot | snapshot 层只读 frame |
| `DiagnosticReport` | report | 人类可读展示、渲染、redaction 的输入模型 |
| `TextDiagnosticRenderer` | renderer | 把 `DiagnosticReport` 渲染成文本 |
| `ErrorIdentity` | stable identity | 当前错误的稳定身份快照 |
| `ErrorPolicyDecision` | policy | policy 的计算结果 |
| `ErrorPolicyInput` | policy input | 把 `identity + report` 绑定成统一消费输入 |
| `ErrorProtocolSnapshot` | protocol snapshot | 把 `identity + decision + report` 固化成完整协议输入 |

这些概念不是不重要，而是更偏：

- 分层实现
- 协议中间态
- 高级导出/调试
- 自定义扩展

### 12.4 最短流转图

用当前实现的主调用路径来理解，最顺的心智模型是：

```text
StructError<R>
  -> identity_snapshot()
  -> report()
  -> policy_report()            // identity + report
  -> policy_snapshot(policy)    // + decision
  -> http/cli/log/rpc response
```

如果从对象层次来理解，也可以记成：

```text
StructError<R>
  -> ErrorSnapshot
  -> DiagnosticReport
  -> ErrorIdentity
  -> ErrorPolicyDecision
  -> HTTP / CLI / log / RPC projection
```

但要注意：

- `ErrorIdentity` 的正式入口是 `StructError::identity_snapshot()`
- `ErrorPolicyInput` 实际上把 `identity + report` 组合成统一消费输入
- `policy snapshot` 才是当前最完整的统一协议输入

### 12.5 一句话总结

如果只记最后一句，可以记这个：

- `StructError` 是运行时对象
- `Snapshot` / `Report` 是内部结构分层对象
- `Identity` / `Policy` / `Projection` 是外部消费协议对象
