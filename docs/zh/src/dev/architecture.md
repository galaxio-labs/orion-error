# orion-error 0.8.0 架构

本文描述的是 orion-error `0.8.0` 的理想设计架构：它解释公开 API 背后的设计约束、核心数据流和治理目标。文中的结构体片段用于说明模型边界，不等同于源码逐字段快照；精确字段以 `src/` 中实现为准。

## 问题

大型 Rust 服务中的错误处理，有五个未满足的需求：

1. **收敛不丢信息。** 下层技术错误需要抽象成上层稳定语义，但原始根因（source chain、detail、context）必须保留给排障使用。
2. **跨层传播。** 错误经过多层（handler → service → repository → database），每层都需要附着自己的上下文，但不能丢弃前面的信息。
3. **边界投影。** 同一个错误面向不同对象必须有不同视图：最终用户（安全消息）、运维（组件 + 可重试性）、协议客户端（稳定 code + 结构）、开发者（完整链）。
4. **可治理的稳定身份。** 错误需要稳定、机器可读的 identity，在重构后仍保持不变，贯穿 HTTP/RPC/日志/CLI 边界。
5. **结构化载体。** 错误携带 detail、source chain、操作上下文和 metadata，全部是结构化字段，而不是字符串拼接。

现有方案各自解决了一部分：

| 库 | 优势 | 未覆盖 |
|----|------|--------|
| `thiserror` | 本地错误 enum 建模，生成 Display + From | 跨层传播、上下文附着、协议投影 |
| `anyhow` | 应用层错误统一，context() | 稳定 identity、协议输出、细粒度分类路由 |
| `color-eyre` | 丰富的诊断报告 | 同 anyhow——无协议或 identity 层 |

**orion-error** 瞄准的是这个空白：**大规模治理**——错误经过 3-5 层后，在协议边界以稳定结构输出的场景。

---

## 核心洞察：Reason/Carrier 分离

最核心的设计决策是：**把错误的语义分类（reason）和传播机制（carrier）分离。**

```rust
// reason = 这是什么类型的错误
enum AppReason {
    InvalidInput,
    OrderNotFound,
    General(UnifiedReason),
}

// carrier = 它怎么传播
let err: StructError<AppReason> = AppReason::OrderNotFound
    .to_err()
    .with_detail("order #42 not found")
    .with_source(db_error)
    .with_context(ctx);
```

### 为什么要分离？

如果 reason 和 carrier 合在一起——就像典型的 `thiserror` enum 用法——每个运行时机制（context 附着、source 追踪、协议投影）都得在每个 enum 上重新实现。carrier (`StructError<T>`) 只需要实现一次。

reason 保持轻薄——只需要实现 `DomainReason` marker trait：

```rust
pub trait DomainReason: PartialEq + Display + Debug + Send + Sync + 'static {}
```

| 约束 | 原因 |
|------|------|
| `Display` + `Debug` | 错误必须可打印，用于诊断和日志 |
| `PartialEq` | 支持测试中断言 |
| `Send + Sync` | `StructError` 需要跨 async 任务边界，能被 `anyhow::Error` 或 `Box<dyn Error>` 捕获 |
| `'static` | 支持类型擦除 (`dyn Error`) 和 `SourceFrame` 存储 |

---

## 错误流转

```text
raw std error ──→ .source_err(reason, detail) ──→ 首次进入结构化系统
                                                        │
                                                  conv_err()
                                              (reason 重新映射)
                                                        │
                          report / exposure / display_chain
```

### 1. 入口：`source_err(reason, detail)`

统一入口，同时支持原始 `std::error::Error` 和已结构化的 `StructError` 源：

```rust
let result = std::fs::read_to_string("config.toml")
    .source_err(AppReason::system_error(), "read config failed")?;
```

- 原始错误作为 source frame 存储，保留 Display 和 Debug 输出
- `reason` 成为错误的稳定分类
- `detail` 提供当前层的解释

### 2. 跨层转换：`conv_err()`

当上游已是 `StructError<R1>`，只需要改变 reason 类型时使用：

```rust
fn upper_layer() -> Result<(), StructError<UpperReason>> {
    lower_layer().conv_err()?;
    Ok(())
}
```

需要 `UpperReason: From<LowerReason>`。所有 detail、context、source chain、metadata 都保留。

`From<StructError<R1>> for StructError<R2>` 的 blanket impl 被 Rust 的 orphan rule 阻止（`From` 和 `StructError` 都不属于用户 crate），因此使用显式 trait 方法。

### 3. 首次进入 vs 跨层转换

| 方法 | 语义 | Source 保留方式 |
|------|------|----------------|
| `source_err(reason, detail)` | 创建新的语义边界 | 作为未结构化或结构化 source 包裹 |
| `conv_err()` | 只重新映射 reason 类型 | 保留所有 detail、context、source、metadata |

---

## 核心类型

### `StructError<T: DomainReason>`

统一的运行时载体。概念上它把 reason 和运行时传播数据装进一个小尺寸 carrier：

```rust
pub struct StructError<T: DomainReason> {
    imp: Box<StructErrorImpl<T>>,
}
```

`Box` 用于保持 `StructError` 足够小（指针大小），因为它经常通过 `Result` 返回。

### `StructErrorImpl<T>`

存储错误传播所需的数据。简化模型如下：

```rust
struct StructErrorImpl<T> {
    reason: T,
    detail: Option<String>,
    position: Option<String>,
    context: Option<Arc<Vec<OperationContext>>>,
    source_payload: Option<InternalSourcePayload>,
}
```

关键决策：
- **`context: Option<Arc<Vec<...>>>`** — 惰性分配：没有 context 的错误不产生堆分配。`Arc` 使 context chain 可以廉价 clone
- **`Box<StructErrorImpl<T>>`** — `StructError` 自身保持小尺寸（一个指针），最小化 `Result` 的大小

### `OperationContext`

运行时上下文载体。概念上它描述“当前层正在做什么、访问什么、附带哪些诊断字段、是否触发日志输出”等信息：

```rust
pub struct OperationContext {
    action: Option<String>,
    locator: Option<String>,
    fields: Vec<(String, String)>,
    path: Vec<String>,
    metadata: ErrorMetadata,
    result: OperationResult,
    exit_log: bool,
}
```

- `doing(...)` — 正在执行什么操作（"load config", "validate order"）
- `at(...)` — 正在访问什么资源（"config.toml", "order #42"）
- `with_field(...)` — 人可读的诊断字段
- `with_meta(...)` — 机器消费的结构化 metadata（仅用于序列化）
- `success()` / `fail()` / `cancel()` 与日志方法 — 让调用方用少量代码记录操作结果

### `SourceFrame`

表示 source chain 中的一个元素。简化模型如下：

```rust
pub struct SourceFrame {
    pub index: usize,
    pub message: SmolStr,
    pub display: Option<SmolStr>,
    pub debug: Option<SmolStr>,
    pub type_name: Option<SmolStr>,
    pub error_code: Option<i32>,
    pub reason: Option<SmolStr>,
    pub path: Option<SmolStr>,
    pub detail: Option<SmolStr>,
    pub metadata: ErrorMetadata,
    pub is_root_cause: bool,
    pub context_fields: Vec<(SmolStr, SmolStr)>,
}
```

字符串字段使用 `SmolStr`（短字符串零分配优化），使 source chain 遍历时的 clone 更快。

---

## 消费路径

三个独立的消费出口，各自返回同一错误的不同视图：

### `report()` → `DiagnosticReport`

人类可读的诊断信息。只要求 `DomainReason`。

```rust
let report: DiagnosticReport = err.report();
println!("{}", report.render());
```

输出：
```text
reason: system error
detail: read config failed
context:
  [0] place_order [user_id: 42]
```

### `exposure(&policy)` → `ErrorProtocolSnapshot`

协议边界投影。需要 `ErrorIdentityProvider`（由 `#[derive(OrionError)]` 提供）。

```rust
let proto = err.exposure(&MyPolicy);
let http_json = proto.to_http_error_json()?;   // {"status": 500, "code": "sys.io_error", ...}
let log_json = proto.to_log_error_json()?;     // 完整结构化日志输出
let cli_json = proto.to_cli_error_json()?;     // 面向运维的摘要
let rpc_json = proto.to_rpc_error_json()?;     // 面向上游的协议输出
```

`ExposurePolicy` trait 控制决策：

| 方法 | 默认值 | 覆盖频率 |
|------|--------|---------|
| `http_status()` | 500 | 最常见 |
| `visibility()` | `Internal` (Biz → `Public`) | 常见 |
| `retryable()` | `false` | 偶尔 |
| `default_hints()` | `[]` | 很少 |

`Visibility` 控制哪些错误信息到达外部调用方：

| | `Public` | `Internal` |
|---|---------|-----------|
| HTTP `message` | 使用 detail | 使用 reason（隐藏 detail） |
| RPC `detail` | 暴露 | `null` |

### `display_chain()` → 格式化字符串

Source chain 展开，用于排障。不要求额外 trait。

```text
system error
  -> Info: read config failed
  -> Caused by:
      1. outer source
      2. inner source
```

### `identity_snapshot()` → `ErrorIdentity`

稳定身份识别，不涉及协议投影：

```rust
let id = err.identity_snapshot();
assert_eq!(id.code, "sys.io_error");
```

---

## UnifiedReason

`UnifiedReason` 是内置的通用错误分类，覆盖大多数服务中都会出现的错误类别：

| 分类 | 编码范围 | 示例 |
|------|---------|------|
| 业务 | 100-105 | `validation_error`, `not_found` |
| 基础设施 | 200-204 | `system_error`, `network_error`, `timeout` |
| 配置与外部 | 300-301 | `core_conf`, `external_error` |

设计为不需要领域特化 reason 时的兜底。领域 enum 通常把它作为透明变体包含：

```rust
#[derive(OrionError)]
enum AppReason {
    #[orion_error(identity = "biz.invalid")]
    Invalid,
    #[orion_error(transparent)]
    General(UnifiedReason),
}
```

`#[orion_error(transparent)]` 属性将 `stable_code()`、`error_category()` 和 `Display` 委托给内部的 `UnifiedReason`。

---

## 显式 StdError 桥接

`StructError<T>` **不**实现 `std::error::Error`。这是有意为之：

1. **防止意外的类型擦除。** 如果 `StructError` 实现了 `StdError`，调用代码可能通过 `.into()` 或 `Box<dyn Error>` 无意中擦除 reason 类型，丢失结构化 identity。
2. **让边界跨越保持显式。** 当需要与 `StdError` 生态互操作时，转换是显式的：

```rust
let std_ref: StdStructRef<'_, AppReason> = err.as_std();
let owned: OwnedStdStructError<AppReason> = err.into_std();
let dyn_owned: OwnedDynStdStructError = err.into_dyn_std();
```

---

## Derive 宏

`#[derive(OrionError)]` 自动生成核心 trait 实现：

| Trait | 用途 | 来源 |
|-------|------|------|
| `Display` | 人类可读的错误信息 | 从 `message` 属性生成，或从 `identity` 自动推导 |
| `DomainReason` | Carrier 兼容性 | 空的 marker 实现 |
| `ErrorCode` | 兼容旧系统的传统数值编码 | 从 `code` 属性生成，或默认 500 |
| `ErrorIdentityProvider` | 稳定 code + category | 从 `identity` 和 `category` 属性生成 |

### 属性

| 属性 | 是否必需 | 生成 |
|------|---------|------|
| `identity = "biz.foo"` | 是（除非 `transparent`） | `stable_code()` 返回 `"biz.foo"` |
| `category = Biz` | 否（从 `identity` 前缀推断） | `error_category()` 返回指定分类 |
| `transparent` | `identity` 的替代 | 将所有方法委托给内部类型 |
| `message = "..."` | 否（从 `identity` 自动生成） | 自定义 `Display` 输出 |
| `code = ...` | 否（默认 500） | 传统数值 `error_code()` |

协议、日志聚合和监控应以 `ErrorIdentity.code` / `stable_code()` 作为稳定身份。`ErrorCode` 是数字码兼容层，不应作为新的外部协议主键。

### 透明变体构造器委托

当 enum 包含透明变体且包装了 `UnifiedReason` 时，所有 `UnifiedReason` 构造器会自动生成为该 enum 的方法：

```rust
#[derive(OrionError)]
enum AppReason {
    #[orion_error(transparent)]
    General(UnifiedReason),
}

// 自动生成：
AppReason::system_error()      // 而不是 AppReason::General(UnifiedReason::system_error())
AppReason::validation_error()
AppReason::not_found_error()
```

---

## 第三方错误集成

第三方错误类型通过 `source_err()` 进入结构化系统。支持的类型：

| 类型 | Feature | 机制 |
|------|---------|------|
| `std::io::Error` | 内置（无需 feature） | 直接 `UnstructuredSource` 实现 |
| `serde_json::Error` | `serde_json` | 直接 `UnstructuredSource` 实现 |
| `anyhow::Error` | `anyhow` | 尝试结构化恢复，失败则退化为未结构化 source |
| `toml::de::Error` | `toml` | 直接 `UnstructuredSource` 实现 |
| 自定义类型 | — | 通过 `RawStdError` + `raw_source()` 显式 opt-in |

Opt-in 设计（`RawStdError`）防止静默的结构化到未结构化降级：

```rust,ignore
impl RawStdError for MyError {}

let result: Result<(), MyError> = Err(MyError);
let err = result
    .map_err(raw_source)
    .source_err(AppReason::system_error(), "my operation failed")?;
```

---

## 设计演化

### 命名：UvsReason → CommonReason → UnifiedReason

内置 reason 类型经历了三次命名：

- **`UvsReason`** — 原始名称，含义不直观
- **`CommonReason`** — 中间改名，但 "Common" 听起来像"普通"而非"统一"
- **`UnifiedReason`** — 最终名称，反映其作用：具体错误收敛（统一）到这个分类

`pub type UvsReason = UnifiedReason;` 作为 deprecated 别名保留，用于迁移兼容。

### Variant 命名：Uvs → General

领域 enum 中的透明变体更名为 `General`：

```rust
// 之前
Uvs(UnifiedReason),

// 之后
General(UnifiedReason),
```

`General` 比 `Uvs` 更清楚地表达"这是非领域特化错误的兜底"。

### 消费路径收敛：snapshot 不作为主路径

orion-error 0.8.0 的架构主路径是 `report()`、`exposure()`、`display_chain()` 和 `identity_snapshot()`。

稳定机器身份由 `identity_snapshot()` 提供；面向 HTTP/RPC/CLI/log 的结构化边界输出由 `exposure()` 和 `ErrorProtocolSnapshot` 提供；人类诊断由 `report()` 提供。这样可以减少一条独立 snapshot 类型体系带来的 API 面，同时保留稳定身份和协议投影能力。

### API 命名：exposure

与 `report()` 保持一致。这个名字表达的是："在边界按策略暴露这个错误"，而不是要求用户先理解内部快照模型。

---

## Feature 门控

| Feature | 启用内容 | 默认 |
|---------|---------|------|
| `derive` | 过程宏派生（`OrionError`、`ErrorCode`、`ErrorIdentityProvider`） | 是 |
| `log` | `OperationContext` 日志方法（`ctx.info()`、`.debug()`、`.warn()`、`.error()`）和 Drop 自动日志 | 是 |
| `tracing` | Tracing 集成（同时启用时优先使用 tracing 而非 log） | 否 |
| `serde` | 核心类型的 Serialize/Deserialize 支持 | 否 |
| `serde_json` | 协议 JSON 投影方法（`to_http_error_json()` 等） | 否 |
| `anyhow` | `anyhow::Error` 互操作（支持结构化 source 恢复） | 否 |
| `toml` | `toml::de::Error` / `toml::ser::Error` 互操作 | 否 |

---

## 项目结构

```
src/
  lib.rs              — Crate 根，re-export，分层模块
  core/
    domain.rs         — DomainReason trait
    reason.rs         — ErrorCode trait、ErrorCategory enum、ErrorIdentityProvider trait
    universal.rs      — UnifiedReason enum（内置分类）
    error/
      carrier.rs      — StructError<T>、StructErrorImpl<T>
      builder.rs      — StructErrorBuilder<T>
      identity.rs     — ErrorIdentity struct、identity_snapshot()
      source_chain.rs — SourceFrame、source payload 基础设施
      std_bridge.rs   — StdStructRef、OwnedStdStructError、OwnedDynStdStructError
    context/
      types.rs        — OperationContext、OperationScope
      convert.rs      — ContextAdd trait
    metadata.rs       — ErrorMetadata、MetadataValue
    report/
      diagnostic.rs   — DiagnosticReport、redaction
      protocol.rs     — ErrorProtocolSnapshot、ExposurePolicy、Visibility
  traits/
    contextual.rs     — ErrorWith trait
    conversion.rs     — ConvErr、ConvStructError、ToStructError
    source_err.rs     — SourceErr、RawStdError、RawSource
  testing.rs          — 测试断言辅助
```

---

## 约束

### Orphan Rule

`From<StructError<R1>> for StructError<R2>` 的 blanket 实现不能提供——`From`（std）和 `StructError`（本 crate）都不属于用户的 crate。显式的 `conv_err()` 方法是 intended path：

```rust
let result: Result<(), StructError<UpperReason>> = lower_result.conv_err()?;
```

### Send + Sync

`DomainReason` 要求 `Send + Sync`。这是必要的——`StructError` 需要在 async 任务边界之间传递，并能被 `anyhow::Error` 或 `Box<dyn Error>` 捕获。对于单线程使用，这是一个微小但不可省略的约束。
