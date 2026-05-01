# orion-error 与 Rust 错误生态方案对比

> 对比范围：anyhow / thiserror / color-eyre / orion-error

---

## 1. 定位总览

| 维度 | anyhow | thiserror | color-eyre | **orion-error** |
|------|--------|-----------|------------|-----------------|
| **定位** | 快速错误处理 | 标准错误类型 derive | 诊断式错误报告 | **结构化错误治理框架** |
| **目标用户** | 应用开发者（快速原型） | 库作者 | 应用开发者（诊断） | **大型多团队工程** |
| **问题域** | 减少错误处理样板代码 | 减少 Error impl 样板代码 | 改善错误诊断输出 | **统一错误建模 → 运行时传播 → 边界协议投影** |
| **抽象层级** | 类型擦除 | 类型安全 enum | 类型擦除 + 诊断 | **泛型结构化载体** |

---

## 2. 核心能力对比

### 错误定义

| 能力 | anyhow | thiserror | color-eyre | orion-error |
|------|--------|-----------|------------|-------------|
| 自定义错误类型 | 不直接支持 | `#[derive(Error)]` | 不直接支持 | `#[derive(OrionError)]` |
| 泛型错误类型 | `Box<dyn Error>` | 用户定义 enum | `Box<dyn Error>` | `StructError<T: DomainReason>` |
| 稳定 Identity | 无 | 无 | 无 | `stable_code()` + `ErrorCategory` |
| 数值 ErrorCode | 无 | `#[error(...)]` 间接 | 无 | 内建 `error_code()` |
| `Display` / `source` | 自动 | 自动 | 自动 | 自动（`OrionError` derive） |

### 运行时传播

| 能力 | anyhow | thiserror | color-eyre | orion-error |
|------|--------|-----------|------------|-------------|
| 上下文附着 | `.context(...)` / `.with_context(...)` | 无 | `.sections()` / `.note()` / `.with_section()` | `OperationContext`（doing/at/path + KV + metadata） |
| Context 路径 | 单层 context 链 | 无 | 单层 | **多层嵌套 path 规整**：target_path segments |
| 自定义元数据 | 无（仅消息） | 无 | `Section` trait | `ErrorMetadata`（typed KV，不进入 Display） |
| Source 链追踪 | 标准链 | 标准链 | 标准链 + `SpanTrace` | **双通道**（Std/Struct）+ `SourceFrame` 丰富元数据 |
| 跨类型转换 | `anyhow!()` 宏 | `#[from]` | `eyre!()` 宏 | `convert_error()` + `upcast()` |

### 边界输出

| 能力 | anyhow | thiserror | color-eyre | orion-error |
|------|--------|-----------|------------|-------------|
| Human 诊断 | `.display_chain()` | 无 | `{}` 彩色输出 | `report().render()` + `RedactPolicy` |
| 协议 JSON (HTTP/RPC) | 无 | 无 | 无 | `exposure_snapshot()` → `to_http_error_json()` / `to_rpc_error_json()` / `to_cli_error_json()` / `to_log_error_json()` |
| 稳定快照 | 无 | 无 | 无 | `StableErrorSnapshot` + `schema_version` |
| 暴露策略 | 无 | 无 | 无 | `ExposurePolicy`（status/visibility/hints/retryable + 按 `stable_code` 控制） |
| 脱敏/Redaction | 无 | 无 | 支持（有限） | `RedactPolicy` trait（贯穿 report/projection/identity） |

### std::error::Error 生态

| 能力 | anyhow | thiserror | color-eyre | orion-error |
|------|--------|-----------|------------|-------------|
| 实现 `StdError` | 是 | 是 | 是 | **显式 Bridge**（`as_std()` / `into_std()` / `into_dyn_std()`） |
| `dyn Error` 兼容 | 天然 | 天然 | 天然 | 有损转换（`OwnedDynStdStructError`） |
| 与第三方错误互操作 | `.context()` / `anyhow!()` | `#[from]` | `.sections()` / `eyre!()` | `source_err()`` / `raw_source()` |

---

## 3. 与 anyhow 对比

### anyhow 的定位

anyhow 是 **快速错误处理** 工具。它的核心抽象是类型擦除：把一切错误抹平为 `Box<dyn Error>`，让调用方可以跳过繁琐的类型定义。

### orion-error 的立足点

- anyhow 的目的地是"快速抹平然后继续"；orion-error 的目的地是"带着结构化身份和上下文走到边界再输出"
- orion-error 不愿意擦除类型——`StructError<T>` 保留了 `T: DomainReason` 的静态信息
- orion-error 的 context 不是单层 String，而是多层 `OperationContext` 加结构化 path

### 各有所长的场景

| 场景 | 推荐方案 |
|------|---------|
| 快速脚本、CLI 原型 | anyhow |
| 每一层都需要精确错误身份 | orion-error |
| 对外协议需要统一 error JSON | orion-error |
| 只需要"知道出错了" | anyhow |
| 需要控制什么信息暴露给客户端 | orion-error |

---

## 4. 与 thiserror 对比

（参见 [thiserror-comparison.md](./thiserror-comparison.md)，这里只做摘要。）

- thiserror 是 **标准错误类型 derive 工具**；orion-error 是 **治理框架**
- thiserror 覆盖 `Display`/`source`/`From` 生成；orion-error 覆盖结构化身份、上下文、快照、协议投影
- 两者不互斥：thiserror 类型可以作为 source 进入 `StructError`，边界外的标准错误继续用 thiserror

---

## 5. 与 color-eyre 对比

### color-eyre 的定位

color-eyre 是 **诊断体验增强** 工具。它在 anyhow 基础上增加了：
- 彩色格式化的错误报告
- 自定义 Section 情感（`sections`, `notes`, `warnings`）
- `SpanTrace` / `Backtrace` 集成
- `"Report Handler"` 可插拔模式

### orion-error 的立足点

- color-eyre 仍然是 **错误展示层** 优化，而 orion-error 覆盖了从定义到边界输出的全链路
- color-eyre 没有 stable identity，没有 protocol projection，没有 ExposurePolicy
- color-eyre 的 Section 机制是自由格式（any type implements `Section`），orion-error 的 context 有固定的 doing/at/path 语义
- color-eyre 的 Report Handler 可以自定义输出，但仍然是人类可读格式；orion-error 面向多协议输出（human + JSON + 稳定快照）

### 各有所长的场景

| 场景 | 推荐方案 |
|------|---------|
| 终端应用需要漂亮错误输出 | color-eyre |
| 后端服务需要统一协议错误响应 | orion-error |
| 需要彩色 backtrace 和 span trace | color-eyre |
| 需要按错误身份做 Exposure 控制 | orion-error |
| 需要快照持久化 / 稳定导出 | orion-error |

---

## 6. 综合决策树

```
你的项目是多层服务/多团队工程吗？
├── 否 → 你只需要：
│   ├── 定义少量本地错误类型 → thiserror
│   ├── 快速处理错误 → anyhow
│   └── 终端展示要好 → color-eyre
└── 是 → 评估额外需求：
    ├── 错误身份需要稳定（协议/监控依赖） → orion-error
    ├── 需要统一错误 JSON 给 HTTP/RPC/CLI → orion-error
    ├── 需要脱敏/红action 策略 → orion-error
    ├── 需要快照持久化 → orion-error
    └── 以上都不需要 → anyhow + thiserror 组合够用
```

---

## 7. 共存策略

orion-error 设计上不与生态对立。推荐的分工：

| 层 | 推荐方案 |
|----|---------|
| 边界外（第三方库、FFI） | thiserror / 标准 Error trait |
| 进入结构化体系 | orion-error .source_err()` |
| 业务层传播 | orion-error `StructError<R>` |
| 跨层（repo → service → handler） | orion-error `upcast()` |
| 边界输出 | orion-error `exposure_snapshot()` |
| 快速原型 / 胶水代码 | anyhow（orion-error 提供了 feature `anyhow` 支持） |
| 终端诊断展示 | orion-error `report().render()` 或 color-eyre（非冲突） |

---

## 8. 推荐与不推荐

### 推荐 orion-error 的场景

- 多层架构的 Rust 后端服务（repo → service → handler → protocol）
- 对外提供 HTTP/RPC/gRPC 接口
- 微服务架构需要稳定错误码和监控分类
- 多团队协作，需要统一工程规范
- 需要持久化/序列化错误快照

### 不推荐 orion-error 的场景

- 单文件脚本或 CLI 工具（anyhow 更轻量）
- 底层库需要纯 `std::error::Error` 接口暴露（thiserror 更适合）
- 项目只有一两层，不需要结构化上下文追踪
