# 双通道：工业级系统的错误治理模型 —— PPT 稿

---
## 第 1 页 · 封面

**双通道：工业级系统的错误治理模型**

—— 如何让错误在治理层面收敛、在诊断层面保真

Orion 体系 · 错误治理方法论

---
## 第 2 页 · 本文结构

**三层结构：**

| 层次 | 内容 | 解决的问题 |
|------|------|-----------|
| 方法论层 | 错误处理为什么重要、核心矛盾、五项原则 | 为什么要治理 |
| 工程落地层 | Rust `orion-error` 实现：稳定身份、诊断链、边界投影、桥接 | 怎么落地 |
| 工业验证层 | WarpParse + AI skills 的工业级验证 | 是否可用 |

**快速主线（四段）：**
1. 核心矛盾：收敛 vs. 诊断
2. 解决方案：双通道错误治理模型
3. Rust 落地：`orion-error`
4. 工业验证：WarpParse

---
## 第 3 页 · 错误处理：原型与工业级的分水岭

**原型只需要正确路径跑通；工业级需要在非理想条件下：**

- 可运行
- 可诊断
- 可恢复
- 可演进

**现实：系统不会长期运行在理想条件下**

输入变化 → 依赖退化 → 网络抖动 → 配置漂移 → 数据脏状态 → 业务规则迭代

> 错误不是"正常逻辑之外的意外文本"，而是系统在非理想条件下继续运行、恢复状态、决定对外响应、支持诊断时必须传递的信息。

---
## 第 4 页 · 失去治理的代价

**错误信息没有统一形态，跨团队/跨系统/跨边界传播时：**

| 现象 | 后果 |
|------|------|
| 同一失败，A 模块返回字符串，B 返回 enum，C 直接 panic | 上层无法统一处理 |
| 每层重新拼 JSON，结构不一致 | 边界输出混乱 |
| 日志中有散落消息，没有完整错误路径 | 排障困难 |
| 不敢动错误类型——不知道谁依赖了错误字符串 | 重构风险高 |

> 错误治理定义了失败发生后系统如何保留信息、跨层传递、对外暴露、支撑诊断和演进。它不是业务逻辑的装饰，而是业务逻辑失败时的信息架构。

---
## 第 5 页 · 行业探索：各语言的错误处理

| 语言 | 机制 | 痛点 |
|------|------|------|
| C | 返回码 + `errno` | 信息分散，易漏检 |
| Java | 异常机制（checked/unchecked） | 层次膨胀，边界语义不清 |
| Go | 显式返回 `error` | 易退化为层层包装的字符串 |
| Rust | `Result<T,E>` + `?` + enum | 分类、边界暴露仍需工程层面设计 |

> 语言机制能降低错误处理成本，却不能替代错误治理本身。真正需要解决的不是选择异常还是 `Result`，而是失败信息如何被分类、保留、转换、暴露和观测。

---
## 第 6 页 · 来自优秀项目的证据

| 项目 | 做法 | 启发 |
|------|------|------|
| gRPC | 跨语言 RPC 失败收敛为标准状态码 | 稳定分类让调用方可重试、降级、告警 |
| PostgreSQL | 稳定 SQLSTATE 错误码 | 机器契约和人类文案应该分离 |
| Kubernetes | condition 写入 `status` | 错误可以是可查询、可自动化的系统状态 |
| Terraform | diagnostics：severity + summary + detail + path | 错误应指出位置、原因和修复方向 |
| rustc | 错误码 + 位置 + label + note + help | 诊断信息本身是产品体验 |
| Envoy | access log response flags | 边界层错误应能被聚合、搜索、告警 |

> 方向一致：优秀的错误处理都把失败路径设计成稳定的信息系统。

---
## 第 7 页 · 核心矛盾：收敛 vs. 诊断

**一对根本矛盾：**

```
调用方需要                         排障方需要
稳定、有限的分类          ←→        完整、保留细节的信息
（用于重试/降级/告警）              （用于定位根因）
```

**两种极端：**

- 暴露过多技术细节 → 上层依赖底层实现，重构时错误契约被破坏
- 只保留业务分类 → 排障时失去根因："原始失败是什么？经过了哪些层？"

> 核心问题：如何同时让错误在治理层面收敛，在诊断层面保真。

---
## 第 8 页 · 不充分的解决方案

| 策略 | 对调用方 | 对排障方 |
|------|---------|---------|
| 只抛技术异常 | 无法治理 | 信息完整 |
| 只抛业务错误 | 可以治理 | 丢失根因 |
| 纯字符串链式包装 | 无法治理 | 可读但不可结构化查询 |
| 保留类型信息的链式包装 | 可做局部治理 | 保留原因链，但分类与边界仍需额外约束 |
| 吞掉错误 | 干净 | 丢失所有信息 |

> 单一形态同时满足两类需求 → 牺牲其中一边。

---
## 第 9 页 · 砖块不等于建筑

**Java 的异常 + 错误码、Go 的 sentinel error + wrapping、Rust 的 enum + cause chain —— 都是砖块，但砖块不等于建筑。**

**三个根本问题：**

1. **身份不稳定**：异常类型承担分发角色，继承层次重构时会变
2. **分类空间天然膨胀**："一种失败一个类" → 无限制增长，无法收敛到有限分类
3. **治理和诊断共用同一通道**：互相牵制，治理动作散落在每个 handler 中

**建筑需要四根承重墙：**

- 稳定身份（不受类型重构影响）
- 有限分类空间（受兼容演进规则约束）
- 诊断保留（跨层不丢失）
- 集中边界策略（不在 handler 中重复决策）

---
## 第 10 页 · 双通道错误治理模型

**核心方法论：把分类信息和诊断信息分离到两个维度，通过两个通道传递。**

```
错误 = 稳定身份 + 稳定分类 + 诊断链 + 上下文 + 细节
```

| 通道 | 包含什么 | 服务谁 | 稳定性要求 |
|------|---------|--------|------------|
| **治理通道** | 稳定身份、稳定分类、category、retryable、severity、暴露等级 | 调用方、网关、监控、运维策略、协议客户端 | 高，应被文档化和测试约束 |
| **诊断通道** | 原因链、操作上下文、关键字段、动态细节、底层错误 | 开发者、SRE、排障工具、日志系统 | 可动态变化，但须保真且可追溯 |

---
## 第 11 页 · 错误组成部分

| 组成部分 | 含义 | 例子 |
|----------|------|------|
| 稳定身份 | 机器可判读的错误主键，面向长期兼容 | `order.not_found`、`system.timeout` |
| 稳定分类 | 面向治理决策的有限类别 | 业务错误、配置错误、系统错误 |
| 治理属性 | 从稳定身份和分类派生的辅助决策字段 | category、retryable、severity、暴露等级 |
| 诊断链 | 跨层传播时保留的 cause/source 路径 | service → repository → database timeout |
| 上下文 | 当前操作的结构化环境 | operation、tenant、order_id、component |
| 细节 | 当前层对这次失败的具体解释 | `read config failed`、`upstream returned 503` |

> 同一个错误通过不同视图同时服务两类需求，减少调用方和排障方互相牺牲。

---
## 第 12 页 · 五项原则总览

**双通道模型落地需要五项原则配合：**

| 原则 | 核心要求 |
|------|---------|
| 原则一：统一载体 | 自有跨层路径使用统一的结构模型 |
| 原则二：治理通道稳定 | 错误分类契约按向后兼容规则演进 |
| 原则三：诊断通道保真 | 跨层传播追加信息，不破坏已有诊断链 |
| 原则四：边界集中投影 | 边界暴露策略集中定义，不在每个边界点重新决定 |
| 原则五：显式桥接外部 | 进入外部生态应是显式的，信息丢失是有意为之 |

---
## 第 13 页 · 原则一：统一载体

**自有跨层错误传播路径应使用统一的结构模型。**

反例 —— 三种函数返回三种形状：
```rust
fn read_file() -> io::Result<Data>
fn validate() -> Result<Data, ValidationError>
fn process()  -> Result<Data, String>
```

正例 —— 统一载体，参数化分类空间：
```text
read_file() -> Result<Data, StructuredError<ErrorClass>>
validate()  -> Result<Data, StructuredError<ErrorClass>>
process()   -> Result<Data, StructuredError<ErrorClass>>
```

> 变化的是分类空间和上下文，不是错误形状本身。统一载体不是要求第三方库全改成同一种类型，而是团队控制的内部传播路径使用同一种结构模型。

---
## 第 14 页 · 原则二：治理通道保持稳定

**错误分类契约应按向后兼容规则演进。**

**兼容规则：**
- 可以新增错误身份或分类
- 不应删除已对外承诺的错误身份
- 不应改变已有身份的语义
- 不应让同一身份在不同边界产生矛盾的治理动作
- 可以调整错误文案、诊断细节、上下文字段

| 应该稳定的 | 可以变化的 |
|-----------|-----------|
| 稳定错误身份 | 诊断细节 |
| 分类语义 | 错误信息文案 |
| category（业务/系统/配置） | 具体的技术细节 |

---
## 第 15 页 · 原则三：诊断通道跨层保真

**错误在内部传播时应追加信息，不应破坏已有诊断链。**

反例 —— 丢弃下层信息：
```text
service() -> Result<Data, ServiceError> {
    data = repository()?  // RepositoryConnectionFailed 的具体信息丢失
}
```

正例 —— 保留下层错误为 cause：
```text
service() -> Result<Data, StructuredError<ServiceClass>> {
    data = repository()
        .wrap_as_cause(ServiceDependencyFailed, "load repository data failed")
}
```

**判断标准：是否跨越语义域？**
- 同语义域：分类收敛 + 诊断保留
- 跨语义域：建立新语义边界，下层错误作为 cause 保留

---
## 第 16 页 · 原则四：边界集中投影

**边界暴露策略应集中定义，不在每个边界点重新决定。**

```
StructuredError<ErrorClass>
    -> error_identity
    -> exposure_policy
    -> HTTP / RPC / CLI / log / metric
```

集中策略覆盖：
- 对外错误码和用户可见消息
- HTTP/RPC/CLI 格式映射
- 日志级别和结构化日志字段
- 是否触发告警或计入 SLA
- 是否建议调用方重试、降级
- 诊断信息脱敏和暴露等级

> 两个 handler 对同一错误返回不同状态码 = 治理通道稳定性被破坏。

---
## 第 17 页 · 原则五：显式桥接外部生态

**进入外部生态应是显式的，信息丢失是有意为之而非无意遗漏。**

反例：
```text
handle(error_as_text)  // 擦除了结构化信息，调用者不知情
```

正例：
```text
plain_error = err.to_plain_error()
log_record  = err.to_log_record(redaction_policy)
```

**每个桥接函数应有清楚契约：**
- 目标消费者是谁？
- 保留什么？—— 身份、分类、原因链摘要、上下文
- 丢弃什么？—— 内部实现类型、敏感字段、过长底层错误
- 脱敏什么？—— token、密钥、用户隐私、内部拓扑
- 如何降级？—— 目标生态只能接收字符串时压缩什么

---
## 第 18 页 · 错误传播的三种模式

**错误传播不是机械向上抛，经历三种动作：**

| 模式 | 动作 | 要点 |
|------|------|------|
| **首次进入** | 原始错误进入结构化体系 | ① 选择分类 ② 给出 detail ③ 保留 source |
| **跨层转换** | 下层分类收敛到上层分类空间 | 同语义域收敛 / 跨语义域包裹 |
| **边界输出** | 在系统边界投影 | 选择输出格式，应用暴露策略 |

**三个诊断概念的分工：**
- `source/cause` → 回答根因是什么
- `context` → 回答在哪、对谁、执行什么
- `detail` → 回答当前层如何理解这次失败

---
## 第 19 页 · 完整传播示例

**"提交订单"失败的三层传播：**

**Repository 层 —— 首次进入：**
```text
identity: "repository.connection_failed"
detail: "insert order failed"
context: { operation: "insert_order", order_id, component: "order_repository" }
source: 原始数据库错误
```

**Service 层 —— 跨语义域转换：**
```text
identity: "order.submit_dependency_unavailable"
detail: "submit order failed"
context: { operation: "submit_order", order_id, tenant }
source: Repository 层的 StructError（保留完整诊断链）
```

**HTTP Handler —— 边界投影：**
```text
治理通道 → HTTP 503 + 用户提示 + 重试建议 + 指标标签
诊断通道 → 日志中保留完整 cause chain + context
```

---
## 第 20 页 · 治理成熟度等级

| 等级 | 名称 | 特征 |
|------|------|------|
| **L0** | 无治理 | 错误类型散乱；边界拼接字符串；排障依赖 grep |
| **L1** | 统一载体 | 同一结构模型，但分类随意，原因链常被丢弃 |
| **L2** | 稳定分类 | 分类契约稳定、有文档；边界策略统一；测试断言错误身份 |
| **L3** | 治理驱动 | 分类直接映射治理动作；策略可配置；新错误需 review |

**大多数团队在 L0 和 L1 之间。**

---
## 第 21 页 · 从 L1 到 L2：最被低估的一步

**不是把返回类型换成统一载体就结束了。**

**需要五个动作：**
1. 标准化分类契约 —— 明确稳定身份、分类语义、category 和治理含义
2. 梳理存量错误 —— 把散落的字符串、技术异常迁移到稳定分类
3. 建立边界策略 —— 统一 HTTP/RPC/CLI/log/metric 投影规则
4. 建立测试规范 —— 断言错误身份和治理决策，而非错误消息
5. 建立评审习惯 —— 新增错误时讨论语义归属，而非只讨论能否编译

> L1 到 L2 不是局部重构，而是团队协作模式的变化：错误分类从个人实现细节变成共享工程语言。

---
## 第 22 页 · 不适用场景

**双通道治理不是银弹：**

1. **小型项目、原型、脚本**
   - 边界少、生命周期短，局部处理即可

2. **性能极端敏感的场景**
   - 结构化路径有分配、原因链、序列化等成本

3. **错误不需要跨层传播**
   - 所有错误都在一层内处理完毕，收益接近于零

---
## 第 23 页 · 语言机制与生态采纳

**方法论语言无关，落地成本因语言而异。两个维度：**

- **语言表达能力**：是否方便表达稳定分类、结构化载体、原因链、边界投影
- **生态采纳成本**：团队在既有生态中采用这套治理的组织和迁移成本

| 语言 | 表达能力 | 采纳成本 | 主要原因 |
|------|---------|---------|----------|
| Rust | 高 | 中 | 类型系统匹配，但错误生态路径较多 |
| TypeScript | 中高 | 中 | discriminated union 方便，运行时需 schema |
| Swift | 高 | 中 | enum/Result 表达自然 |
| Java | 中 | 中 | sealed class 改善分类表达，异常生态成熟 |
| Go | 低中 | 中低 | 类型表达弱，但显式 error 返回高度统一 |

> 真正决定治理质量的往往不是语言本身，而是团队是否建立了稳定身份、诊断保留、边界策略和演进规则。

---
## 第 24 页 · Rust 落地：orion-error

**Rust 适合结构化错误治理，但 Rust 不会自动完成治理。**

```text
Result<T, StructError<R>>

R                 -> 治理通道：reason / identity / category
StructError<R>    -> 运行时载体：detail / context / source chain
ExposurePolicy    -> 边界投影策略
report / interop  -> 诊断与外部生态桥接
```

**五项原则的 Rust 映射：**

| 原则 | Rust / orion-error 落地方式 |
|------|---------------------------|
| 统一载体 | `Result<T, StructError<R>>` 统一跨层传播 |
| 治理通道稳定 | 领域 reason 定义稳定 identity、category |
| 诊断通道保真 | detail、context、source chain 保留 |
| 边界集中投影 | ExposurePolicy 统一决定输出 |
| 显式桥接 | report、redacted render、std error interop |

---
## 第 25 页 · Rust 设计规则

**规则一：按语义域定义 Reason**
```rust
enum RepositoryReason {
    #[orion_error(identity = "repository.connection_failed")]
    ConnectionFailed,
    #[orion_error(identity = "repository.write_failed")]
    WriteFailed,
}
enum OrderReason {
    #[orion_error(identity = "order.submit_dependency_unavailable")]
    SubmitDependencyUnavailable,
}
```

**规则二：首次进入时建立结构化错误** —— 同时完成：选择分类 + 给出 detail + 保留 source

**规则三：跨语义域时建立新边界** —— 同域收敛，跨域包裹

**规则四：边界只做投影** —— 交给集中策略，不重新解释错误

**规则五：测试错误身份，不是错误文案**
```rust
assert_eq!(err.identity_snapshot().code, "order.submit_dependency_unavailable");
assert_eq!(exposed.decision.http_status, 503);
```

---
## 第 26 页 · Java 映射方案

**核心：每个语义域定义一个 sealed class，域之间无继承关系。**

```java
// data access 语义域
public sealed abstract class RepositoryError extends RuntimeException
    permits RepositoryError.ConnectionFailed, RepositoryError.WriteFailed {
    public abstract String identity();  // "repository.connection_failed"
    public abstract String category();  // "system"
    public abstract boolean retryable();
}

// order 业务语义域 —— 独立类型，不继承 RepositoryError
public sealed abstract class OrderError extends RuntimeException
    permits OrderError.DependencyUnavailable, OrderError.InvalidState {
    // ...
}
```

**跨域转换：构造新域异常，把旧域异常作为 cause 保留**
```java
catch (RepositoryError e) {
    throw new OrderError.DependencyUnavailable("submit order failed", e, ctx);
}
```

| 概念 | Rust | Java |
|------|------|------|
| 语义域分类 | `enum RepositoryReason` | `sealed class RepositoryError` |
| 治理通道 | reason + identity 字符串 | 子类覆写 `identity()` / `category()` |
| 诊断通道 | detail / context / source | `getMessage()` / `context()` / `getCause()` |
| 统一载体 | `StructError<R>` 泛型参数化 | 不可行 —— JLS 禁止泛型类继承 Throwable |

---
## 第 27 页 · 工业验证：WarpParse

**WarpParse：Orion 体系中面向高吞吐日志解析与 ETL 的核心引擎。**

Benchmark 数据（WarpParse 0.12.0 vs Vector-VRL 0.49.0）：
- 纯解析：1.56x - 20.30x EPS 倍数
- 解析+转换：1.34x - 17.90x EPS 倍数

**没有结构化错误时：**
```text
unexpected token at line 12
→ 开发者需打开文件、定位行列、猜测出错字段
```

**引入双通道治理后：**
```text
identity : rule.syntax
category : config
context  : { rule_file, line: 12, column: 18, field: "request_time", ... }
policy   : block rule activation, show repair hint, do not page SRE
```

> 吞吐越高，失败路径越需要结构化能力，否则处理能力越强，错误扩散和排障成本也被同步放大。

---
## 第 28 页 · 面向 AI 的工程化复用

**方法论 + 设计原则 + 库使用规范 → 可复用的 Engineering Skills**

Skills 把错误治理从"靠经验提示 AI"变成"给 AI 一套工程约束"：

| 阶段 | AI 可做的事情 |
|------|-------------|
| 规划 | 识别 L0/L1/L2 现状，设计分类契约和迁移路径 |
| 设计 | 划分语义域，定义 reason、identity、category |
| 实现 | 选择正确的 API：首次进入、跨层收敛、边界投影 |
| Review | 检查：丢失 source？依赖错误文案？handler 中拼响应？缺少身份测试？ |
| 迁移 | 字符串错误、临时 enum → 稳定双通道结构 |

> AI 不再只是临时生成几段错误处理代码，而是围绕一套明确的治理模型工作。

---
## 第 29 页 · 总结

**错误处理是系统从原型走向工业级应用的分水岭。**

**双通道错误治理模型：**

```
治理通道                          诊断通道
───────                          ───────
稳定身份 + 稳定分类               原因链 + 上下文 + 细节
↓                                ↓
调用方决策                        排障定位
协议投影                          调试修复
监控告警                          运行观测
SLA 统计                          系统演进
长期兼容                          可追溯
```

**四根承重墙：**
1. 稳定身份 —— 不受类型重构影响
2. 有限分类空间 —— 受兼容演进规则约束
3. 诊断保留 —— 跨层不丢失
4. 集中边界策略 —— 不在 handler 中重复决策

> 只有当失败路径也具备稳定分类、完整诊断、集中投影和可演进契约时，系统才真正从"能跑"走向"可长期运行"。

---
## 第 30 页 · 谢谢

**双通道：工业级系统的错误治理模型**

- 方法论：https://github.com/galaxio-labs/orion-skills
- 工业验证：WarpParse
- 可运行示例：`orion-error/examples/order_case.rs`
