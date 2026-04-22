# Orion Error Mini RFC

更新时间：2026-04-22

对于 `0.6.x / V1 API` 的具体落地执行、修复顺序与评审标准，先以 [V1 修复与评审基线](./v1-fix-and-review-plan.md) 为准；本文档继续承担设计解释、边界说明与 `V2` 规划。

本文档给出一版面向 `orion-error` 的改进型 API 草案。

目标不是复盘现状，而是回答一个更直接的问题：

- 如果从今天重新设计，`orion-error` 应该怎样更好？

这是一份 mini RFC，因此只覆盖：

- 设计目标
- 核心数据模型
- API 草案
- 编译期 / 运行期约束策略
- 与现状的映射关系
- 迁移路线

## 1. 背景

当前 `orion-error` 的核心设计方向是对的：

- 错误必须结构化
- source 需要保真
- 已结构化错误不能再被当成普通 source 附加

但它的 API 体验存在明显问题：

1. 普通错误和结构化错误的处理入口过于相似
2. 调用者需要记住隐式规则
3. 误用有时在运行时 panic，而不是更早暴露
4. `owe_conf_source / with_source / wrap / with_struct_source` 的边界不够直观

说明：

- 本文中提到的 `owe_conf()` / `owe_conf_source()` / `owe_*()` / `owe_*_source()` 均属于历史讨论对象
- 当前代码只保留兼容态的 `owe(...)`
- `owe_*()` 与 `owe_*_source()` 已从主代码移除

因此，本 RFC 的目标不是改变理念，而是改善 API 人体工学。

## 2. 设计目标

新的 API 需要满足以下 6 个目标：

1. 普通错误和结构化错误必须走不同入口
2. 方法名必须直接表达语义
3. 误用尽量在编译期暴露
4. 上下文补充方式统一
5. `anyhow` 必须有清晰定位
6. 迁移成本必须可控

## 3. 非目标

本 RFC 不尝试：

- 重写整个 `StructError` 渲染系统
- 改造所有 reason 枚举
- 一次性替换所有旧 API
- 解决 stable Rust 上所有“负 trait bound”限制

## 4. 核心判断

错误生命周期应拆成两段，而不是混在一套 API 里：

### 4.1 第一段：原始错误进入结构化体系

示例：

- `io::Error`
- `reqwest::Error`
- `serde_json::Error`
- `git2::Error`
- `anyhow::Error`

这类错误还没有 reason/category，需要第一次被结构化。

### 4.2 第二段：结构化错误向更高层上卷

示例：

- `StructError<ConfIOReason> -> StructError<RunReason>`
- `StructError<SourceReason> -> StructError<RunReason>`
- `StructError<SinkReason> -> StructError<RunReason>`

这类错误已经有自己的结构，不能再按普通 source 附加。

当前 API 最大的问题，就是没有把这两段清晰拆开。

## 5. 数据模型草案

### 5.1 `StructError`

```rust
pub struct StructError<R> {
    reason: R,
    detail: Option<String>,
    context: Vec<ContextFrame>,
    source: Option<SourcePayload>,
}
```

### 5.2 source 类型

```rust
pub enum SourcePayload {
    Std(Box<dyn std::error::Error + Send + Sync>),
    Struct(Box<dyn StructErrorDyn>),
}
```

这表示：

- 普通 source
- 结构化 source

在模型层就是两种不同东西，而不是一个模糊的 `with_source(...)`。

### 5.3 reason 抽象

```rust
pub trait Reason: std::fmt::Debug + std::fmt::Display + Send + Sync + 'static {
    fn code(&self) -> &'static str;
    fn category(&self) -> ErrorCategory;
}

pub enum ErrorCategory {
    Conf,
    Biz,
    Logic,
    Sys,
}
```

这允许：

- 不同领域 reason 自己定义 code
- 顶层报告系统按 category 渲染

这里需要补一条边界约束：

- `Reason` 才是结构化错误的主语义载体
- `ErrorCategory` 只是报告层、统计层、治理层使用的粗粒度派生视图
- `category` 可以由 `reason` 派生，但不能反向推出完整 `reason`
- 因此 `into_as(...)` / `wrap_as(...)` 一类 API 必须显式给出 target reason，而不能只给 category

换句话说：

- reason taxonomy 不能被 category 替代
- category 只负责粗粒度分桶，不参与错误身份判定

### 5.4 runtime / snapshot / report 分层

新的模型不要求 `StructError<R>` 继续同时承担：

- 运行时错误载体
- 可克隆值对象
- 可序列化导出对象
- 测试断言快照对象

这几类职责。

更合理的长期方向是分层：

```rust
pub struct StructError<R> {
    reason: R,
    detail: Option<String>,
    context: Vec<ContextFrame>,
    source: Option<SourcePayload>,
}

pub struct StructErrorSnapshot {
    code: String,
    category: ErrorCategory,
    detail: Option<String>,
    context: Vec<ContextFrameSnapshot>,
    source: Option<Box<StructErrorSnapshot>>,
}
```

语义建议如下：

- `StructError<R>`
  - 面向运行时传播
  - 重点是错误语义、上下文、source 链、bridge 能力
- `StructErrorSnapshot`
  - 面向序列化、测试断言、报告渲染、稳定导出
  - 不要求保留原始错误对象能力

这意味着：

- runtime carrier 不必继续强行维持 `Clone / PartialEq / Serialize`
- 如需稳定输出，应显式转成 snapshot / report 层
- `StructError` 本体的设计目标应优先服务运行时错误传播，而不是导出便利性

## 6. API 草案

## 6.1 普通错误进入结构化体系：`into_as(...)`

```rust
pub trait IntoStructError<T, E> {
    fn into_as<R>(self, reason: R, detail: impl Into<String>) -> Result<T, StructError<R>>
    where
        R: Reason;
}
```

这个 trait 面向：

- `Result<T, io::Error>`
- `Result<T, reqwest::Error>`
- `Result<T, anyhow::Error>`
- 其他“非结构化错误”

语义是：

- 这是第一次进入结构化错误系统

示例：

```rust
let text = std::fs::read_to_string(path)
    .at(path)
    .doing("read engine config")
    .into_as(RunReason::ConfigError, "read engine config failed")?;
```

这里需要强调：

- `into_as(reason, detail)` 才是 core API
- `into_conf(...) / into_biz(...) / into_logic(...) / into_sys(...)` 如果存在，也应只是上层项目按固定 reason 预设出来的便捷糖衣
- `orion-error` core 不应默认假设所有项目都存在统一的 `RunReason`

如果项目层希望保留更短写法，可以把它定义成糖衣：

```rust
read_to_string(path)
    .at(path)
    .doing("read engine config")
    .into_conf("read engine config failed")?;
```

其中：

- `into_conf(...) == into_as(RunReason::ConfigError, ...)`
- 这类别名更适合放在应用层或领域层扩展 trait，而不是 `orion-error` core 的唯一主路径

## 6.2 结构化错误上卷：`wrap_as(...)`

```rust
pub trait WrapStructError<T, R1> {
    fn wrap_as<R2>(self, reason: R2, detail: impl Into<String>) -> Result<T, StructError<R2>>
    where
        R2: Reason;
}
```

这个 trait 面向：

- `Result<T, StructError<R>>`

语义是：

- 当前错误已经结构化
- 现在只是向更高层 reason/category 收口

示例：

```rust
let report = wp_cli_core::knowdb::check(work_root, dict)
    .at(work_root)
    .doing("check knowledge base")
    .wrap_as(RunReason::KnowledgeCheckFailed, "知识库检查失败")?;
```

这里需要强调：

- `wrap_as(reason, detail)` 才是核心 API
- `wrap_conf(...) / wrap_biz(...) / wrap_logic(...) / wrap_sys(...)` 如果存在，也应只是上层项目按固定 reason 预设出来的便捷糖衣
- `orion-error` core 不应依赖“只给 category，不给具体 reason”这种隐式规则来构造目标错误

换句话说：

- category 可以帮助渲染
- 但真正决定错误语义的仍然是显式的 target reason

## 6.3 显式 source API

```rust
impl<R: Reason> StructError<R> {
    pub fn with_std_source<E>(self, err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static;

    pub fn with_struct_source<R2>(self, err: StructError<R2>) -> Self
    where
        R2: Reason;
}
```

这个设计的重要意义是：

- 不再保留语义模糊的 `with_source(...)`
- 让调用者从方法名上知道自己在做什么

## 6.4 上下文 builder：`at(...)` / `doing(...)`

```rust
pub trait ErrorContext {
    fn at(self, target: impl Into<TargetRef>) -> Self;
    fn doing(self, action: impl Into<String>) -> Self;
    fn tag(self, key: &'static str, value: impl Into<String>) -> Self;
}
```

建议：

- `attach_context(...)` 取代 error-side 的通用 `with(...)`
- `at(...)` 只用于明确的 locator / resource 引用
- `doing(...)` 替代大部分 `want(...)`

示例：

```rust
EngineConfig::load(root, dict)
    .at(root)
    .doing("load engine config")
    .wrap_conf("load engine config failed")?;
```

## 6.5 构造器风格 API

为不使用 trait 风格的场景，建议同时提供 builder：

```rust
StructError::conf(RunReason::ConfigError)
    .detail("load engine config failed")
    .at(path)
    .doing("read config")
    .with_std_source(err)
```

以及：

```rust
StructError::conf(RunReason::ConfigError)
    .detail("knowledge check failed")
    .with_struct_source(err)
```

## 6.6 上下文动作命名建议

当前讨论里，一个很实际的问题是：

- 为什么不继续使用 `want(...)`？

结论是：

- `want(...)` 不是不能用
- 但它不适合作为大型工程错误上下文里的主命名

原因主要有 4 个。

### 6.6.1 `want(...)` 的语义不稳定

`want(...)` 表达的是“主观意图”，而错误上下文更需要“客观动作”。

例如：

- `want("load config")`

它可能被理解成：

- 我想要加载配置
- 本次调用的目标是加载配置
- 期望配置被成功加载

而错误系统真正需要记录的，通常是：

- 当前正在做什么

所以：

- `doing("load config")`
- `while_doing("load config")`

会比 `want("load config")` 更稳。

### 6.6.2 `want(...)` 容易和“期望/断言”语义混淆

在大型工程里，“期望”本来就是一个常见语义：

- expected vs actual
- invariant
- precondition
- validation expectation

因此 `want(...)` 很容易和下面这些概念串味：

- 用户期望
- 业务期望
- 校验条件
- 断言失败

这会导致 review 时需要额外猜测：

- 这里记录的是“动作现场”
- 还是“逻辑期望”

### 6.6.3 `want(...)` 不利于统一错误句式

错误上下文最终往往会被格式化成一句固定句式。

例如：

- `at /tmp/a.toml, doing read config`
- `at project root, doing load engine config`

如果改成 `want(...)`，句式会明显变弱：

- `at /tmp/a.toml, want read config`
- `at project root, want load engine config`

读起来不像错误现场，更像需求描述。

### 6.6.4 `want(...)` 会让成功路径和失败路径的语义边界变漂

`want(...)` 有时像：

- 本来打算做什么

有时又像：

- 期望系统达到什么状态

例如下面几种写法：

- `want("load config")`
- `want("config should be valid")`
- `want("find project root")`

它们其实分别在表达：

- 动作
- 断言
- 目标

一个方法名承载三类语义，不利于长期收敛。

### 6.6.5 候选命名对比

#### `doing(...)`

优点：

- 语义直接
- 能准确表达“当前正在做什么”
- 和 `at(...)` 组合自然
- 适合作为默认公共 API

例子：

```rust
at(path).doing("read config")
at(root).doing("load engine config")
```

缺点：

- 稍微偏口语

总体判断：

- 这是最适合做默认主命名的方案

#### `while_doing(...)`

优点：

- 语义最完整
- 明确表达“错误发生在某动作执行期间”
- 歧义最少

例子：

```rust
at(path).while_doing("read config")
```

缺点：

- 名字偏长
- 高频调用场景略显啰嗦

总体判断：

- 如果优先追求清晰度，它甚至比 `doing(...)` 更强

#### `action(...)`

优点：

- 中性
- 偏抽象，适合框架层

缺点：

- 不够自然
- 更像“设置字段”，不像“补上下文”

总体判断：

- 可用，但不如 `doing(...)` 自然

#### `trying(...)`

优点：

- 能表达“失败发生在尝试阶段”
- 有一定自然语言感

缺点：

- 容易弱化动作事实
- 让上下文显得像“尚未真正执行”

总体判断：

- 适合少数场景，不适合做主命名

#### `want(...)`

优点：

- 很短

缺点：

- 歧义最大
- 容易和期望/断言混淆
- 不像错误现场描述

总体判断：

- 不建议继续作为主命名

### 6.6.6 推荐结论

本 RFC 建议：

1. 主命名使用 `doing(...)`
2. 如确实需要更完整的句式，可额外提供 `while_doing(...)`
3. `want(...)` 保留兼容期，但进入 deprecated path

因此：

- `with(...)` 更适合作为对象/位置上下文
- `doing(...)` 更适合作为动作上下文

两者组合后，错误现场会更稳定，也更容易统一渲染。

## 7. `anyhow` 的定位

`anyhow::Error` 既不是：

- 普通标准库错误

也不是：

- 已结构化错误

它属于：

- “已聚合但未结构化”的错误

所以本 RFC 认为 `anyhow` 应该被视作 `into_as(...)` 家族的一部分，而不是 `wrap_*` 家族。

即：

```rust
fn into_as<R>(self, reason: R, detail: impl Into<String>) -> Result<T, StructError<R>>
where
    R: Reason;
```

内部允许专门为 `anyhow::Error` 提供实现。

但这里需要补充一条严格规则：

- 默认把 `anyhow::Error` 当作未结构化错误处理
- 后续版本可以考虑优先识别并提取 `orion-error` 官方 bridge 类型
- 不做泛化的 source 链扫描
- 不猜第三方 wrapper
- 不尝试从任意 `anyhow` 链里“魔法恢复”结构化错误

也就是说：

- `anyhow` 默认进入 `into_as(...)`
- 如果未来引入 `OwnedStdStructError<R>` 这类官方 bridge，才考虑把它作为“已结构化错误桥接体”处理

因此，不建议继续让 `anyhow` 参与：

- `owe_conf_source()`
- `with_struct_source(...)`

更准确地说：

- `anyhow` 本身不是结构化错误
- `anyhow` 可以携带官方 bridge
- bridge 是否可提取，应由显式 `downcast` 规则决定，而不是由链式猜测决定

## 8. 误用防护策略

理想目标是：

- 编译期禁止误用

但 stable Rust 对负约束支持有限，因此本 RFC 推荐分层处理：

### 8.1 第一优先级：通过 API 设计减少误用

- 普通错误只走 `into_*`
- 结构化错误只走 `wrap_as(...)`
- source 分成 `with_std_source` 和 `with_struct_source`

### 8.2 第二优先级：通过 trait 实现收紧边界

尽量避免给过宽的泛型 `Result<T, E>` 暴露同名入口。

对于 V1，约束应进一步明确成：

- 不为 `Result<T, E> where E: StdError` 提供 `into_as(...)` 的 blanket impl
- 否则在 `StructError: StdError` 仍保留时，`into_as(...)` 一定会误吞 `StructError<_>`
- `into_as(...)` 只对显式允许的“未结构化错误”类型开放
- `wrap_as(...)` 只对 `Result<T, StructError<_>>` 开放

更具体地说：

- V1 应引入一个封闭的未结构化错误入口，例如库内 `UnstructuredSource`
- 只给明确的 raw error 类型实现，例如 `std::io::Error`、`anyhow::Error`、`serde_json::Error`
- `StructError<_>` 永远不实现这个入口 trait
- 对未知第三方 raw error，如需进入 `into_as(...)`，应要求显式 opt-in 包装，而不是重新开放 `E: StdError` 的大 blanket

V1 的正确解法可以进一步写死为：

- `IntoAs` 只对封闭的 `UnstructuredSource` 开放
- 库内 allowlist 类型直接实现 `UnstructuredSource`
- `raw_source(...)` 只接受 `E: RawStdError`
- `RawStdError` 是公开 marker trait，不提供 blanket impl
- 下游只能为“自己的本地 raw StdError 类型”实现 `RawStdError`
- 下游不能为 `StructError<_>` 实现 `RawStdError`
  - 因为 `StructError<_>` 是外部类型
  - 这正是利用 orphan rule 保住 V1 边界
- `RawSource<E>` 再由库内实现 `UnstructuredSource`

这套组合满足 V1 的三个同时约束：

- 不给 `E: StdError` 开 blanket
- 不误吞 `StructError<_>`
- 未知第三方 raw error 仍保留显式 opt-in 逃生门

这一点决定了 V1 是否真的“兼容但不继续埋坑”：

- 牺牲一部分 blanket convenience
- 换取结构化错误不再被误当普通 source 重新结构化

### 8.3 第三优先级：保留运行时保护

如果仍然发生误用，可以保留 panic 或 debug assert 作为最后防线。

但运行时 panic 不应成为主要交互方式。

## 9. `StructError` 保留/退出 `StdError` 的架构对比

这是一个必须单独讨论的设计点。

问题不是“能不能做”，而是“值不值得为了更纯的 API 边界，付出生态兼容成本”。

结论先行：

- 技术上完全可行
- 纯设计上更干净
- 但它是一次明确的破坏性取舍

### 9.1 为什么退出 `StdError` 会更干净

当前 `StructError` 同时扮演两种角色：

1. 结构化领域错误载体
2. 标准错误生态中的普通 `StdError`

这会导致一个根本问题：

- 在类型系统里，它既像“特殊的结构化错误”
- 又像“普通 source 错误”

于是：

- `with_source(...)`
- `with_struct_source(...)`
- `wrap(...)`

这些 API 很难彻底分流。

如果 `StructError` 退出 `StdError`，那么：

- 普通错误和结构化错误的 trait 实现天然分开
- 自动分流会简单很多
- 误用更容易在编译期暴露

### 9.2 为什么代价很高

Rust 生态默认围绕 `std::error::Error` 建工具。

一旦 `StructError` 不再实现 `StdError`，会影响：

1. 与 `anyhow` / `eyre` / 通用报告系统的直接兼容
2. 很多只接受 `StdError` 的第三方接口
3. 使用 `source()` 递归遍历错误链的现有工具
4. 用户对“它也是个普通 error”的既有心智

因此，这不是小修，而是大版本架构调整。

### 9.3 三种方案

#### 方案 A：继续保留 `StructError: StdError`

设计：

- `StructError` 继续实现 `std::error::Error`
- 通过新 API 强化边界：
  - `into_*`
  - `wrap_as(...)`
  - `with_std_source`
  - `with_struct_source`

优点：

- 生态兼容性最好
- 对现有用户最温和
- 迁移成本最低

缺点：

- 类型层仍然不够纯
- 很多误用仍要靠 API 约束和运行时保护兜底
- 自动分流实现仍然别扭

#### 方案 B：`StructError` 退出 `StdError`

设计：

- `StructError` 只作为结构化领域错误
- 普通 `StdError` 和 `StructError` 彻底分流

优点：

- 设计最纯
- trait 边界最清晰
- 自动分流最自然
- 最接近“一个公开入口，内部自动处理两类 source”的目标

缺点：

- 生态兼容性下降明显
- 会打破很多现有泛型约束
- 迁移成本最高

#### 方案 C：双层模型

设计：

- 核心层：
  - `StructError<R>` 不实现 `StdError`
- 桥接层：
  - `OwnedStdStructError<R>` 或类似 wrapper 实现 `StdError`

优点：

- 内部设计保持纯净
- 外部仍保留标准生态桥接能力
- 可以兼顾长期架构和短期兼容

缺点：

- 类型数量增加
- 实现复杂度最高
- 文档和用户教育成本更高

### 9.4 对比表

| 方案 | 设计纯度 | 生态兼容 | 迁移成本 | 自动分流可实现性 | 推荐度 |
|---|---:|---:|---:|---:|---:|
| A. 保留 `StdError` | 中 | 高 | 低 | 中 | 高 |
| B. 退出 `StdError` | 高 | 低 | 高 | 高 | 中 |
| C. 双层模型 | 高 | 中高 | 中高 | 高 | 很高 |

### 9.5 推荐路线

如果作者希望控制风险，本 RFC 推荐：

1. 短期先走方案 A
   - 不动 `StructError: StdError`
   - 先把 API 分流做好
2. 中期演进到方案 C
   - 保持内部纯设计
   - 同时保留桥接层
3. 不建议直接跳到方案 B
   - 除非愿意承担明确的大版本 break

一句话总结：

> 从纯设计角度看，退出 `StdError` 更优雅；从工程演进角度看，先把 API 分流做好更现实。

### 9.6 方案 C 的具体类型草案

如果真的要把方案 C 做成长期形态，建议把“结构化错误本体”和“标准错误桥接体”拆开。

核心层：

```rust
pub struct StructError<R> {
    reason: R,
    detail: Option<String>,
    context: Vec<ContextFrame>,
    source: Option<SourcePayload>,
}

pub enum SourcePayload {
    Std(Box<dyn std::error::Error + Send + Sync>),
    Struct(Box<dyn StructChainDyn>),
}

pub trait StructChainDyn: Send + Sync {
    fn reason_code(&self) -> &'static str;
    fn display_text(&self) -> String;
    fn next(&self) -> Option<&dyn StructChainDyn>;
}
```

桥接层：

```rust
pub struct OwnedStdStructError<R> {
    inner: StructError<R>,
}

pub struct StdStructRef<'a, R> {
    inner: &'a StructError<R>,
}
```

这里的边界是：

- `StructError<R>` 是领域内的“真错误”
- `OwnedStdStructError<R>` 是给 `anyhow` / 第三方生态用的桥
- `StdStructRef<'a, R>` 是只读借用桥，适合只需要 `Display + source()` 的场景

这样可以把“结构化错误”和“标准错误兼容层”从类型层彻底分开。

### 9.7 一个自动入口在双层模型下如何成立

用户前面提到的诉求是对的：

- 如果能只保留一个方法，自动处理结构化错误和非结构化错误，会更顺手

在当前模型里，这件事不够优雅，根本原因是：

- `StructError` 自己也是 `StdError`
- 因此“普通错误 blanket impl”和“结构化错误专用 impl”天然重叠

但在方案 C 里，这个障碍会消失。

可以直接定义：

```rust
pub trait IntoSourcePayload {
    fn into_source_payload(self) -> SourcePayload;
}

impl<E> IntoSourcePayload for E
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn into_source_payload(self) -> SourcePayload {
        SourcePayload::Std(Box::new(self))
    }
}

impl<R> IntoSourcePayload for StructError<R>
where
    R: Reason,
{
    fn into_source_payload(self) -> SourcePayload {
        SourcePayload::Struct(Box::new(self))
    }
}
```

然后给 `StructError` 一个统一入口：

```rust
impl<R: Reason> StructError<R> {
    pub fn attach_source<S>(mut self, src: S) -> Self
    where
        S: IntoSourcePayload,
    {
        self.source = Some(src.into_source_payload());
        self
    }
}
```

于是调用侧就可以真正写成：

```rust
StructError::conf(RunReason::ConfigError)
    .detail("load config failed")
    .attach_source(err)
```

这里的 `err` 可以是：

- `io::Error`
- `anyhow::Error`
- `StructError<ConfReason>`

而 API 仍然没有歧义。

这正是方案 C 最大的优雅点：

- 自动分流不是靠运行时猜
- 也不是靠方法名堆叠
- 而是靠类型系统自然完成

### 9.8 双层模型的桥接 API

如果 `StructError<R>` 不再实现 `StdError`，那就需要把“接入标准生态”的姿势设计完整。

建议至少提供这组桥接 API：

```rust
impl<R: Reason> StructError<R> {
    pub fn into_std(self) -> OwnedStdStructError<R>;
    pub fn as_std(&self) -> StdStructRef<'_, R>;
}

impl<R: Reason> std::fmt::Display for OwnedStdStructError<R> { ... }
impl<R: Reason> std::error::Error for OwnedStdStructError<R> { ... }

impl<'a, R: Reason> std::fmt::Display for StdStructRef<'a, R> { ... }
impl<'a, R: Reason> std::error::Error for StdStructRef<'a, R> { ... }
```

建议语义如下：

- `into_std()`
  - 把结构化错误转成可移动的标准错误对象
  - 用于 `anyhow::Error::new(...)`、第三方接口返回值、线程边界传递
- `as_std()`
  - 提供临时借用视图
  - 用于只想给现有报告系统读取 `Display` 和有限 bridge 信息的场景

还可以补一层互转：

```rust
impl<R: Reason> From<StructError<R>> for OwnedStdStructError<R> { ... }

impl<R: Reason> OwnedStdStructError<R> {
    pub fn into_struct(self) -> StructError<R>;
}
```

这样可以保证：

- 领域代码内部一直操作 `StructError<R>`
- 只有在必须进入 `StdError` 生态时才显式桥接

这会显著降低“明明是结构化错误，却被误当普通 source 再挂一次”的概率。

需要明确的是：

- 这里的 bridge 目标是“显式接入标准错误生态”
- 不是“在所有现有 `source()` 工具里完美复刻原始结构化链的全部行为”
- 尤其是借用桥 `StdStructRef<'_, R>`，更适合作为边界适配器，而不是长期持有的通用错误节点

因此：

- V2 的设计目标应是可解释、可桥接、边界清晰
- 不应承诺通过 bridge 完全复刻 runtime `StructError` 的内部表示

### 9.9 方案 C 下的迁移步骤

如果未来要从现状迁到双层模型，建议按下面 4 步走，而不是一次性硬切。

第 1 步：先把 API 语义分流

- 保留当前 `StructError: StdError`
- 先引入：
  - `into_*`
  - `wrap_*`
  - `with_std_source`
  - `with_struct_source`
- 让业务代码先停止依赖模糊入口

第 2 步：内部先改成双 source 模型

- 内部 source 存储改成：
  - `SourcePayload::Std`
  - `SourcePayload::Struct`
- 即使外部 API 暂不 break，内部也先完成语义分层

第 3 步：引入桥接类型，但暂不移除旧实现

- 增加：
  - `OwnedStdStructError<R>`
  - `StdStructRef<'_, R>`
  - `into_std()`
  - `as_std()`
- 文档开始推荐：
  - 领域内传 `StructError<R>`
  - 出边界时再桥接为 `StdError`

第 4 步：下一个大版本移除 `StructError: StdError`

- 删除 `impl StdError for StructError<R>`
- 把所有需要标准错误兼容的位置改成显式桥接
- 此时就可以把统一的 `attach_source(...)` 正式公开为主入口

这个迁移顺序的价值在于：

- 先改调用者心智，再改底层类型关系
- 先把“该怎么用”收敛，再做破坏性升级
- 可以把 break change 控制在一个明确的大版本里

### 9.10 结论：方案 C 才真正支持“一个方法自动处理两类错误”

因此，对“能不能只有一个方法自动处理结构化错误和非结构化错误”这个问题，答案是：

- 在当前模型下，可以做，但会别扭，也很难彻底避免重叠
- 在方案 C 下，这件事是自然成立的

也就是说：

- 如果只想小步优化，优先做 API 分流
- 如果追求长期优雅性，最终还是应走双层模型

这也是为什么本 RFC 把方案 C 评为长期最值得投入的方向。

## 10. 推荐调用风格

### 10.1 普通错误

```rust
let file = std::fs::File::open(path)
    .at(path)
    .doing("open sample file")
    .into_as(RunReason::ConfigError, "open sample file failed")?;
```

如果项目层提供糖衣，也可以写成：

```rust
let file = std::fs::File::open(path)
    .at(path)
    .doing("open sample file")
    .into_conf("open sample file failed")?;
```

### 10.2 结构化错误上卷

```rust
let conf = EngineConfig::load(root, dict)
    .at(root)
    .doing("load engine config")
    .wrap_as(RunReason::ConfigError, "load engine config failed")?;
```

如果项目层提供糖衣，也可以写成：

```rust
let conf = EngineConfig::load(root, dict)
    .at(root)
    .doing("load engine config")
    .wrap_conf("load engine config failed")?;
```

### 10.3 业务冲突

```rust
return Err(
    StructError::biz(RunReason::ReloadConflict)
        .detail("reload already in progress")
        .at("reload")
        .doing("start reload")
);
```

### 10.4 非 `StdError` parser 错误

```rust
parser.parse(data).map_err(|err| {
    StructError::conf(RunReason::ConfigError)
        .detail(format!("invalid syntax: {}", err))
        .at(file)
        .doing("parse config")
})?;
```

## 11. 与当前 API 的映射

建议映射关系如下：

| 当前 API | 建议新 API | 语义 |
|---|---|---|
| `owe_conf_source()` | `into_as(...)` | 普通错误第一次结构化 |
| `owe_conf()` | `into_as(...)` 或 `wrap_as(...)` | 视上游类型而定 |
| `err_conv()` | 保留独立语义，或映射到未来显式 `conv_as(...)` | 结构化错误的 reason 类型转换，保留原结构 |
| `err_wrap(...)` | `wrap_as(...)` | 结构化错误上卷并建立新的外层语义边界 |
| `with_source(...)` | `with_std_source(...)` / `with_struct_source(...)` | 显式区分 source 类型 |
| `with(...)` | 需按语义拆分迁移，不应机械替换 | 当前位置/键值上下文/上下文挂接当前混用 |
| `want(...)` | `doing(...)` | 操作意图 |

## 12. 可直接由 `orion-error` 吃掉的繁琐样板

如果站在大型工程的角度看，`orion-error` 还可以继续吃掉一批高频重复样板。

这些能力的价值不只是“少写几行”，而是：

- 统一错误入口
- 降低 review 成本
- 减少误用机会
- 让调用侧更接近“声明意图”，而不是“手搓装配错误对象”

### 12.1 `with(...).want(...).owe_*()` 三段式

当前很多错误入口都会写成：

```rust
read_to_string(path)
    .with(path)
    .want("read config")
    .owe_conf_source()?;
```

这类写法的问题是：

- 信息分散
- 命名不稳定
- 调用侧要记住太多步骤

更好的方向是统一成：

```rust
read_to_string(path)
    .at(path)
    .doing("read config")
    .into_as(RunReason::ConfigError, "read config failed")?;
```

如果项目层提供糖衣，也可以写成：

```rust
read_to_string(path)
    .at(path)
    .doing("read config")
    .into_conf("read config failed")?;
```

如果继续做快捷封装，还可以支持：

```rust
read_to_string(path).conf_read(path)?;
```

也就是一次性收掉：

- 位置
- 动作
- 分类
- source 挂接

### 12.2 `map_err(|e| ...)` 的闭包样板

当前大量“错误上卷”代码都在手写闭包：

```rust
foo().map_err(|e| {
    StructError::conf(RunReason::ConfigError)
        .detail("load config failed")
        .with_struct_source(e)
})?;
```

这类写法在大型工程里会重复无数次。

更适合的收敛方式是：

```rust
foo().wrap_as(RunReason::ConfigError, "load config failed")?;
```

或者：

```rust
foo().wrap_conf("load config failed")?;
```

其中：

- `wrap_as(...)` 是核心能力
- `wrap_conf(...)` 是项目级糖衣

目标是让“结构化错误上卷”不再依赖手写闭包。

### 12.3 `anyhow` 边界映射过于手工

目前 `anyhow::Result<T>` 在很多边界上都需要人工判断：

- 能不能直接 `?`
- 应不应该 `owe_conf_source()`
- 还是要改成 `map_err(...)`

这类决策不该留给业务代码反复思考。

建议直接提供面向 `anyhow` 的标准入口：

```rust
foo_anyhow().into_conf("load config failed")?;
foo_anyhow().into_run(RunReason::ConfigError, "load config failed")?;
```

更建议 core 层统一成：

```rust
foo_anyhow().into_as(RunReason::ConfigError, "load config failed")?;
```

这样 `anyhow` 会被稳定视为：

- 已聚合但未结构化错误

除非：

- 它内部明确包裹了 `orion-error` 官方 bridge 类型

而不是落在一块模糊地带。

### 12.4 source 附加动作仍然太依赖调用者判断

当前调用者经常要自己分辨：

- 这是普通错误
- 这是结构化错误
- 应该 `with_source`
- 还是 `with_struct_source`
- 还是根本不该 attach，而应该 `wrap`

这对调用者负担太重。

如果未来演进到双层模型，建议直接提供统一入口：

```rust
err.attach_source(src)
```

其中 `src` 可以是：

- `io::Error`
- `anyhow::Error`
- `StructError<_>`

自动分流应由类型系统完成，而不是由用户记规则。

但这条统一入口的前提是：

- `StructError<R>` 已退出 `StdError`
- 结构化错误进入标准生态时必须经过官方 bridge
- 自动分流只覆盖“普通标准错误 / 结构化错误本体”两类清晰输入

### 12.5 常见 I/O / parse / command 场景应内建快捷 helper

在大型工程里，高频错误模式是非常固定的。

例如：

- 读文件
- 写文件
- 创建目录
- 解析 TOML
- 解析 JSON
- 执行子命令
- 发 HTTP 请求

这些都适合直接给快捷 helper。

例如：

```rust
read_to_string(path).conf_read(path)?;
write(path, body).conf_write(path)?;
toml::from_str(text).conf_parse(path, "toml")?;
cmd.output().sys_cmd("git status")?;
```

这样可以显著减少重复样板，也更容易保证错误上下文一致。

### 12.6 detail 文本重复

当前很多调用会同时写动作和 detail：

```rust
.at(path)
.doing("read config")
.into_as(RunReason::ConfigError, "read config failed")?;
```

其中：

- `doing("read config")`
- `into_conf("read config failed")`

表达高度重复。

因此建议支持自动 detail 派生，例如：

```rust
.at(path)
.doing("read config")
.into_auto(RunReason::ConfigError)?;
```

或者：

```rust
.at(path)
.into_with_action(RunReason::ConfigError, "read config")?;
```

即：

- 动作用于上下文
- 默认错误消息从动作自动派生

这样能显著减少文案重复。

### 12.7 reason 构造入口偏长

当前显式构造错误通常写成：

```rust
StructError::conf(RunReason::ConfigError)
```

这没有错，但在高频 `return Err(...)` 场景下偏长。

可以考虑提供更短的 reason 构造入口，例如：

```rust
RunReason::ConfigError.err()
RunReason::ConfigError.error()
err_conf(RunReason::ConfigError)
```

这样更适合高频业务分支直接返回结构化错误。

### 12.8 顶层打印 / 退出逻辑仍然分散

很多应用层项目都会重复写：

- 顶层错误收口
- CLI 打印
- 退出码决定

例如：

- `err_conv`
- `print_run_error`
- `main() -> ExitCode`

这些逻辑其实也适合由 `orion-error` 提供统一 helper，例如：

```rust
orion_error::report(run())
orion_error::exit_with(run())
orion_error::main(run())
```

让应用层只关心：

- 业务执行

而不是反复包一层报告壳。

### 12.9 测试辅助能力不足

当前测试里经常只能写：

```rust
assert!(err.to_string().contains("parser-abort"));
```

这类断言很脆弱。

更好的支持方式是提供结构化测试接口，例如：

- `err.code()`
- `err.category()`
- `err.context_contains("read config")`
- `err.source_contains("parser-abort")`

这样测试会更稳定，也能减少对字符串格式的耦合。

### 12.10 迁移辅助不能只靠文档

如果未来真的要推动 API 收敛，仅靠 RFC 和文档不够。

建议同步提供：

- deprecated 提示里的替代写法
- lint：检测 `with_source(StructError<_>)`
- lint：检测对 `StructError<_>` 再做 `owe_*_source()`
- codemod 或批量替换脚本

对大型工程来说，这些迁移辅助和 API 本身同样重要。

### 12.11 优先级建议

如果只选最值得优先做的 5 项，本 RFC 建议：

1. 正式化 `into_as(...) / wrap_as(...)`
2. 提供统一 `attach_source(...)` 方向
3. 为高频 I/O / parse / command 场景提供快捷 helper
4. 支持从 `doing(...)` 自动派生默认 detail
5. 提供测试与迁移辅助

一句话总结：

> 更好的 `orion-error` 不只是“能表达复杂错误”，还应该主动吃掉那些所有项目都会重复写的错误样板。

## 13. 迁移策略

本 RFC 不建议一次性重写现有 API。

推荐分三步：

### 阶段 1：增加新 API，不删旧 API

- 引入 `into_*`
- 或更明确地说：引入 `into_as(...)`
- 引入 `wrap_as(...)`
- 引入 `with_std_source` / `with_struct_source`
- 文档明确旧 API 进入 deprecated path

### 阶段 2：新代码只允许新 API

- lint / review 约束：
  - 不新增 `with_source(...)`
  - 不新增 `owe_conf_source()` 用于结构化错误
  - 不新增“只给 category 不给具体 reason”的上卷入口

### 阶段 3：逐步收旧 API

- 把旧 API 标记为 deprecated
- 保留足够长的兼容期

## 14. 两版本演进路线

为了避免把“短期可落地的兼容优化”和“长期更纯净的类型重构”混在一起，本 RFC 建议把 `orion-error` 的改进明确拆成两个版本。

### 14.1 V1：兼容版

V1 的目标不是追求类型模型最优雅，而是：

- 先把错误语义收敛
- 先把误用明显减少
- 先让大型工程可以平滑迁移

建议 V1 包含以下内容：

- 新增 `into_*`
- 或更明确地说：新增 `into_as(...)`
- 新增 `wrap_as(...)`
- 新增 `with_std_source`
- 新增 `with_struct_source`
- 新增 `at(...)`
- 新增 `doing(...)`
- 明确 V1 的 `into_as(...)` 边界：
  - 不对 `E: StdError` 提供 blanket impl
  - 只对显式允许的未结构化错误类型开放
  - `StructError<_>` 不能进入 `into_as(...)`
  - `Result<T, StructError<_>>` 只能走 `wrap_as(...)` / `err_conv()`
- 明确 `anyhow` 的定位：
  - 默认作为“已聚合但未结构化错误”进入 `into_as(...)`
  - 官方 bridge 识别作为后续增强项，不属于当前 V1 已实现能力
  - 不做泛化链扫描
- 文档与示例全部切到新风格
- 旧 API 保留，但进入 deprecated path
- 增加 lint / review 规则 / 迁移提示
- 内部 source 模型先完成二分：
  - `Std`
  - `Struct`
- 不要求 runtime `StructError` 继续扩张 `Clone / PartialEq / Serialize` 语义
- 开始明确 snapshot / report 是独立层，而不是 `StructError` 本体职责

V1 明确不做的事：

- 不移除 `StructError: StdError`
- 不强推统一 `attach_source(...)`
- 不做大范围 break change
- 不要求现有用户立即改完

因此，V1 的本质是：

- API 分流版

它解决的是：

- 现在怎么把错误写对
- 怎么减少 `with_source(...)` / `owe_*_source()` 的误用

而不是：

- 从根上重做类型边界

### 14.1.1 V1 最小落地清单

如果目标是先做一版能发版、能迁移、能在真实项目里推广的新 API，建议 V1 最小范围按下面顺序落地：

1. 数据模型先完成 source 二分
   - `SourcePayload::Std`
   - `SourcePayload::Struct`
   - 先把内部语义分开，再谈公开 API 收敛
2. 增加两个核心入口
   - `into_as(reason, detail)`
   - `wrap_as(reason, detail)`
3. 增加显式 source API
   - `with_std_source(...)`
   - `with_struct_source(...)`
4. 增加上下文新命名
   - `at(...)`
   - `doing(...)`
   - `want(...)` 保留但进入 deprecated path
5. 明确 `into_as(...)` 的封闭实现策略
   - 不对 `E: StdError` 提供 blanket impl
   - 只对显式允许的未结构化错误类型开放
   - `StructError<_>` 永远不能进入 `into_as(...)`
   - 未知第三方 raw error 需要显式 opt-in 包装
6. 明确 `anyhow` bridge 规则
   - 默认按未结构化错误进入 `into_as(...)`
   - 当前 V1 只保证不做链扫描
   - 官方 bridge 识别留待后续版本补齐
7. 保留旧 API，但补上迁移提示
   - `owe_*`
   - `err_conv()`
   - `with_source(...)`
   - `want(...)`
8. 文档、示例、测试全部切到新主路径
   - 示例优先使用 `into_as(...)`
   - 项目层糖衣示例可以保留，但必须标注“非 core 主路径”
9. 增加最小迁移辅助
   - deprecated note
   - review checklist
   - lint 或脚本检测典型误用

如果 V1 需要再进一步压缩范围，最不该删除的 4 项是：

1. `into_as(...)`
2. `wrap_as(...)`
3. `with_std_source(...) / with_struct_source(...)`
4. `into_as(...)` 的封闭实现边界

### 14.2 V2：破坏性更新版

V2 的目标是把长期设计真正做干净。

建议 V2 包含以下内容：

- `StructError<R>` 退出 `StdError`
- 引入双层模型：
  - `StructError<R>`
  - `OwnedStdStructError<R>`
  - `StdStructRef<'_, R>`
- 正式引入 snapshot / report 层：
  - `StructErrorSnapshot`
  - `ErrorReport`
- 正式公开统一 `attach_source(...)`
- 让 source 自动分流成为主路径
- 删除或彻底废弃模糊 API：
  - `with_source(...)`
  - `want(...)`
  - 旧式泛化 `owe_*_source()` 语义
- 标准生态兼容全部通过桥接层完成

因此，V2 的本质是：

- 类型模型重构版

它解决的是：

- 为什么一个方法自动处理两类 source 在当前模型下不够优雅
- 如何把结构化错误和标准错误桥接彻底分层

### 14.2.1 V2 implementation plan

如果决定启动 V2，建议不要直接从旧代码上零散加 patch，而是按下面顺序明确落地。

#### A. 先冻结新的核心类型边界

V2 的第一步不是改 API 名字，而是先冻结下面这组类型关系：

```rust
pub struct StructError<R> {
    reason: R,
    detail: Option<String>,
    context: Vec<ContextFrame>,
    source: Option<SourcePayload>,
}

pub enum SourcePayload {
    Std(Box<dyn std::error::Error + Send + Sync>),
    Struct(Box<dyn StructChainDyn>),
}

pub struct OwnedStdStructError<R> {
    inner: StructError<R>,
}

pub struct StdStructRef<'a, R> {
    inner: &'a StructError<R>,
}
```

这一步需要明确三条硬规则：

- `StructError<R>` 不再实现 `StdError`
- 标准生态兼容只通过 `OwnedStdStructError<R>` / `StdStructRef<'_, R>` 完成
- source 内部从一开始就区分 `Std` 和 `Struct`

也就是说，V2 必须先从类型边界上完成“结构化错误”和“标准错误桥接体”的分流。

#### B. 再冻结 bridge API

核心类型边界确定后，再定义桥接 API：

```rust
impl<R> StructError<R> {
    pub fn into_std(self) -> OwnedStdStructError<R>;
    pub fn as_std(&self) -> StdStructRef<'_, R>;
}

impl<R> From<StructError<R>> for OwnedStdStructError<R> { ... }

impl<R> OwnedStdStructError<R> {
    pub fn into_struct(self) -> StructError<R>;
}
```

bridge API 的目标应限制在：

- 显式进入 `StdError` 生态
- 显式从 bridge 回到结构化错误

而不是：

- 承诺所有第三方 `source()` 工具都能无损复刻原始 `StructError` 内部表示

因此，bridge 层要追求：

- 边界清晰
- 行为可解释
- 兼容足够好

而不是追求“看起来像完全没拆层”。

#### C. 然后公开统一 `attach_source(...)`

只有在 `StructError<R>` 已经退出 `StdError` 之后，统一入口才是类型上自然成立的：

```rust
pub trait IntoSourcePayload {
    fn into_source_payload(self) -> SourcePayload;
}

impl<E> IntoSourcePayload for E
where
    E: std::error::Error + Send + Sync + 'static,
{ ... }

impl<R> IntoSourcePayload for StructError<R>
where
    R: Reason,
{ ... }

impl<R> StructError<R> {
    pub fn attach_source<S>(self, src: S) -> Self
    where
        S: IntoSourcePayload;
}
```

V2 里这组 API 才能成为公开主路径，因为此时：

- 普通 `StdError`
- 结构化 `StructError<R>`

在类型系统里已经彻底分流，不再重叠。

#### D. 再升级 `at(...)` / `doing(...)` 的真实语义

V1 中：

- `at(...)` 只是 `with(...)` 的命名糖衣
- `doing(...)` 只是 `want(...)` 的命名糖衣

V2 才应该正式把它们提升为模型语义，而不是只做命名糖衣。

建议 V2 明确：

- `at(...)`
  - 用于位置、对象、资源引用
  - 对应结构化 target / locator 字段
- `doing(...)`
  - 用于动作现场
  - 对应结构化 action / phase 字段

一旦进入 V2：

- 文档不再使用“只是命名糖衣”的描述
- `want(...)` 不再作为主路径存在
- `with(...)` 不再承担这两类语义的混合入口

也就是说，V2 不只是改名字，而是把上下文协议从旧模型迁到新模型。

#### E. 正式拆出 snapshot / report 层

V2 的另一个关键动作是停止让 `StructError<R>` 同时承担：

- runtime carrier
- serde export object
- stable snapshot
- report rendering payload

建议在 V2 里显式引入：

```rust
pub struct StructErrorSnapshot { ... }
pub struct ErrorReport { ... }
```

并明确职责：

- `StructError<R>`
  - 运行时传播
- `StructErrorSnapshot`
  - 稳定导出 / 测试断言
- `ErrorReport`
  - 人类可读渲染 / redaction / logging export

这样可以避免未来继续把“导出层需求”反向压回 runtime carrier。

#### F. 最后再处理旧 API 清理

只有在上面几步都落稳后，才建议正式处理旧 API：

- 删除或彻底废弃 `with_source(...)`
- 删除或彻底废弃 `want(...)`
- 删除旧式泛化 `owe_*_source()` 主路径语义
- 把 `err_wrap(...)` 明确降到 bridge/compat 层，或彻底移除

顺序必须是：

- 先有新模型
- 再有 bridge
- 再有统一 source 主路径
- 最后再清旧入口

不能反过来。

### 14.2.2 V2 迁移顺序建议

如果把 V2 做成真正可发布的工程计划，建议按下面 5 步推进：

1. 冻结新类型模型
   - `StructError<R>` 退出 `StdError`
   - `OwnedStdStructError<R>` / `StdStructRef<'_, R>` 定型
2. 落 bridge API
   - `into_std()`
   - `as_std()`
   - `into_struct()`
3. 落统一 source 主路径
   - `attach_source(...)`
   - `IntoSourcePayload`
4. 升级上下文协议
   - `at(...)` / `doing(...)` 变成真实模型语义
   - `want(...)` 退出主路径
5. 清理旧 API
   - `with_source(...)`
   - `err_wrap(...)`
   - 旧式 `owe_*_source()` 主路径语义

这 5 步里，最不应该被跳过的是前 2 步。

因为只要：

- `StructError<R>` 仍然兼任 `StdError`

那么：

- 统一 `attach_source(...)`
- source 自动分流
- 更纯净的 trait 边界

都很难真正自然成立。

### 14.3 为什么必须拆成两个版本

如果直接一步走到 V2，会遇到两个现实问题：

1. 迁移成本过高
2. 用户会同时面对：
   - 命名变化
   - API 变化
   - trait 边界变化
   - 第三方兼容方式变化

这会把“收敛错误设计”变成一次高风险升级。

而拆成 V1 / V2 后，节奏会更健康：

- V1 先统一心智和写法
- V2 再重构底层类型边界

也就是：

- 先解决“怎么用”
- 再解决“为什么这个类型模型更优雅”

### 14.4 推荐发布策略

本 RFC 建议：

1. 先发布 V1
2. 在一段兼容周期内收集真实使用反馈
3. 待调用侧基本完成 API 分流后，再启动 V2

具体来说：

- V1 负责把错误写法从“经验规则”变成“清晰规则”
- V2 负责把规则进一步固化为更好的类型边界

### 14.4.1 deprecated 计划

这里需要特别说明 Rust 的 `#[deprecated]` 语义：

- `#[deprecated(since = "0.7.0", note = "...")]` 里的 `since`
- 只是元数据说明
- 不会等到真正发布 `0.7.0` 才开始告警
- 只要属性已经写进代码，当前版本编译就会产生 warning

因此，本 RFC 不建议在 `0.6.x` 就提前写入：

```rust
#[deprecated(since = "0.7.0", note = "...")]
```

否则会导致：

- 还没到 `0.7.0`
- 下游就已经开始收到 deprecated warning

更合理的发布节奏应是：

1. `0.6.x`：软废弃阶段
   - 不加 `#[deprecated]`
   - 只在 RFC、文档、示例、review 规则里明确旧 API 进入 deprecated path
2. `0.7.0`：正式废弃提示阶段
   - 再把选定的旧 API 加上 `#[deprecated(since = "0.7.0", note = "...")]`
3. `0.8.x` 或后续大版本：评估是否移除
   - 视实际迁移情况决定是否删掉旧 API

`0.7.0` 适合正式标记 deprecated 的旧入口，应限制在“已经有稳定替代路径，且替代关系清晰”的范围内：

- `StructError::with_source(...)`
  - 替代：`with_std_source(...)` 或 `with_struct_source(...)`
- `StructErrorBuilder::source(...)`
  - 替代：`source_std(...)` 或 `source_struct(...)`
- `OperationContext::want(...)`
  - 替代：`doing(...)`
- `OperationContext::with_want(...)`
  - 替代：`with_doing(...)`
- `ErrorWith::want(...)`
  - 替代：`doing(...)`

这些 API 适合进入正式 deprecated 的原因是：

- 新旧语义差异可解释
- 调用侧替换路径直接
- 不需要额外推断“到底该换成哪个新入口”

而下面这些旧 API 不建议在 `0.7.0` 就正式标为 deprecated：

- `err_conv()`
  - 原因：当前还没有完全等价的新名字；它也不等于 `wrap_as(...)`
- `err_wrap(...)` / `wrap(...)`
  - 原因：`wrap_as(...)` 多了 `detail` 参数，不是严格一对一替代
- `owe(...)` / `owe_source(...)`
  - 原因：`into_as(...)` 目前是封闭入口，不是它们的完全平替
- `owe_*()` / `owe_*_source()`
  - 原因：旧 API 的快捷语义仍有现实迁移价值，且并未被 V1 完整覆盖
- `with(...)`
  - 原因：`at(...)` 在 V1 只是命名糖衣，`with(...)` 仍承载混合上下文语义，不能机械替换

推荐的 deprecated note 风格应尽量直接给出迁移方向，例如：

- `with_source(...)`
  - `use with_std_source(...) for non-structured errors; use with_struct_source(...) for StructError sources`
- `source(...)`
  - `use source_std(...) for non-structured errors; use source_struct(...) for StructError sources`
- `want(...)`
  - `use doing(...) as the V1 primary naming path; OperationContext storage semantics are unchanged`
- `with_want(...)`
  - `use with_doing(...) as the V1 primary naming path; OperationContext storage semantics are unchanged`

### 14.5 一句话总结

- V1：兼容地收敛语义
- V2：破坏性地收敛模型

这两个版本不是替代关系，而是前后衔接的演进关系。

## 15. 取舍评价

这套新设计的优点：

- API 语义更直观
- 错误生命周期分段更清楚
- 误用更少
- 维护者更容易 code review

代价：

- 方法名变长
- trait 数量增多
- 需要一轮迁移

但在错误系统里，这个取舍是值得的。

更长但更难写错，通常优于更短但需要靠人脑记规则。

## 16. 最终结论

更好的 `orion-error` 不应该只是“功能齐全”，而应该做到：

1. 普通错误和结构化错误天然分流
2. 方法名本身表达意图
3. 上下文 builder 统一
4. `anyhow` 有明确定位
5. 误用尽可能提前暴露

一句话总结：

> 最好的错误 API，不是“防止你用错”，而是“让你几乎不需要思考也不容易用错”。

## 17. V3：稳定架构协议版

前面的 `V1 / V2` 主要解决的是：

- API 如何分流
- source 模型如何收敛
- `StructError` 与 `StdError` 的关系如何调整

如果继续往前走，`V3` 要解决的就不再只是“错误对象怎么设计”，而是：

- 如何让错误体系成为跨 crate、跨边界、跨出口都稳定的工程协议

换句话说：

- `V1` 偏兼容收敛
- `V2` 偏模型重构
- `V3` 偏协议化、治理化、制度化

### 17.1 V3 的目标

`V3` 版本的核心目标有 5 个：

1. 让错误身份稳定，而不是主要依赖 `detail` 文本
2. 让上下文可机器消费，而不只是人类可读
3. 让 runtime / snapshot / renderer 三层真正闭合
4. 让 CLI / HTTP / 日志 / 测试输出共享同一套协议
5. 让错误规范可被 lint / test / migration checker 自动约束

### 17.2 V3 的核心判断

`V2` 已经足够像一个“优秀错误内核”；
`V3` 要把它推进成“稳定架构协议”。

这意味着 `orion-error` 不再只回答：

- 怎么构造错误
- 怎么 attach source

还必须回答：

- 错误的稳定身份是什么
- 哪些上下文是 typed meta，哪些只是展示文本
- 不同出口如何稳定映射
- 如何保证不同项目不会把同一个库用成不同方言

### 17.3 稳定身份：`Reason + Code + Category`

`V3` 保持 `Reason` 是主语义载体，继续坚持：

- `Reason` 决定错误身份
- `ErrorCategory` 只是粗粒度分桶
- `detail` 不参与错误身份判定

但在 `V3` 里，需要把 `code` 的治理规则正式化。

建议：

```rust
pub trait Reason: Debug + Display + Send + Sync + 'static {
    fn code(&self) -> &'static str;
    fn category(&self) -> ErrorCategory;
}
```

并增加协议约束：

- `code` 必须稳定
- `code` 必须可用于测试断言
- `code` 必须可用于 CLI / HTTP / 日志 / telemetry 分桶
- `code` 的变更应视为兼容性事件，而不只是文案调整

建议命名风格：

- `<domain>.<kind>`
- 如：
  - `conf.file_not_found`
  - `conf.invalid_value`
  - `biz.reload_in_progress`
  - `logic.unsupported_file_type`
  - `sys.io_error`

进一步建议：

- `orion-error` core 不强制具体命名
- 但应在文档中明确 code namespace 规则与稳定性承诺

### 17.4 Typed Meta：从“上下文文本”升级为“结构化上下文”

`V2` 的 `at(...) / doing(...) / tag(...)` 已经解决了可读性问题，但还不够支撑稳定协议。

`V3` 需要明确区分两类上下文：

1. typed meta
2. display context

建议核心模型演进为：

```rust
pub struct StructError<R> {
    reason: R,
    detail: Option<String>,
    context: Vec<ContextFrame>,
    meta: ErrorMeta,
    source: Option<SourcePayload>,
}

pub struct ErrorMeta {
    file_path: Option<String>,
    target: Option<String>,
    operation: Option<String>,
    component: Option<String>,
    resource_id: Option<String>,
    hint: Vec<String>,
    extra: BTreeMap<&'static str, String>,
}
```

边界建议：

- `at(...)` 优先落到 `file_path` / `target`
- `doing(...)` 优先落到 `operation`
- `tag(...)` 只作为扩展槽位，不应替代稳定字段
- `with(...)` 在 `V3` 中继续弱化，避免长期承载混合语义

这会直接带来四个收益：

- renderer 可以稳定渲染，而不是猜文本
- 测试可以断言 `operation` / `file_path` / `component`
- HTTP/CLI 可以生成更稳定的 hint
- observability / telemetry 可以直接消费 meta

### 17.5 Runtime / Snapshot / Report / Renderer 四层闭合

`V2` 已经提出了 `runtime / snapshot / report` 分层方向；`V3` 需要把它做成完整闭环。

建议最终拆成四层：

1. `StructError<R>`
   - 运行时传播对象
2. `StructErrorSnapshot`
   - 稳定导出对象
3. `ErrorReport`
   - 面向展示策略的渲染输入
4. `Renderer`
   - 不同出口的格式化实现

草案：

```rust
pub struct StructErrorSnapshot {
    code: String,
    category: ErrorCategory,
    detail: Option<String>,
    meta: ErrorMetaSnapshot,
    source: Option<Box<StructErrorSnapshot>>,
}

pub struct ErrorReport {
    code: String,
    category: ErrorCategory,
    summary: String,
    detail: Option<String>,
    hints: Vec<String>,
    meta: ErrorMetaSnapshot,
    source_chain: Vec<SourceReportFrame>,
}
```

然后提供稳定转换：

```rust
impl<R: Reason> StructError<R> {
    pub fn snapshot(&self) -> StructErrorSnapshot;
    pub fn report(&self) -> ErrorReport;
}
```

以及 renderer 协议：

```rust
pub trait ErrorRenderer {
    type Output;

    fn render(&self, report: &ErrorReport) -> Self::Output;
}
```

这层设计的关键是：

- runtime 不直接背负所有展示责任
- snapshot 不直接等于 CLI 文本
- renderer 不反向污染错误内部模型

### 17.6 出口映射协议

`V3` 必须补齐一层 `report policy`，定义不同出口如何消费 `code / category / meta`。

至少需要覆盖：

- CLI
- HTTP / RPC
- 日志
- 测试断言
- 机器导出

建议约束：

- CLI：
  - 优先显示 `summary + detail + operation + file_path`
  - 默认保留 source chain
- HTTP：
  - 优先基于 `code / category` 映射状态码
  - 默认隐藏底层 raw source 文本
- 日志：
  - 优先输出完整 `code / category / meta / source_chain`
- 测试：
  - 优先断言 `code / category / meta`
  - 避免对 `detail` 全文本做脆弱断言
- 导出：
  - 统一基于 `snapshot` / `report`，而不是复用 CLI 文本

示意：

```rust
pub trait ErrorPolicy {
    fn http_status(&self, code: &str, category: ErrorCategory) -> u16;
    fn user_visibility(&self, code: &str) -> Visibility;
    fn default_hints(&self, code: &str) -> &'static [&'static str];
}
```

这一步非常重要，因为它把“错误对象”升级成了“错误协议”。

### 17.7 Enforcement：让规范可验证

大型工程里，只有设计，没有 enforcement，最终一定会漂。

`V3` 需要把错误规则制度化：

- lint / grep 规则
- compile-time trait boundary
- snapshot test helpers
- migration checker
- deprecated API banlist

建议至少提供：

1. `cargo` 检查建议
   - 禁止 `with_source(...)`
   - 禁止在新代码里继续写 `want(...)`
   - 禁止 `Result<T, String>` 出现在领域边界
2. 测试 helper
   - `assert_err_code(...)`
   - `assert_err_category(...)`
   - `assert_err_operation(...)`
   - `assert_err_path(...)`
3. migration checker
   - 自动扫描旧 API
   - 给出推荐替换路径
4. report snapshot helper
   - 断言稳定 snapshot，而不是断言 CLI 文本

示意：

```rust
assert_err_code(&err, "biz.reload_in_progress");
assert_err_category(&err, ErrorCategory::Biz);
assert_err_operation(&err, "reload engine");
```

### 17.8 V3 对 `StructError: StdError` 的最终立场

`V3` 不改变 `V2` 的推荐方向：

- 长期最优形态仍然是方案 C
  - `StructError<R>` 作为领域真错误
  - `OwnedStdStructError<R>` 作为标准生态桥接层

原因是：

- 稳定架构协议要求内部语义纯净
- 同时大型 Rust 工程仍然需要接入 `anyhow` / 第三方生态

因此，`V3` 的立场不是“彻底抛弃标准错误生态”，而是：

- 把桥接变成显式边界动作
- 不再让领域内错误与 `StdError` 身份长期混在一起

### 17.9 V3 推荐的 API 层次

建议把 API 分成四层：

1. Core model
   - `StructError<R>`
   - `SourcePayload`
   - `ErrorMeta`
   - `StructErrorSnapshot`
   - `ErrorReport`
2. Construction API
   - `into_as(...)`
   - `wrap_as(...)`
   - `with_std_source(...)`
   - `with_struct_source(...)`
   - `at(...)`
   - `doing(...)`
3. Bridge API
   - `into_std()`
   - `as_std()`
   - 官方 `anyhow` bridge
4. Governance API
   - test helper
   - lint/check helper
   - migration helper
   - renderer/policy trait

这能避免把所有能力都挤进一个 `StructError` 类型上。

### 17.10 V3 与 V1 / V2 的关系

`V3` 不是推翻 `V1 / V2`，而是在它们之上补齐协议层。

推荐路线：

1. `V1`
   - 先在兼容前提下完成 API 分流与旧入口收敛
2. `V2`
   - 完成 source 模型与 bridge 模型重构
3. `V3`
   - 增加稳定 code 治理、typed meta、report policy、enforcement

因此更准确地说：

- `V1` 解决“别再继续混”
- `V2` 解决“把模型拆对”
- `V3` 解决“让这套模型成为长期协议”

### 17.11 一句话结论

如果 `V2` 的关键词是：

- `split source`
- `wrap vs into`
- `StructError vs StdError`

那么 `V3` 的关键词应该是：

- `stable code`
- `typed meta`
- `snapshot/report/renderer`
- `policy`
- `enforcement`

一句话总结：

> V2 让 `orion-error` 成为一套更干净的错误模型；V3 才让它成为一套真正稳定的大型工程错误协议。
