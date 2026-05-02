# 大型工程中的错误治理：通用方法论

## 为什么错误治理是一件独立的事

大多数项目把错误处理当作"每个函数自己的事"。每个函数决定怎么处理自己的失败，然后这个决策在下一个函数里被重新做一次。

这种模式在小型项目中可以工作——三五层的调用链，每个人都知道每一层该做什么。但当代码库生长到数十万行、四五层架构、多人协作时，错误处理的碎片化变成最大的技术债之一：

- 同一种失败，A 模块返回字符串，B 模块返回 enum，C 模块直接 panic
- 边界输出时，每一层都在重新拼 JSON，拼出来的结构却不一致
- 排障时，日志里有散落的消息，但没有一条完整的错误路径
- 重构时，不敢动错误类型——因为不知道哪些上层依赖于这个错误的字符串内容

这不仅仅是"写得不好"的问题。**错误治理本身就是一件独立的事**，和业务逻辑一样需要架构设计。

---

## 核心矛盾

任何错误治理方案都要处理一对根本矛盾：

**收敛 vs. 诊断**

- 调用方需要**稳定的、有限的分类**，否则无法做出治理决策（重试、降级、告警、返回给用户）
- 排障方需要**完整的、保留细节的信息**，否则无法定位根因

这对矛盾不只是技术问题，也是一种需求张力。调用方需要稳定、受控、可治理的信息；排障方需要更完整的上下文和根因。

### 不充分的解决方案

| 策略 | 对调用方 | 对排障方 |
|------|---------|---------|
| 只抛技术异常 | 无法治理 | 信息完整 |
| 只抛业务错误 | 可以治理 | 丢失根因 |
| 字符串链式包装 | 无法治理 | 可读但不可结构化查询 |
| 吞掉错误 | 干净 | 丢失所有信息 |

### 分离到两个维度

核心方法论：**把"分类信息"和"诊断信息"分离到两个不同的维度，通过两个不同的通道传递。**

```
错误 = 分类（枚举变体）+ 诊断（source chain + context + detail）
```

调用方只看分类。排障方可以访问完整的诊断信息。

同一个错误可以通过不同视图同时服务两类需求，减少调用方和排障方互相牺牲。

---

## 五个设计原则

### 原则一：统一载体

**每条错误路径应该使用相同的运行时载体。**

反例：

```rust
// A 模块返回 io::Error
fn read_file() -> io::Result<Data>

// B 模块返回自定义 enum
fn validate() -> Result<Data, ValidationError>

// C 模块返回字符串
fn process() -> Result<Data, String>
```

每条错误路径的调用者都需要学习一套新的错误类型。组合两个不同函数的错误路径几乎不可管理。

正例：

```rust
// 所有错误路径返回同一载体
fn read_file() -> Result<Data, StructError<AppReason>>
fn validate() -> Result<Data, StructError<AppReason>>
fn process() -> Result<Data, StructError<AppReason>>
```

载体是统一的。变化的只有分类枚举。

### 原则二：分类稳定

**错误分类枚举应该比业务代码更稳定。**

错误的分类体系是契约——调用方依赖它做治理决策。如果分类经常变化，所有治理决策（网关路由、告警规则、监控面板）都需要同步更新。

| 应该稳定的 | 可以变化的 |
|-----------|-----------|
| 分类枚举变体 | 错误信息文案 |
| 稳定 identity code | detail 内容 |
| category（业务/系统/配置） | 具体的技术细节 |

稳定分类的另一个好处：它是人和系统之间的共享接口。运维配置告警规则、网关配置状态码映射、API 文档描述错误响应——这些全部依赖于稳定分类，而不是依赖于错误文本。

### 原则三：跨层保留

**错误在内部传播时应该追加信息，不应破坏已有诊断链。**

反例：

```rust
fn repository() -> Result<Data, RepoError> {
    // 数据库连接失败，返回 RepoError::ConnectionFailed
}

fn service() -> Result<Data, ServiceError> {
    let data = repository()?;  // 丢弃了 RepoError 的具体信息
    Ok(data)
}
```

正例：

```rust
fn repository() -> Result<Data, StructError<RepoReason>> {
    // 数据库连接失败
}

fn service() -> Result<Data, StructError<ServiceReason>> {
    let data = repository()
        .source_err(ServiceReason::DependencyFailed, "load repository data failed")?;
    Ok(data)
}
```

每层保留的信息形成一条完整的错误链。排障时可以从最终错误追溯到原始根因。如果当前层只是改变 reason 类型，不建立新的语义边界，则使用 `conv_err()`；如果当前层要表达新的失败语义，则使用 `source_err(...)` 把下层结构化错误作为 source 保留下来。边界输出时再按策略做 redaction 和 projection。

### 原则四：集中决策

**边界暴露策略应该集中定义，而不是在每个边界点重新决定。**

反例：

```rust
// handler A
match err {
    AppError::NotFound => HttpResponse::new(404, "not found"),
    AppError::Timeout => HttpResponse::new(503, "try again"),
}

// handler B
match err {
    AppError::NotFound => HttpResponse::new(404, "resource missing"),
    AppError::Timeout => HttpResponse::new(504, "gateway timeout"),
}
```

两个 handler 对同一个错误的输出不一致。

正例：

```rust
// 策略集中定义
impl ExposurePolicy for MyPolicy {
    fn http_status(&self, identity: &ErrorIdentity) -> u16 {
        match identity.code.as_str() {
            "biz.not_found" => 404,
            "sys.timeout" => 503,
            _ => 500,
        }
    }
}

// 所有边界点使用同一策略
handler.err.exposure(&policy).to_http_error_json()
```

### 原则五：显式桥接

**进入外部生态（日志系统、标准错误接口、第三方库）应该是显式的。**

反例：

```rust
// 隐式实现了 std::error::Error，调用者可以在不知情的情况下擦除类型
fn handle(err: Box<dyn Error>)  // 擦除了错误的结构化信息
```

正例：

```rust
// 需要显式选择进入外部生态
let std_ref = err.as_std();
let boxed: Box<dyn Error> = err.into_boxed_std();
```

显式桥接确保结构化信息的丢失是有意为之，不是无意遗漏。

---

## 错误传播的三种模式

### 首次进入

原始错误（IO 错误、解析错误、网络错误）第一次进入结构化系统。此时需要：

1. 选择合适的分类（业务 vs 系统 vs 配置）
2. 提供当前层能给出的解释（detail）
3. 保留原始错误作为 source

### 跨层转换

上层需要将下层的错误分类收敛到自己的分类空间。此时：

- 如果只是分类重新映射，保留所有诊断信息
- 如果要建立新的语义边界，将下层错误作为 source 包裹

这取决于当前层是否是一个新的语义边界。

### 边界输出

在系统边界（HTTP handler、RPC 端点、CLI 入口、日志写入点）输出错误。此时：

1. 选择输出格式（JSON、文本、结构化日志）
2. 应用暴露策略（哪些信息可以对外暴露）
3. 输出

---

## 治理等级

一个团队的错误治理成熟度可以分为四个等级：

**L0：无治理**

- 错误类型散乱：`std::io::Error` / `String` / `Box<dyn Error>` / 自定义 enum 混用
- 边界输出拼接字符串
- 排障依赖 grep 日志

**L1：统一载体**

- 所有错误路径返回同一载体类型
- 但分类随意，相同的失败在不同模块归类不一致
- 有 source chain，但 source 经常在跨层时被丢弃

**L2：稳定分类**

- 分类枚举稳定，有文档定义
- 边界输出有统一策略（即使同一策略）
- source chain 在跨层传播中完整保留
- 测试中断言错误身份，而不是断言错误消息

**L3：治理驱动**

- 错误分类直接映射到治理动作（重试、降级、告警、SLA 计算）
- 边界策略可配置，不同环境可不同
- 错误指标进入监控系统
- 新错误类型需要 review 才能加入

大多数团队在 L0 和 L1 之间。orion-error 帮助团队到达 L2。

---

## 不适用场景

这套方法论不是万能的。以下场景不适合：

1. **小型项目、原型、脚本。** 三五层的调用链没有必要引入分层治理。
2. **性能极端敏感的场景。** 结构化错误路径会有分配、source/context 采集和序列化成本；Rust 泛型本身通常不是运行时开销，但可能增加编译时间和代码体积。
3. **错误不需要跨层传播。** 如果所有错误都在一层内处理完毕，这套方法论的收益接近于零。

---

## 各语言实现可行性

这套方法论对语言的依赖主要在三个方面：**代数类型**（reason 分类是否穷尽）、**泛型**（载体能否参数化）、**错误传递方式**（返回值 vs 异常）。不同语言在这三个维度上的匹配度不同。

### Rust — 原生匹配

Rust 同时满足三个条件：
- 代数类型（`enum`）表达 reason 分类，`match` 提供穷尽性检查
- 泛型（`StructError<T>`）提供类型安全的载体参数化
- 无异常机制，错误通过返回值传递，自然与载体配合

### TypeScript — 亲和度高

```typescript
type AppReason =
  | { kind: "not_found"; id: string }
  | { kind: "system_error" };
```

Union type + discriminated union 天然适合 reason 分类。`neverthrow`、`fp-ts` 的 `Either` 等库提供了返回值式错误处理。弱点：运行时无泛型类型信息。

### Swift — 亲和度高

代数类型（enum with associated values）表达 reason 分类。`Result<T, E>` 在 Swift 5.0+ 中原生支持。社区中有使用 `Result` 替代 `throws` 的实践。

### C# — 中等亲和度

泛型支持良好（运行时保留类型信息），但异常机制主导生态。缺少 discriminated union（可用 `OneOf` 库模拟）。最适合的映射方式：用异常类型层次做分类、用 ASP.NET Core 中间件做集中策略。

### Java — 不自然但可行

泛型擦除，异常机制主导。但 Java 的 cause chain（`Throwable.initCause()`）比 source chain 历史更早。Spring 的 `@ControllerAdvice` 已经是集中策略的成熟模式。最适合：借鉴思想，映射到现有机制。

### C++ — 技术可行，生态无约定

模板保留类型信息，`std::expected`（C++23）提供类似 `Result` 的机制。但错误处理四分五裂（异常、错误码、expected、boost::outcome），无主导载体。

### Go — 最困难

`error` 接口默认只要求 `Error() string`，结构化信息需要通过自定义 error 类型、`errors.Is` / `errors.As` 和 wrapping 额外建立。Go 不是不能做错误治理，而是生态默认路径更偏轻量包装，治理约束需要团队主动设计。

### 综合排名

| 语言 | 亲和度 | 代数类型 | 泛型 | 返回值式错误 |
|------|--------|---------|------|------------|
| Rust | ★★★★★ | enum + match | 完整 | 原生 |
| Swift | ★★★★☆ | enum + associated values | 完整 | Result 原生 |
| TypeScript | ★★★★☆ | union type | 完整（仅编译时） | 常见（neverthrow） |
| C# | ★★★☆☆ | 无（OneOf 可模拟） | 完整（运行时保留） | 非常见（异常主导） |
| Java | ★★☆☆☆ | 无 | 擦除 | 非常见（异常主导） |
| C++ | ★★☆☆☆ | 无 | 完整 | std::expected（C++23） |
| Go | ★☆☆☆☆ | 无 | 可用但不惯例 | 原生但无结构化载体 |

---

## 总结

大型工程中的错误治理，本质上是一个**信息架构问题**——而不是一个异常处理语法问题。

争论"checked exception vs unchecked exception"或"Result vs try-catch"是在语法层面解决问题。真正的挑战是：

- 错误信息如何组织（分类 vs 诊断分离）
- 错误信息如何在层间传递（跨层保留原则）
- 错误信息如何输出（边界集中决策）

这三个问题在 Rust、Java、Go、C++ 中都存在，与具体语言无关。任何语言的大型项目都可以（也应该）建立自己的错误治理架构——不管用什么语法来表达它。
