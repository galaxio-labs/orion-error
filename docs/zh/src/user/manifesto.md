# 大型工程中的错误治理：通用方法论

这篇文章讨论一个问题：工业级系统如何让错误在治理层面收敛、在诊断层面保真。

如果只想快速把握主线，可以先读这四段：

1. [核心矛盾](#核心矛盾)：为什么错误治理的本质是“收敛 vs. 诊断”。
2. [我们的方案：双通道错误治理模型](#我们的方案双通道错误治理模型)：模型如何拆分治理通道和诊断通道。
3. [基于 Rust 的错误治理方案](#基于-rust-的错误治理方案)：如何用 `orion-error` 落地。
4. [工业级应用验证：WarpParse](#工业级应用验证warpparse)：高吞吐 ETL 场景如何验证这套方法。

本文分成三层：

- **方法论层**：错误处理为什么重要、核心矛盾是什么、五项原则如何定义。
- **工程落地层**：Rust 下如何用 `orion-error` 实现稳定身份、诊断链、边界投影和桥接。
- **工业验证层**：WarpParse 和 AI skills 如何把这套模型用于真实工程。

---

## 错误处理是原型与工业级的分水岭

做好系统的错误处理，是系统从原型走向工程级应用的必由之路。原型只需要证明正确路径可以跑通；工程级应用还必须在非理想条件下可运行、可诊断、可恢复、可演进。

系统并不会长期运行在理想条件下。输入会变化，依赖会退化，网络会抖动，配置会漂移，数据会积累脏状态，业务规则会迭代。处理路径也会随着用户、环境、状态和策略动态分叉。正确路径不是系统运行的全部；失败、降级、重试、回滚、补偿和人工介入同样是系统生命周期的一部分。

因此，错误不是"正常逻辑之外的意外文本"，而是系统在非理想条件下继续运行、恢复状态、决定对外响应和支持诊断时必须传递的信息。

很多项目在早期都会把错误处理当作"每个函数自己的事"。每个函数决定怎么表达自己的失败，然后这个决策在下一个函数、下一个模块、下一个边界里被重新做一次。

这种模式在小型项目中可以工作——调用链短、边界少、参与者对上下文有共同记忆。但当错误信息没有统一形态，且开始跨越团队、子系统、服务边界、协议边界或长期兼容边界时，失败路径上的行为就会变得不可治理：

- 同一种失败，A 模块返回字符串，B 模块返回 enum，C 模块直接 panic
- 边界输出时，每一层都在重新拼 JSON，拼出来的结构却不一致
- 排障时，日志里有散落的消息，但没有一条完整的错误路径
- 重构时，不敢动错误类型——因为不知道哪些上层依赖于这个错误的字符串内容

这些现象不一定来自某个函数"写得不好"。它们也可能来自缺乏统一工具、缺乏团队规范、历史演进、人员流动或边界职责不清。关键在于：一旦错误需要在多个边界之间传播、被多个角色消费、并长期保持兼容，错误就不再只是局部控制流。

错误治理的价值在于，它定义了失败发生后系统如何保留信息、如何跨层传递、如何对外暴露、如何支撑诊断和演进。它不是业务逻辑之外的装饰，而是业务逻辑失败时的信息架构。

---

## 行业一直在探索错误处理

错误处理的重要性早已得到专业工程师和工业界的共同认可。真正困难的地方不在于是否要处理错误，而在于如何有效处理错误：既不能让错误吞没业务代码，也不能让失败路径退化成不可治理的字符串；既要让调用方能够做稳定决策，又要让排障方保留足够细节。

不同语言对错误处理的设计，正说明这个问题长期没有唯一答案。

- C 主要依赖返回码、`errno` 和约定。它直接、低成本，但错误信息容易分散，调用方也容易漏检。
- Java 把异常机制作为主路径，并区分 checked exception 与 unchecked exception。它强化了错误传播能力，但也带来了异常层次膨胀、边界语义不清和过度捕获的问题。
- Go 强调显式返回 `error`。这种方式让失败路径可见，但如果缺少团队约束，错误很容易变成层层包装的字符串。
- Rust 通过 `Result<T, E>`、`?`、枚举和类型系统把错误纳入普通控制流。它让错误路径更显式、更可组合，但仍然需要工程层面的分类、上下文、边界暴露和诊断策略。

这些设计各有取舍。语言机制能降低错误处理的成本，却不能替代错误治理本身。大型工程真正需要解决的不是选择异常、返回码还是 `Result`，而是失败信息在系统中如何被分类、保留、转换、暴露和观测。

对研发团队来说，有效完成错误处理确实是一个难题。它横跨类型设计、调用链传播、日志与观测、协议输出、用户体验、运维策略和长期兼容；任何一个环节各自为政，最终都会在排障、重构和边界协作时暴露成本。因此，错误处理不能只依赖个人经验和局部习惯，而需要一套可讨论、可执行、可演进的方法论。

到今天为止，业界并没有形成一种跨语言、跨框架、跨业务形态的统一错误治理方案。但一些优秀项目已经在不同方向上给出了可参考的实践：稳定错误码、结构化诊断、集中边界策略、状态化错误呈现、可观测错误信号、面向用户的修复提示。这些实践说明，错误治理不是单一 API 的问题，而是一组围绕失败信息展开的工程约束。

## 来自优秀项目的证据

| 项目 | 值得借鉴的做法 | 对本文模型的启发 |
|------|----------------|------------------|
| gRPC | 将跨语言 RPC 失败收敛为标准状态码 | 稳定分类让调用方可以重试、降级、告警和映射用户响应 |
| PostgreSQL | 使用稳定 SQLSTATE 错误码，不依赖错误文本 | 机器契约和人类文案应该分离 |
| Kubernetes | 把就绪状态、失败原因和 condition 写入 `status` | 错误可以是可查询、可观察、可自动化处理的系统状态 |
| Terraform | diagnostics 包含 severity、summary、detail、attribute path | 错误应该指出位置、原因和修复方向 |
| rustc | 错误码、源码位置、label、note、help 共同构成诊断体验 | 诊断信息本身是产品体验 |
| Envoy | access log response flags 表达稳定失败原因 | 边界层错误应能被聚合、搜索、告警和自动分析 |

这些项目的形态不同，但方向一致：优秀的错误处理都会把失败路径设计成稳定的信息系统。它既有机器可判断的分类，也有人能理解的诊断；既能在内部保留细节，也能在边界按策略暴露；既服务当前请求，也服务后续排障、监控和演进。

---

## 核心矛盾

任何错误治理方案都要处理一对根本矛盾：

**收敛 vs. 诊断**

- 调用方需要**稳定的、有限的分类**，否则无法做出治理决策（重试、降级、告警、返回给用户）
- 排障方需要**完整的、保留细节的信息**，否则无法定位根因

这对矛盾不只是技术问题，也是一种需求张力。调用方需要稳定、受控、可治理的信息；排障方需要更完整的上下文和根因。两类需求都合理，但它们天然拉向不同方向。

如果错误向调用方暴露过多技术细节，上层就会依赖数据库、网络库、文件系统、第三方 SDK 的具体失败形态，系统边界会被技术实现穿透。重构底层实现时，错误契约也会被迫变化。

如果错误只保留上层业务分类，排障时又会失去关键路径：原始失败是什么、发生在哪个组件、经过了哪些层、每一层追加了什么上下文、最终为什么被映射成这个对外响应。

因此，错误治理的主要矛盾不是"要不要包装错误"，而是：**如何同时让错误在治理层面收敛，在诊断层面保真。**

### 不充分的解决方案

| 策略 | 对调用方 | 对排障方 |
|------|---------|---------|
| 只抛技术异常 | 无法治理 | 信息完整 |
| 只抛业务错误 | 可以治理 | 丢失根因 |
| 纯字符串链式包装 | 无法治理 | 可读但不可结构化查询 |
| 保留类型信息的链式包装 | 可做局部治理 | 保留原因链，但分类与边界策略仍需额外约束 |
| 吞掉错误 | 干净 | 丢失所有信息 |

这里需要区分两种"包装"：纯字符串链只是把错误文案一层层拼起来；保留类型信息的链式包装，例如 cause chain、typed wrapping、`errors.Is` / `errors.As` 一类机制，可以支持一定程度的结构化查询。后者比纯字符串链更成熟，但它通常只解决"原因如何保留和查询"，并不自动解决稳定错误身份、分类边界、暴露策略和治理动作映射。

这些方案如果只依赖单一形态同时满足两类需求，结果往往是牺牲其中一边：要么调用方拿到的信息太散，无法自动化治理；要么排障方拿到的信息太少，只能重新翻日志和复现现场。

### 砖块不等于建筑

一个常见的质疑是：Java 的异常类型 + 错误码、Go 的 sentinel error + wrapping、Rust 的 enum + cause chain，这些机制不是已经覆盖了双通道模型要解决的大部分问题吗？

这些机制确实都是砖块，但砖块不等于建筑。问题出在三个方向。

**异常类型或错误码作为分发主键，身份不稳定。** Java 生态中，异常类型通常承担分发角色：`catch (OrderNotFoundException e)`。但异常类型受继承层次控制，重构时会变。错误码通常是异常的附赠字段，调用方先做 `instanceof`，再取 `getErrorCode()`——错误码不是路由主键。双通道模型的主张是：把错误身份提升为治理通道的第一公民，边界策略基于身份路由，与类型的继承层次解耦。不管你用 `enum`、错误码字符串还是 tagged union 来表达它，身份本身必须是稳定的、可文档化的、被测试约束的契约。

**分类空间天然膨胀。** 异常机制鼓励"一种失败一个类"：`SubmitDependencyUnavailableException`、`InvalidStateException`……类的数量跟随业务失败模式无限制增长。没有机制强制"收敛到有限分类"。如果异常类型同时承担分类职责，分类就不可能稳定——每新增一个 exception class，所有依赖异常层次做路由的边界都可能受影响。双通道模型把分类空间（`R`）限制为有限枚举，新增变体是有意为之的兼容演进，不是随意加类。

**治理和诊断共用同一通道，两类需求互相牵制。** cause chain、structured wrapping、`errors.Is`/`errors.As` 解决的是诊断保留问题——根因如何保留和查询。它们不解决：这个错误应该触发重试还是降级？应该映射到哪个 HTTP 状态码？应该暴露给用户还是只记日志？这些治理动作如果由异常类型、字段或局部判断来做，就会散落在每个 handler、每个 catch 块、每个 `errors.Is` 调用中。双通道模型做的是把治理信息和诊断信息分离成不同通道，治理动作由集中策略而非局部代码决定。

综上，关键区别不在于你用的是什么语言机制，而在于你的错误架构有没有以下几根承重墙：稳定身份（不受类型重构影响）、有限分类空间（受兼容演进规则约束）、诊断保留（跨层不丢失）、集中边界策略（不在 handler 中重复决策）。失去了这些结构，任何语言的错误处理都会长成不可治理的灌木丛——哪怕你用的砖块是顶好的。

### 我们的方案：双通道错误治理模型

核心方法论：**把"分类信息"和"诊断信息"分离到两个不同的维度，通过两个不同的通道传递。**

```text
错误 = 稳定身份 + 稳定分类 + 诊断链 + 上下文 + 细节
```

稳定身份和稳定分类服务治理决策：重试、降级、告警、HTTP/RPC/CLI 映射、用户提示、SLA 统计。它们应该有限、稳定、可文档化、可测试。

诊断链、上下文和细节服务问题定位：底层原因、经过的层、当前操作、关键字段、组件信息、环境信息。它们可以更丰富、更动态，但不应该成为外部调用方的稳定契约。

这两个通道分别是：

| 通道 | 包含什么 | 服务谁 | 稳定性要求 |
|------|---------|--------|------------|
| 治理通道 | 稳定身份、稳定分类、category、retryable、severity、暴露等级 | 调用方、网关、监控、运维策略、协议客户端 | 高，应该被文档化和测试约束 |
| 诊断通道 | 原因链、操作上下文、关键字段、动态细节、底层错误 | 开发者、SRE、排障工具、日志系统 | 可以动态变化，但必须保真且可追溯 |

其中，`category` 是稳定分类的固定治理维度，常见取值包括 business / system / config，用于快速区分错误归属域，辅助告警路由、日志聚合和边界策略。`retryable`、`severity`、暴露等级等属于治理属性，通常由稳定身份和分类策略派生。

几个组成部分的边界如下：

| 组成部分 | 含义 | 例子 |
|----------|------|------|
| 稳定身份 | 机器可判断的错误主键，面向长期兼容 | `order.not_found`、`system.timeout` |
| 稳定分类 | 面向治理决策的有限类别 | 业务错误、配置错误、系统错误、超时、限流 |
| 治理属性 | 从稳定身份和分类策略派生的辅助决策字段 | category、retryable、severity、暴露等级 |
| 诊断链 | 错误跨层传播时保留下来的 cause/source 路径 | service failure -> repository failure -> database timeout |
| 上下文 | 当前操作的结构化环境信息，回答"在哪里、对谁、执行什么" | operation、tenant、path、order_id、component |
| 细节 | 当前层对这次失败的具体解释，通常更接近人类诊断文本 | `read config failed`、`upstream returned 503` |

同一个错误可以通过不同视图同时服务两类需求，减少调用方和排障方互相牺牲。

---

## 方案原则

双通道模型要落地，至少需要五项原则配合：统一载体负责承载结构，治理通道保持稳定，诊断通道跨层保真，边界集中投影，外部生态显式桥接。

本节中的代码只表达方法论形状，是语言无关的伪代码，不对应任何具体库的 API。

### 原则一：用统一载体承载双通道信息

**应用自有的跨层错误传播路径应该使用统一的结构模型。**

反例：

```rust
// A 模块返回 io::Error
fn read_file() -> io::Result<Data>

// B 模块返回自定义 enum
fn validate() -> Result<Data, ValidationError>

// C 模块返回字符串
fn process() -> Result<Data, String>
```

每条错误路径的调用者都需要学习一套新的错误形状。组合两个不同函数的错误路径时，调用方既要判断分类，又要重新拼接诊断信息，还要决定边界输出格式。

正例：

```text
read_file() -> Result<Data, StructuredError<ErrorClass>>
validate() -> Result<Data, StructuredError<ErrorClass>>
process() -> Result<Data, StructuredError<ErrorClass>>
```

载体模型是统一的，才能同时承载稳定分类和诊断信息。变化的是分类空间和上下文内容。不同层可以拥有自己的分类空间，但跨层传播时应该有清晰的收敛或边界转换规则。

统一载体不是要求第三方库、标准库、框架异常或协议错误全部改成同一种类型；它要求团队控制的内部传播路径使用同一种结构模型，并在进入或离开外部生态时显式桥接。

### 原则二：让治理通道保持稳定

**错误分类契约应该按向后兼容规则演进。**

错误的分类体系是契约——调用方依赖它做治理决策。这里的"稳定"不是指永远不能增加新分类，而是指已有分类的机器身份和语义不能随意变化。

错误身份是治理通道里的机器主键，通常表现为一个稳定字符串、错误码或协议字段，例如 `business.not_found`、`system.timeout`、`config.invalid`。调用方、网关、监控、告警和文档都应该依赖这个身份，而不是依赖错误消息文本。

分类契约的演进应该遵循兼容规则：

- 可以新增错误身份或分类，用来表达新的业务失败或系统失败。
- 不应该删除已经对外承诺的错误身份；如果必须废弃，应保留兼容映射或经过明确的版本迁移。
- 不应该改变已有错误身份的语义，例如把 `business.not_found` 从"资源不存在"改成"无权限访问"。
- 不应该让同一个错误身份在不同边界产生互相矛盾的治理动作，例如有的地方映射为可重试，有的地方映射为不可重试。
- 可以调整错误文案、诊断细节、上下文字段和底层原因链，只要不破坏稳定身份和分类语义。

| 应该稳定的 | 可以变化的 |
|-----------|-----------|
| 稳定错误身份 | 诊断细节 |
| 分类语义 | 错误信息文案 |
| category（业务/系统/配置） | 具体的技术细节 |

稳定分类的另一个好处：它是人和系统之间的共享接口。运维配置告警规则、网关配置状态码映射、API 文档描述错误响应——这些全部依赖于稳定身份和分类语义，而不是依赖于错误文本。枚举、异常类型、错误码或 tagged union 都只是表达这个契约的具体方式。

### 原则三：让诊断通道跨层保真

**错误在内部传播时应该追加信息，不应破坏已有诊断链。**

反例：

```text
repository() -> Result<Data, RepoError> {
    // 数据库连接失败，返回 RepositoryConnectionFailed
}

service() -> Result<Data, ServiceError> {
    data = repository()?  // 丢弃了下层错误的具体信息
    return data
}
```

正例：

```text
repository() -> Result<Data, StructuredError<RepositoryClass>> {
    // 数据库连接失败，并保留原始数据库错误
}

service() -> Result<Data, StructuredError<ServiceClass>> {
    data = repository()
        .wrap_as_cause(ServiceDependencyFailed, "load repository data failed")
    return data
}
```

每层保留的信息形成一条完整的错误链。排障时可以从最终错误追溯到原始根因。

这里有两种不同操作：

- 如果当前层只是把下层分类收敛到上层分类空间，不建立新的语义边界，应保留原有诊断链，不制造新的错误叙事。
- 如果当前层要表达新的失败语义，应把下层错误作为原因保留下来，并追加当前层的解释。

判断标准是语义域，而不是函数层数。

如果上下层仍在同一个语义域内，错误转换通常只是分类收敛。例如 database driver、query executor、repository helper 都属于 data access 语义域；它们之间可以把底层连接失败收敛为 `RepositoryConnectionFailed`，同时保留原始数据库错误和上下文，不必每一层都新增一段业务叙事。

如果错误跨越了语义域或架构责任边界，就应该建立新的语义边界。例如 data access 失败进入 order service 时，上层关心的可能不是"数据库连接失败"，而是"订单草稿加载失败"或"提交订单依赖不可用"。这时应追加 service 层语义，并把下层 data access 错误作为原因保留下来。

可以用几个问题辅助判断：

- 当前层是否在向上层隐藏一个实现细节？
- 当前层是否拥有新的业务含义、用户意图或操作目标？
- 当前层是否会改变治理动作，例如从底层 timeout 映射为业务依赖不可用？
- 如果未来替换下层实现，上层错误契约是否应该保持不变？

如果答案是"是"，通常说明这里是语义边界；如果只是模块拆分、工具函数或同一领域内的技术分层，通常只需要分类收敛和诊断保留。

边界输出时再按策略做脱敏和投影。

### 原则四：在边界集中投影

**边界暴露策略应该集中定义，而不是在每个边界点重新决定。**

结构化错误在内部传播时携带治理通道和诊断通道；到达系统边界时，边界层应先从治理通道取得稳定身份，也就是 `error_identity`，再交给统一策略决定如何投影。

```text
StructuredError<ErrorClass>
    -> error_identity
    -> exposure_policy
    -> HTTP / RPC / CLI / log / metric
```

反例：

```text
// handler A
match err {
    NotFound => HttpResponse(404, "not found"),
    Timeout => HttpResponse(503, "try again"),
}

// handler B
match err {
    NotFound => HttpResponse(404, "resource missing"),
    Timeout => HttpResponse(504, "gateway timeout"),
}
```

两个 handler 对同一个错误的输出不一致。

正例：

```text
// 策略集中定义
policy.status(error_identity) {
    match error_identity {
        "business.not_found" => 404
        "system.timeout" => 503
        _ => 500
    }
}

// 所有边界点使用同一策略
render_error_response(err, policy)
```

集中策略不只负责 HTTP 状态码。它通常还应该覆盖：

- 对外错误码和用户可见消息
- HTTP/RPC/CLI 的格式映射
- 日志级别和结构化日志字段
- 是否触发告警或计入 SLA
- 是否建议调用方重试、降级或停止重试
- 诊断信息的脱敏和暴露等级
- 指标标签和错误聚合维度

这些决策如果散落在每个 handler、worker、controller 或 adapter 中，同一个错误身份就可能在不同边界产生不同表现，最终破坏治理通道的稳定性。

### 原则五：显式桥接外部生态

**进入外部生态（日志系统、标准错误接口、第三方库）应该是显式的。**

反例：

```text
// 调用者可以在不知情的情况下把错误降级为普通字符串或通用异常
handle(error_as_text)  // 擦除了错误的结构化信息
```

正例：

```text
// 需要显式选择进入外部生态
plain_error = err.to_plain_error()
log_record = err.to_log_record(redaction_policy)
```

显式桥接确保结构化信息的丢失、脱敏或降级是有意为之，不是无意遗漏。但仅仅"显式调用一个转换函数"还不够；每个桥接函数都应该有清楚的桥接契约。

桥接契约需要说明：

- 目标消费者是谁：用户、协议客户端、日志系统、监控系统、第三方库，还是标准错误接口。
- 保留什么：稳定身份、分类、原因链摘要、操作上下文、关键字段、retryable、severity。
- 丢弃什么：内部实现类型、敏感字段、过长的底层错误、无法稳定解析的动态文本。
- 脱敏什么：token、密钥、用户隐私、租户隔离信息、内部拓扑、SQL 片段或请求载荷。
- 如何降级：当目标生态只能接收字符串或普通异常时，哪些结构化字段会被压缩进文本，哪些字段会彻底丢失。

不同桥接目标应该有不同契约。写日志时通常应保留稳定身份、分类、操作上下文、关键字段和原因链摘要；对外响应时通常只暴露稳定错误码、可公开消息和必要的修复提示；进入标准错误接口时可能只能保留文本、source 链和有限类型信息。桥接的重点不是"所有信息都带出去"，而是让每一次信息投影都可审计、可测试、可预期。

---

## 错误传播的三种模式

错误传播不是把同一个错误对象机械向上抛。一个工业级错误通常会经历三种动作：首次进入结构化系统、跨层转换、边界输出。

### 首次进入

原始错误（IO 错误、解析错误、网络错误）第一次进入结构化系统。此时需要：

1. 选择合适的分类（业务 vs 系统 vs 配置）
2. 提供当前层能给出的解释（detail）
3. 保留原始错误作为底层原因

这里的三个诊断概念分工不同：

- `source/cause` 是底层真实失败，回答"根因是什么"。
- `context` 是结构化环境，回答"在哪里、对谁、执行什么"。
- `detail` 是当前层解释，回答"当前层如何理解这次失败"。

### 跨层转换

上层需要将下层的错误分类收敛到自己的分类空间。此时：

- 如果只是分类重新映射，保留所有诊断信息
- 如果要建立新的语义边界，将下层错误作为底层原因包裹

这取决于当前层是否是一个新的语义边界。

### 边界输出

在系统边界（HTTP handler、RPC 端点、CLI 入口、日志写入点）输出错误。此时：

1. 选择输出格式（JSON、文本、结构化日志）
2. 应用暴露策略（哪些信息可以对外暴露）
3. 输出

### 一个完整传播示例

下面的伪代码展示一次"提交订单"失败如何经过三种模式。

第一步，数据库错误首次进入结构化系统。repository 层选择 data access 语义下的稳定分类，保留数据库错误作为 source，并添加当前操作上下文。

```text
repository.insert_order(order) -> Result<(), StructuredError<RepositoryClass>> {
    db.insert(order)
        .on_error(source_error) {
            return StructuredError {
                identity: "repository.connection_failed",
                class: RepositoryConnectionFailed,
                detail: "insert order failed",
                context: {
                    operation: "insert_order",
                    order_id: order.id,
                    component: "order_repository"
                },
                source: source_error
            }
        }
}
```

第二步，service 层跨越到业务语义域。它不把数据库连接失败直接暴露给上层，而是表达当前业务操作失败：提交订单依赖不可用。同时保留下层 repository 错误作为 cause。

```text
service.submit_order(order) -> Result<(), StructuredError<ServiceClass>> {
    repository.insert_order(order)
        .on_error(repo_error) {
            return StructuredError {
                identity: "order.submit_dependency_unavailable",
                class: SubmitDependencyUnavailable,
                detail: "submit order failed",
                context: {
                    operation: "submit_order",
                    order_id: order.id,
                    tenant: order.tenant
                },
                source: repo_error
            }
        }
}
```

第三步，HTTP handler 到达系统边界。它不重新解释错误，也不拼接底层数据库消息，而是把错误交给集中策略投影成响应、日志和指标。

```text
handler.post_orders(req) -> HttpResponse {
    result = service.submit_order(req.order)

    if result is error {
        err = result.error
        identity = err.identity

        log_record = policy.to_log_record(err)
        metrics.record(policy.metric_labels(identity))

        return HttpResponse {
            status: policy.http_status(identity),
            body: policy.public_body(identity),
            retry_after: policy.retry_after(identity)
        }
    }
}
```

这条链里，治理通道最终给边界层的是 `order.submit_dependency_unavailable`，用于决定状态码、用户消息、重试建议和指标标签；诊断通道内部仍然保留了 service detail、repository detail、上下文和原始数据库错误。调用方不需要知道数据库细节，排障方仍然可以追溯根因。

---

## 治理等级

一个团队的错误治理成熟度可以分为四个等级：

**L0：无治理**

- 错误类型散乱：`std::io::Error` / `String` / `Box<dyn Error>` / 自定义 enum 混用
- 边界输出拼接字符串
- 排障依赖 grep 日志

**L1：统一载体**

- 自有跨层错误传播路径返回同一结构模型
- 但分类随意，相同的失败在不同模块归类不一致
- 有原因链，但原因链经常在跨层时被丢弃

**L2：稳定分类**

- 分类契约稳定，有文档定义
- 边界输出使用统一策略
- 原因链在跨层传播中完整保留
- 测试中断言错误身份，而不是断言错误消息

**L3：治理驱动**

- 错误分类直接映射到治理动作（重试、降级、告警、SLA 计算）
- 边界策略可配置，不同环境可不同
- 错误指标进入监控系统
- 新错误类型需要 review 才能加入

大多数团队在 L0 和 L1 之间。L1 到 L2 是最容易被低估的一步：它不是把返回类型换成统一载体就结束，而是要让团队对"哪些失败应该共享同一个身份"、"哪些分类代表可重试"、"哪些错误可以对外暴露"形成共同语义。

从 L1 走到 L2，通常需要完成几类工作：

- 标准化分类契约，明确稳定身份、分类语义、category 和治理含义。
- 梳理存量错误，把散落的字符串、技术异常和临时 enum 迁移到稳定分类。
- 建立边界策略，统一 HTTP/RPC/CLI/log/metric 的投影规则。
- 建立测试规范，断言错误身份和治理决策，而不是断言错误消息。
- 建立评审习惯，让团队在新增错误时讨论语义归属，而不是只讨论代码能否编译。

因此，L1 到 L2 不是一次局部重构，而是团队协作模式的变化：错误分类开始从个人实现细节变成共享工程语言。

L3 则意味着错误治理进入组织流程。新错误类型需要 review，不是为了制造流程负担，而是因为每个新的稳定身份都可能影响告警、重试、SLA、用户文案、协议兼容和运维看板。到了这个阶段，错误分类的变更应像 API 变更一样被管理：有命名规范、有兼容性规则、有策略映射、有测试覆盖，也有废弃和迁移路径。

---

## 不适用场景

这套方法论不是万能的。以下场景不适合：

1. **小型项目、原型、脚本。** 边界少、生命周期短、错误主要在局部被处理时，没有必要引入分层治理。
2. **性能极端敏感的场景。** 结构化错误路径会有分配、原因链和上下文采集、序列化等成本；在静态类型语言中，泛型或模板还可能增加编译时间和代码体积。
3. **错误不需要跨层传播。** 如果所有错误都在一层内处理完毕，这套方法论的收益接近于零。

---

## 语言机制与生态采纳

这套方法论与具体语言无关，但不同语言落地它时会遇到不同成本。这里需要区分两个维度：

- **语言表达能力**：语言本身是否方便表达稳定分类、结构化载体、原因链和边界投影。
- **生态采纳成本**：团队在既有生态中采用这套治理方式需要付出多少组织和迁移成本。

亲和度高不等于采纳容易。Rust 的类型系统非常适合表达这套模型，但错误处理生态长期存在多种路径；Go 的类型表达能力较弱，但错误返回模式高度统一，引入轻量分类规范的组织成本可能反而较低。

### Rust — 原生匹配

Rust 同时满足三个条件：
- 代数类型（`enum`）表达错误分类，`match` 提供穷尽性检查
- 泛型提供类型安全的载体参数化
- 无异常机制，错误通过返回值传递，自然与载体配合

但 Rust 的实际采纳并不自动简单。生态中长期存在 `failure`、`error-chain`、`anyhow`、`thiserror`、`eyre` 等不同取向：有的偏快速传播，有的偏诊断报告，有的偏领域错误定义。团队仍然需要明确边界：哪些层使用结构化治理错误，哪些边界允许快速错误聚合，哪些错误身份进入长期契约。

### TypeScript — 亲和度高

```typescript
type AppErrorClass =
  | { kind: "not_found"; id: string }
  | { kind: "system_error" };
```

Union type + discriminated union 天然适合错误分类。`neverthrow`、`fp-ts` 的 `Either` 等库提供了返回值式错误处理。Zod 等 schema 库也能帮助把输入校验错误结构化，并把字段路径、错误码和用户提示分离出来。

弱点是运行时类型信息有限。TypeScript 的泛型主要存在于编译期，跨进程、跨包、跨 JSON 边界时，仍然需要显式的 runtime tag、schema 或协议字段来保存错误身份和分类。

### Swift — 亲和度高

代数类型（enum with associated values）表达错误分类。`Result<T, E>` 在 Swift 5.0+ 中原生支持。社区中有使用 `Result` 替代 `throws` 的实践。

### C# — 需要映射到异常生态

泛型支持良好（运行时保留类型信息），但异常机制主导生态。缺少原生 discriminated union（可用 `OneOf` 库模拟）。更自然的映射方式通常不是强行改成 Result，而是用异常类型层次表达分类、用 inner exception 保留原因链、用 ASP.NET Core 中间件做集中策略。

### Java — 需要映射到框架约定

泛型擦除，异常机制主导。但 Java 的 cause chain 机制成熟，Spring 的 `@ControllerAdvice`、filter、interceptor 已经是集中策略的常见模式。Java 17+ 的 sealed class、record 和 pattern matching 也让有限分类表达比过去更自然。

更合适的做法通常是借鉴分类稳定、诊断链、边界集中投影这些思想，而不是照搬返回值式载体。下面是一个可行的映射方案。

**核心映射：每个语义域定义一个 sealed class，域之间无继承关系。**

和 Rust 方案对应——Rust 中每个语义域有自己的 `Reason` 枚举，`RepositoryReason` 和 `OrderReason` 是两个独立的类型，不共享 trait 之外的继承。Java 同理：`RepositoryError` 和 `OrderError` 是两个独立的 sealed class，各自在自己的语义域内约束分类空间。跨域时不是向上转型到共同的父类，而是**构造新域的异常，把旧域异常作为 cause 保留**。

```java
// ===== data access 语义域 =====
public sealed abstract class RepositoryError extends RuntimeException
    permits RepositoryError.ConnectionFailed,
           RepositoryError.WriteFailed,
           RepositoryError.General {

    public abstract String identity();
    public abstract String category();
    public abstract boolean retryable();

    private DiagnoseContext ctx;
    public DiagnoseContext context() { return ctx; }

    protected RepositoryError(String detail, Throwable cause, DiagnoseContext ctx) {
        super(detail, cause);
        this.ctx = ctx;
    }

    public static final class ConnectionFailed extends RepositoryError {
        public ConnectionFailed(String detail, Throwable cause, DiagnoseContext ctx) {
            super(detail, cause, ctx);
        }
        public String identity() { return "repository.connection_failed"; }
        public String category() { return "system"; }
        public boolean retryable() { return true; }
    }
    // WriteFailed, General ...
}

// ===== order 业务语义域 =====
public sealed abstract class OrderError extends RuntimeException
    permits OrderError.DependencyUnavailable,
           OrderError.InvalidState,
           OrderError.General {

    public abstract String identity();
    public abstract String category();
    public abstract boolean retryable();

    private DiagnoseContext ctx;
    public DiagnoseContext context() { return ctx; }

    protected OrderError(String detail, Throwable cause, DiagnoseContext ctx) {
        super(detail, cause);
        this.ctx = ctx;
    }

    public static final class DependencyUnavailable extends OrderError {
        public DependencyUnavailable(String detail, Throwable cause, DiagnoseContext ctx) {
            super(detail, cause, ctx);
        }
        public String identity() { return "order.submit_dependency_unavailable"; }
        public String category() { return "system"; }
        public boolean retryable() { return true; }
    }
    // InvalidState, General ...
}
```

`DiagnoseContext` 是跨域通用的 record，不绑定特定语义域：

```java
public record DiagnoseContext(
    String operation,
    String entityId,
    String tenant,
    String component
) {}
```

**完整的传播路径：三层，两个语义域。**

第一步，数据库错误在 Repository 层首次进入结构化体系：

```java
// Repository 层：首次进入
var ctx = new DiagnoseContext("insert_order", order.id, null, "order_repository");
try {
    db.insert(order);
} catch (SQLException e) {
    throw new RepositoryError.ConnectionFailed("insert order failed", e, ctx);
}
```

第二步，Service 层跨越语义域。这里的关键动作不是向上转型，而是**构造新域的异常**：`RepositoryError` 不继承 `OrderError`，两者是平级的独立类型，通过 cause chain 连接。

```java
// Service 层：跨语义域——构造 OrderError，把 RepositoryError 作为 cause
var ctx = new DiagnoseContext("submit_order", order.id, order.tenant, "order_service");
try {
    repository.insert(order);
} catch (RepositoryError e) {
    throw new OrderError.DependencyUnavailable("submit order failed", e, ctx);
    //                                              detail ─────────┘  ↑   ↑
    //                                              cause ────────────┘   │
    //                                              context ──────────────┘
}
```

此时 cause chain 为：
```text
OrderError.DependencyUnavailable
  └─ cause: RepositoryError.ConnectionFailed
       └─ cause: SQLException ("Connection reset")
```

第三步，边界层交给 `@ControllerAdvice` 统一投影。`@ExceptionHandler` 按域注册：`OrderError` 的 handler 处理所有业务语义域的错误，`RepositoryError` 如果没有被上层转换则在边界作为 500 兜底。

```java
@ControllerAdvice
public class ErrorPolicy {
    @ExceptionHandler(OrderError.class)
    public ResponseEntity<ErrorBody> handleOrder(OrderError e) {
        logger.error(e.toLogRecord());              // 诊断通道：完整 cause chain + context
        return ResponseEntity
            .status(statusOf(e.identity()))          // 治理通道：基于身份路由
            .body(new ErrorBody(e.identity(), publicMessageOf(e.identity())));
    }

    @ExceptionHandler(RepositoryError.class)
    public ResponseEntity<ErrorBody> handleRepo(RepositoryError e) {
        // 未被上层转换的 repository 错误 = 内部错误兜底
        logger.error(e.toLogRecord());
        return ResponseEntity.status(500)
            .body(new ErrorBody("internal_error", "internal error"));
    }
}
```

**和 Rust 方案的对比。** 两者的结构一一对应：

| 概念 | Rust | Java |
|------|------|------|
| 语义域分类 | `enum RepositoryReason` | `sealed class RepositoryError` |
| 治理通道 | reason 枚举变体 + identity 字符串 | 子类覆写的 `identity()` / `category()` / `retryable()` |
| 诊断通道 | `StructError` 的 detail / context / source 字段 | `getMessage()` / `context()` record / `getCause()` |
| 统一载体 | `StructError<R>` 泛型参数化 | **不可行** —— JLS 禁止泛型类继承 Throwable |
| 跨域转换 | `source_err(OrderReason::..., detail)` —— 一行 | `catch (RepositoryError e) { throw new OrderError(..., e, ctx) }` —— 四行 |

Java 无法像 Rust 那样用一个 `StructuredError<R>` 泛型类统一所有域的载体——Java 语言规范明确禁止泛型类继承 `Throwable`（`class StructError<R> extends RuntimeException` 是编译错误）。即使绕过 Throwable 改用返回值式载体，又会丢失 `@ControllerAdvice`、cause chain、堆栈追踪等异常生态基础设施。因此 Java 方案只能用独立 sealed class 作为每个域的载体——这是类型系统的硬约束，不是设计偏好。

Java 方案的额外代价：跨域转换必须显式 try-catch，无法像 Rust 的 `?` + `source_err` 那样一行完成。这不是设计问题，是异常机制的固有代价——异常通过抛出/捕获传递，不存在 `map_err` 式的链式转换。

**测试。** 和 Rust 版本同理，断言身份而非消息：

```java
@Test
void shouldFailWithDependencyUnavailableWhenRepoFails() {
    OrderError err = assertThrows(OrderError.class,
        () -> service.submit(order));
    assertEquals("order.submit_dependency_unavailable", err.identity());
    assertTrue(err.retryable());
    // 诊断链完整
    assertNotNull(err.getCause());                   // RepositoryError
    assertNotNull(err.getCause().getCause());         // SQLException
}
```

这个方案的关键不是异常 vs. Result 的选择。关键是：每个语义域的 sealed class 是独立类型（不共享业务继承层次），跨域通过构造新异常 + cause 保留完成，稳定身份是字段主键而非类的副产品，边界策略集中路由，测试断言身份和诊断存在性。这几根承重墙和 Rust 方案完全一致。Java 的附加代价是跨域 try-catch 的代码量和编译器不强制 context 存在——这两项都需要团队纪律补足。

### C++ — 技术可行，生态无约定

模板保留类型信息，`std::expected`（C++23）提供类似 `Result` 的机制，Boost.Outcome 等库也提供了更完整的结果/错误建模能力。但 C++ 错误处理长期并存异常、错误码、expected、Outcome、自定义 status 类型等多条路径，生态无主导载体。技术上可行，组织统一成本较高。

### Go — 需要更多团队约束

`error` 接口默认只要求 `Error() string`，结构化信息需要通过自定义 error 类型、`errors.Is` / `errors.As` 和 wrapping 额外建立。Go 不是不能做错误治理，而是生态默认路径更偏轻量包装，治理约束需要团队主动设计。

### 两个维度的对比

| 语言 | 语言表达能力 | 生态采纳成本 | 主要原因 |
|------|--------------|--------------|----------|
| Rust | 高 | 中 | 类型系统匹配，但错误生态路径较多，需要团队约定边界 |
| Swift | 高 | 中 | enum / Result 表达自然，但 `throws` 仍是重要生态路径 |
| TypeScript | 中高 | 中 | discriminated union 方便，但运行时需要 schema/tag 补足 |
| C# | 中 | 中 | 泛型和中间件成熟，但异常生态主导，DU 需要模拟 |
| Java | 中 | 中 | cause chain 和框架边界成熟，sealed class 改善分类表达，但异常生态主导 |
| C++ | 中高 | 高 | 类型能力强，但错误处理路径分裂，组织统一成本高 |
| Go | 低中 | 中低 | 类型表达较弱，但显式 `error` 返回高度统一，轻量分类规范容易推广 |

这个表只描述本文方法论的落地摩擦，不评价语言优劣。真正决定错误治理质量的，往往不是语言本身，而是团队是否建立了稳定身份、诊断保留、边界策略和演进规则。

---

## 阶段性小结

到这里，本文已经完成通用方法论部分：错误治理为什么重要、主要矛盾是什么、双通道错误治理模型如何组织失败信息。接下来进入具体设计：在 Rust 工程中，如何把这套模型落成可执行、可测试、可演进的错误治理方案。

---

## 基于 Rust 的错误治理方案

Rust 适合做结构化错误治理，但 Rust 本身并不会自动完成治理。`Result<T, E>`、`enum`、`?` 和 trait 解决的是错误表达与传播的语言机制；稳定身份、跨层语义边界、诊断链保留、边界投影和桥接契约，仍然需要工程层面的设计。

`orion-error` 的定位，就是把前文的双通道错误治理模型落成 Rust 基础设施：

```text
Result<T, StructError<R>>

R                 -> 治理通道的 reason / identity / category
StructError<R>    -> 运行时载体，承载 detail / context / source chain
ExposurePolicy    -> 边界投影策略
report / interop  -> 诊断与外部生态桥接
```

这里的 `R` 不是随意的错误枚举，而是当前语义域的错误分类契约。不同 bounded context、架构层或业务域可以拥有自己的 `Reason` 类型；跨域传播时通过显式转换表达语义边界。

下面这张图表达一个错误从底层失败进入结构化体系、跨语义域传播、最终在边界投影的过程。阅读时只需要抓住三点：

- 内部传播使用统一载体 `StructError<R>`。
- 同一个错误同时携带治理通道和诊断通道。
- 边界层只做策略投影，不重新解释错误。

```mermaid
flowchart TB
    raw["原始失败<br/>IO / DB / Network / Parser"]
    repo["Repository 层<br/>StructError&lt;RepositoryReason&gt;"]
    service["Service 层<br/>StructError&lt;OrderReason&gt;"]
    boundary["系统边界<br/>HTTP / RPC / CLI / Worker"]

    raw -->|"首次进入<br/>source_err(reason, detail)"| repo
    repo -->|"跨语义域<br/>source_err(new_reason, detail)"| service
    service -->|"边界投影<br/>exposure(policy)"| boundary

    subgraph governance["治理通道：稳定、有限、可测试"]
        identity["identity<br/>order.submit_dependency_unavailable"]
        category["category<br/>business / system / config"]
        policy["policy<br/>status / retry / severity / visibility"]
    end

    subgraph diagnostic["诊断通道：保真、可追溯"]
        detail["detail<br/>submit order failed"]
        context["context<br/>operation / order_id / tenant / component"]
        source["source chain<br/>service -> repository -> database"]
    end

    service -.携带.-> identity
    service -.携带.-> category
    service -.携带.-> detail
    service -.携带.-> context
    service -.携带.-> source

    identity --> policy
    category --> policy
    policy --> boundary

    boundary --> user["对外响应<br/>稳定错误码 + 可公开消息"]
    boundary --> log["日志 / Report<br/>诊断摘要 + 脱敏上下文"]
    boundary --> metric["指标 / 告警<br/>identity + category + severity"]
```

图中的关键点是：错误在内部传播时不被压扁成字符串；边界层根据策略投影出用户响应、日志/report 和指标/告警三个视图。

### 五项原则的实现映射

| 方法论原则 | Rust / orion-error 落地方式 | 作用 |
|------------|----------------------------|------|
| 用统一载体承载双通道信息 | 内部跨层传播统一使用 `Result<T, StructError<R>>` | 调用方面对同一种错误形状，分类空间由 `R` 参数化 |
| 让治理通道保持稳定 | 领域 reason 定义稳定 identity、category 和分类语义 | 调用方、监控、协议边界依赖身份，不依赖错误文案 |
| 让诊断通道跨层保真 | 使用 detail、context、source chain 保留底层原因与当前层解释 | 上层可以收敛分类，排障仍能追溯根因 |
| 在边界集中投影 | 通过 exposure policy 统一决定 HTTP/RPC/CLI/log/metric 输出 | 避免每个 handler 各自拼响应、各自决定脱敏 |
| 显式桥接外部生态 | report、redacted render、std error interop、protocol JSON 等路径显式转换 | 每次信息降级、脱敏、暴露都有清楚契约 |

### 设计规则一：按语义域定义 Reason

每个语义域定义自己的 reason 类型，而不是把全系统错误塞进一个巨大的全局 enum。

```rust
#[derive(Debug, Clone, OrionError)]
enum RepositoryReason {
    #[orion_error(identity = "repository.connection_failed")]
    ConnectionFailed,

    #[orion_error(identity = "repository.write_failed")]
    WriteFailed,

    #[orion_error(transparent)]
    General(UnifiedReason),
}

#[derive(Debug, Clone, OrionError)]
enum OrderReason {
    #[orion_error(identity = "order.submit_dependency_unavailable")]
    SubmitDependencyUnavailable,

    #[orion_error(identity = "order.invalid_state")]
    InvalidState,

    #[orion_error(transparent)]
    General(UnifiedReason),
}
```

这对应前文的稳定分类契约。`repository.connection_failed` 是 data access 语义；`order.submit_dependency_unavailable` 是业务语义。两者可以由同一次底层失败触发，但它们不应该混成同一个分类。

### 设计规则二：首次进入时建立结构化错误

普通 IO、数据库、网络、解析错误第一次进入治理体系时，需要同时完成三件事：选择当前层分类、给出当前层 detail、保留底层 source。

```rust
fn insert_order(order: &Order) -> Result<(), StructError<RepositoryReason>> {
    let ctx = OperationContext::doing("insert_order")
        .with_field("order_id", order.id.to_string())
        .with_meta("component.name", "order_repository");

    db_insert(order)
        .source_err(RepositoryReason::ConnectionFailed, "insert order failed")
        .map_err(|err| err.with_context(ctx))?;

    Ok(())
}
```

这一步不应该只把底层错误转成字符串。底层错误是 source，`"insert order failed"` 是 repository 层 detail，`order_id` 和 `component.name` 是 context。

### 设计规则三：跨语义域时建立新边界

如果上层只是同一语义域内的分类收敛，可以只做 reason 转换，不新增错误叙事。如果跨越到新的业务语义域，应建立新的语义边界，并把下层结构化错误作为 source 保留。

```rust
fn submit_order(order: &Order) -> Result<(), StructError<OrderReason>> {
    let ctx = OperationContext::doing("submit_order")
        .with_field("order_id", order.id.to_string())
        .with_field("tenant", order.tenant.to_string());

    insert_order(order)
        .source_err(
            OrderReason::SubmitDependencyUnavailable,
            "submit order failed",
        )
        .map_err(|err| err.with_context(ctx))?;

    Ok(())
}
```

service 层没有把 repository 的连接失败直接暴露给 handler，而是表达自己的业务失败语义：提交订单依赖不可用。repository 错误仍然存在于 source chain 中。

### 设计规则四：边界只做投影，不重新解释错误

HTTP handler、RPC 端点、CLI 入口和 worker 边界不应该重新拼装错误语义。它们应该把结构化错误交给集中策略，统一生成对外响应、日志、指标和调试报告。

```rust
fn handle_submit(req: Request) -> HttpResponse {
    match submit_order(&req.order) {
        Ok(()) => HttpResponse::ok(),
        Err(err) => {
            let snapshot = err.exposure(&DefaultExposurePolicy);
            log_error(err.report());
            HttpResponse::from(snapshot)
        }
    }
}
```

边界可以有多个视图：对用户暴露的是 redacted exposure，对开发者和 SRE 保留的是 report，对监控系统输出的是稳定 identity 和 category。

### 设计规则五：测试错误身份，而不是错误文案

错误治理进入 L2 后，测试应该约束稳定身份和治理决策，而不是约束错误消息。错误消息可以优化、翻译、脱敏；稳定身份和分类语义才是长期契约。

```rust
let err = submit_order(&order).unwrap_err();

assert_eq!(
    err.identity_snapshot().code,
    "order.submit_dependency_unavailable"
);

let exposed = err.exposure(&DefaultExposurePolicy);
assert_eq!(exposed.decision.http_status, 503);
```

这类测试会倒逼团队维护稳定分类契约：新增错误要有身份，修改身份要考虑兼容，边界策略要有明确预期。

如果需要一个可运行的完整示例，可以参考 `orion-error/examples/order_case.rs`。这个示例不是最短 hello world，而是一个小型订单场景：解析层、用户层、存储层和订单服务层各自定义 reason，底层普通错误首次进入结构化体系，跨层传播时通过 reason 收敛保留诊断链，最终在边界统一输出。它对应本文方法论中的统一载体、稳定分类、跨层保留和边界投影四个核心动作。

---

## 工业级应用验证：WarpParse

前文提出的方法论命名为 **双通道错误治理模型**：用稳定治理通道承载分类、身份和策略，用诊断通道保留原因链、上下文和细节，再通过集中边界策略完成投影。

`orion-error` 是这套方法论在 Rust 下的基础设施实现：它解决如何定义稳定身份、如何承载诊断链、如何跨层传播、如何投影和桥接。但基础设施本身还需要在真实工业级系统中验证：高吞吐、长链路、多角色、多边界、强观测要求的系统，才会真正检验错误治理是否可用。

WarpParse 是 Orion 体系中面向高吞吐日志解析与 ETL 场景的核心引擎，也是这套方法论的工业级验证案例。根据 `wp-examples/benchmark/report/report_linux.md` 的 Linux 单机 benchmark，WarpParse 0.12.0 在 Nginx、AWS ELB、Firewall、APT Threat、Mixed Log 五类日志，以及 File -> BlackHole、TCP -> BlackHole、TCP -> File 三种拓扑下，对比 Vector-VRL 0.49.0 取得了纯解析 1.56x-20.30x、解析+转换 1.34x-17.90x 的 EPS 倍数区间。

Benchmark 证明的是 WarpParse 所处场景的工业强度：高吞吐、多格式、多拓扑、解析与转换并存。它本身不证明错误治理质量。错误治理的价值，需要从失败路径能否被定位、分类、投影和自动化处理来判断。

在这类系统中，如果规则语法错误只返回一段字符串，例如：

```text
unexpected token at line 12
```

规则开发者仍然需要打开规则文件、定位行列、猜测出错字段、判断是语法问题还是样本不匹配。系统也很难基于这段文本稳定地区分"配置错误"、"数据质量问题"和"运行时系统错误"。

引入双通道错误治理后，同一次失败应该被表达成结构化信息：

```text
identity : rule.syntax
category : config
detail   : unexpected token in extractor expression
context  : {
  rule_file      : "rules/nginx.wpl",
  line           : 12,
  column         : 18,
  field          : "request_time",
  expected_token : "identifier",
  actual_token   : ")"
}
policy   : block rule activation, show repair hint, do not page SRE
```

这才是方法论在 WarpParse 中被验证的关键：规则开发者拿到的是错误位置和修复线索；运行系统拿到的是稳定错误身份和治理策略；运维侧可以把配置错误、数据错误、系统错误分开统计和告警。吞吐越高，失败路径越需要这种结构化能力，否则系统处理能力越强，错误扩散和排障成本也会被同步放大。

### WarpParse 的错误治理结构

WarpParse 的错误处理不是单点返回错误，而是覆盖规则开发、规则验证、运行时解析、管线执行、边界输出和运维观测的一条完整链路。下面这张图可以按三层阅读：

- 失败来源：规则、样本、类型转换、脏数据、运行时资源。
- 双通道承载：治理通道给身份和策略，诊断通道保留规则、样本和运行时上下文。
- 边界视图：规则开发者、运维、调试工具看到不同投影。

```mermaid
flowchart TB
    sample["样本日志<br/>Nginx / ELB / Firewall / APT / Mixed"]
    rule["WPL 规则<br/>字段提取 / 类型转换 / 富化"]
    check["规则验证<br/>syntax / sample / schema"]
    engine["解析运行时<br/>高吞吐 parse / transform"]
    pipeline["ETL 管线<br/>input -> parse -> transform -> output"]
    boundary["系统边界<br/>CLI / API / worker / report"]

    sample --> check
    rule --> check
    check -->|"规则可用"| engine
    engine --> pipeline
    pipeline --> boundary

    subgraph failure["失败来源"]
        syntax["规则语法错误"]
        mismatch["样本不匹配"]
        typeerr["类型转换失败"]
        dirty["脏数据 / 异常字段"]
        runtime["运行时 I/O / backpressure / resource"]
    end

    syntax --> check
    mismatch --> check
    typeerr --> engine
    dirty --> engine
    runtime --> pipeline

    subgraph governance_wp["治理通道"]
        wp_identity["稳定身份<br/>rule.syntax / parse.mismatch / transform.type / runtime.io"]
        wp_category["category<br/>config / data / system"]
        wp_policy["策略<br/>是否中断 / 是否跳过 / 是否告警 / 是否可重试"]
    end

    subgraph diagnostic_wp["诊断通道"]
        wp_rule_ctx["规则上下文<br/>rule file / line / field / pattern"]
        wp_sample_ctx["样本上下文<br/>sample id / input slice / expected field"]
        wp_runtime_ctx["运行时上下文<br/>source / sink / batch / offset / component"]
        wp_source["source chain<br/>parser -> engine -> pipeline"]
    end

    check -.生成.-> wp_identity
    engine -.生成.-> wp_identity
    pipeline -.生成.-> wp_identity

    check -.保留.-> wp_rule_ctx
    check -.保留.-> wp_sample_ctx
    engine -.保留.-> wp_rule_ctx
    engine -.保留.-> wp_source
    pipeline -.保留.-> wp_runtime_ctx

    wp_identity --> wp_policy
    wp_category --> wp_policy
    wp_policy --> boundary

    boundary --> user_view["规则开发者视图<br/>错误位置 + 修复提示"]
    boundary --> ops_view["运维视图<br/>指标 + 告警 + 失败分类"]
    boundary --> debug_view["调试视图<br/>脱敏上下文 + source chain"]
```

这张图表达的是同一个原则：WarpParse 的高性能解析能力和错误治理能力必须同时存在。`orion-error` 提供错误治理基础设施，WarpParse 则验证了这套方法论在工业级高吞吐 ETL 系统中的可用性。

---

## 面向 AI 的工程化复用

双通道错误治理模型不应该只停留在文档里。更有效的做法，是把方法论、设计原则、crate/lib 使用规范、示例代码、反模式和迁移规则整理成可复用的 engineering skills。Orion 体系中的 skills 会沉淀在 `orion-skills` 仓库中：https://github.com/galaxio-labs/orion-skills

类似地，AI 也可以基于这些 skills 产出项目级错误设计文档。例如 Warp Insight 的错误处理系统设计文档：https://github.com/wp-labs/warp-insight/blob/main/doc/design/foundation/error-handling-system.md 。这类文档的价值不只是记录错误类型，而是让 AI 和工程师围绕同一套治理模型讨论分类、传播、边界输出、观测和迁移。

这样做的价值在于：AI 不再只是根据上下文临时生成几段错误处理代码，而是可以围绕一套明确的治理模型工作。面对一个新项目时，AI 可以先识别错误边界、语义域、稳定身份、诊断链和边界输出，再给出错误治理规划；进入实现阶段时，AI 也可以按照约定使用 `orion-error` 这样的 crate，把 reason 定义、source 保留、context 挂载、exposure 策略和测试断言落到代码里。

换句话说，skills 把错误治理从"靠经验提示 AI"变成了"给 AI 一套工程约束"：

- 规划阶段：识别 L0/L1/L2 现状，设计分类契约和迁移路径。
- 设计阶段：划分语义域，定义 reason、identity、category 和治理属性。
- 实现阶段：选择首次进入、跨层收敛、语义边界包装和边界投影的正确 API。
- Review 阶段：检查是否丢失 source、是否依赖错误文案、是否在 handler 中重复拼响应、是否缺少身份测试。
- 迁移阶段：把字符串错误、临时 enum、泛化包装逐步收敛为稳定的双通道结构。

这对工业级项目尤其重要。错误处理横跨架构、协议、观测、测试和团队规范，单靠一次 prompt 很难稳定完成。把方法论和库约束沉淀为 skills 后，AI 才能在不同项目中持续复用同一套高质量工程判断，既能完成错误治理设计，也能生成更一致、更可维护的实现代码。

---

## 总结

错误处理是系统从原型走向工业级应用的分水岭。原型只需要证明正确路径能够跑通；工业级系统必须在输入变化、依赖退化、配置漂移、数据异常、规则演进和运行环境波动下继续保持可运行、可诊断、可恢复、可演进。

本文提出的 **双通道错误治理模型**，核心是把错误信息拆成两条通道：

- **治理通道**：稳定身份、稳定分类、category、retryable、severity、暴露等级，用于调用方决策、协议投影、监控告警、SLA 统计和长期兼容。
- **诊断通道**：原因链、上下文、细节、底层错误，用于排障、调试、规则修复、运行观测和系统演进。

这两条通道解决的是错误治理的核心矛盾：错误在治理层面必须收敛，否则无法自动化决策；错误在诊断层面必须保真，否则无法定位根因。一个成熟的错误体系不能只追求"包装得漂亮"，也不能只依赖语言机制，而要明确稳定身份如何演进、诊断链如何跨层保留、边界策略如何集中投影、外部生态桥接时保留和丢弃什么。

在 Rust 中，`orion-error` 将这套模型落成可复用的基础设施：`StructError<R>` 承载双通道信息，领域 reason 提供稳定分类契约，source chain 和 context 保留诊断路径，exposure/report/interop 完成不同边界的投影和桥接。`orion-error/examples/order_case.rs` 给出了一个小型可运行例子，而 WarpParse 则提供了工业级验证：在高吞吐 ETL 系统中，错误治理直接影响规则开发体验、运行时可观测性、边界输出质量和长期运维成本。

因此，错误治理不是异常语法的附属品，也不是日志格式的局部优化。它是工业级系统的信息架构之一。只有当失败路径也具备稳定分类、完整诊断、集中投影和可演进契约时，系统才真正从"能跑"走向"可长期运行"。
