# Wukong 错误治理模型：从方法论到工程落地

> 上篇讲了契约通道与诊断通道的分离、五项原则和成熟度等级。这篇回答一个问题：方法论怎么在真实系统里落地？

---

## Rust 不会自动完成治理

Rust 的类型系统天然适合错误治理。代数类型表达分类，`match` 提供穷尽检查，泛型参数化载体，`Result<T, E>` 让错误走返回值而非异常。这些都跟 Wukong 模型的结构化思路高度匹配。

但 Rust 只解决了"怎么表达失败"。`enum` 能列举错误变体，不等于变体之间有稳定的语义边界；`?` 能向上传播，不等于原因链会被保留；`thiserror`、`anyhow`、`eyre` 能快速生成错误类型，不等于团队对"哪些失败共享同一标识"有共识。

工具给的是砖块，不是建筑。下面五条是在 Rust 里把 Wukong 模型落成工程约束的具体做法。

---

## 约束一：按语义域定义 Reason，而非全局大枚举

最常见的错误是：一个 `AppError` enum 包揽全系统所有失败。业务错误、数据库错误、解析错误全塞在一起，变体数量跟着模块增长无限制膨胀。

正确的做法是：每个语义域定义自己的 `Reason` 类型。`RepositoryReason`、`OrderReason`、`ParserReason` 各自在自己的边界内约束分类空间。

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

关键点：`RepositoryReason` 和 `OrderReason` 是两个独立类型，不存在继承关系。`repository.connection_failed` 是 data access 语义，`order.submit_dependency_unavailable` 是业务语义。两者可能由同一个数据库超时触发，但调用方应该依赖不同的标识做不同的治理决策——前者触发重试和数据库连接池告警，后者触发降级和业务告警。

语义域的划分依据不是文件目录，而是**调用方是否需要对这两类失败做不同处理**。如果上层对所有失败的处理逻辑一致，就不要拆域——多一个域就多一套转换代码，不是越细越好。

---

## 约束二：首次进入即结构化

底层错误（IO、数据库、网络、解析）第一次进入你的代码时，就应当完成结构化，而不是等到上层再包装。

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

三个动作同时完成：选分类（`ConnectionFailed`）、给当前层解释（`"insert order failed"`）、保留原始错误为 source。不把底层错误转成字符串——转了，排障时数据库 error code、连接池状态这些信息就丢了。

常见的反面做法是早期用 `anyhow::Context` 加个字符串 context 就往上抛。`anyhow` 适合快速原型和一次性脚本，不适合需要稳定分类和完整诊断链的工业级路径。

---

## 约束三：跨语义域建立新边界，保留下层为 source

这是最容易出错的地方。错误从 repository 传到 service 时，是直接暴露底层分类，还是包装成新的业务语义？

判断依据：**当前层是否跨越了语义域？**

- 同语义域内（database driver → query executor → repository helper，都在 data access 域）：只做 reason 收敛，不建立新语义边界。用 `conv_err()` 把下层 reason 映射到当前 reason。
- 跨语义域（data access → order service）：建立新语义边界，把下层结构化错误作为 source 保留。用 `source_err(...)`。

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

这里 service 不把 `RepositoryReason::ConnectionFailed` 暴露给 handler，而是表达业务失败：`order.submit_dependency_unavailable`。repository 的完整错误仍在 source chain 中，排障时可以一路追溯到数据库超时的具体原因。

最常见的反模式是 `return Err(ServiceError::DependencyFailed)` 而丢弃了下层错误——根因永远丢失。另一种是直接把 `RepositoryReason::ConnectionFailed` 透传给上层——调用方（比如 HTTP handler）被迫了解数据库连接失败的含义，底层一重构，错误契约就崩了。

---

## 约束四：边界只做输出，不重新解释错误

HTTP handler、RPC endpoint、CLI 入口、worker 边界——这些地方不应该重新决策"这个错误该返回什么状态码"。决策应该由集中的 exposure policy 完成。

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

集中策略的意义：同一个 `system.timeout` 错误，在十个 handler 里各自 `match` 决策，总有一天 A handler 返回 503、B handler 返回 504。集中定义后，一处修改、处处一致。handler 只做两件事：传错误、取结果。

边界输出有多个视图：对用户返回 redacted exposure（脱敏后的错误码和可公开消息），对开发者通过 report 保留完整诊断摘要，对监控输出稳定 identity 和 category。

---

## 约束五：测试错误标识，不测试错误文案

这是进入 L2 成熟度的标志。错误消息会优化、翻译、脱敏；错误标识才是长期契约。

```rust
let err = submit_order(&order).unwrap_err();

assert_eq!(
    err.identity_snapshot().code,
    "order.submit_dependency_unavailable"
);

let exposed = err.exposure(&DefaultExposurePolicy);
assert_eq!(exposed.decision.http_status, 503);
```

这类测试倒逼团队维护稳定分类契约：新增错误要有标识，修改标识要考虑兼容，边界输出策略要有明确预期。没有这种测试，标识就只是注释里的约定，重构时随时会被破坏。

---

## 错误传播的完整生命周期

以上五条约束定义的是静态结构。运行时，一个错误经历六个阶段：

```
detect → classify → enrich → propagate → output → observe → review/evolve
```

- **detect**：在失败发生处捕获原始错误或业务失败。
- **classify**：选择当前语义域下的稳定错误标识和分类。
- **enrich**：追加 detail、context 和 source，不污染契约通道。
- **propagate**：跨层传播时保留诊断链，必要时建立新语义边界。
- **output**：在 HTTP/RPC/CLI/log/metric 等边界按策略生成输出视图。
- **observe**：通过日志、指标、trace 观察错误分布和治理效果。
- **review/evolve**：根据生产反馈合并、废弃或新增错误标识，调整策略。

最后一步很容易被忽略。错误分类不是一次性建模。某个标识如果长期承载多种治理动作，说明分类过粗；某批标识只有文案差异、治理动作相同，说明分类过细。L2 之后的错误治理，需要用生产观测反向校准分类契约。

---

## 工业验证：WarpParse

方法论需要真实系统检验，不能只靠示例代码。WarpParse 是 Orion 体系中面向高吞吐日志解析与 ETL 的核心引擎，在 benchmark 中对比主流方案取得了 1.34x-20.30x 的 EPS 倍数区间。

但 benchmark 证明的是工业强度——高吞吐、多格式、多拓扑、解析与转换并存。错误治理质量，需要从失败路径判断。

WarpParse 中错误治理真正被验证的是：

- 规则语法错误能否定位到规则文件、行列、字段——而不是一句 `unexpected token at line 12` 让规则开发者自己去猜。
- 配置错误能否阻断规则激活，而不是触发系统故障告警——把"人的问题"和"系统的问题"分开。
- 数据质量错误能否聚合统计，而不污染系统错误——`parse.mismatch` 是数据问题，不应触发 `runtime` 级别的告警。
- 运行时错误能否区分可重试、不可重试和需要人工介入——不同的 `category` 对应不同的运维动作。
- 规则开发者视图、运维视图和调试视图是否来自同一个稳定错误标识——同一个 `rule.syntax`，开发者看到修复提示，运维看到配置错误统计，SRE 不会被误 page。

没有结构化错误时，规则语法失败只是一段字符串。引入 Wukong 治理后，同一次失败被表达为：

```
identity : rule.syntax
category : config
detail   : unexpected token in extractor expression
context  : { rule_file, line, column, field, expected_token, actual_token }
policy   : block rule activation, show repair hint, do not page SRE
```

规则开发者拿到位置和修复线索；运行系统拿到稳定标识和策略；运维侧可把配置错误、数据错误、系统错误分开统计和告警。

**吞吐越高，失败路径越需要结构化能力。** 否则处理能力越强，错误扩散和排障成本也被同步放大。

---

## 不适用场景

Wukong 治理模型不是银弹。

1. **小型项目、原型、脚本。** 边界少、生命周期短、错误在局部处理时，`anyhow` + `?` 就够了，没必要引入分层治理。
2. **性能极端敏感的场景。** 结构化错误路径有分配、原因链和上下文采集等成本，静态类型语言中泛型还可能增加编译时间和代码体积。
3. **错误不需要跨层传播。** 所有错误都在一层内处理完毕，收益接近于零。

---

## 总结

错误治理不是异常语法的附属品，也不是日志格式的局部优化。它是工业级系统的信息架构之一。

上篇讲的是方法论——契约通道与诊断通道的分离、五项原则、四级成熟度。这篇讲的是工程落地——五条设计约束把 Wukong 模型在 Rust 里变成了可操作、可测试、可演进的实践。WarpParse 验证了这套方法论在高吞吐工业系统中的可用性：规则开发者、运维、SRE 拿到的是同一套错误的不同视图，而不是同一段字符串的三种解读。

当失败路径也具备稳定分类、完整诊断、集中输出和可演进契约时，系统才真正从"能跑"走向"可长期运行"。

---

*你在项目中做过错误处理的结构化改造吗？踩过哪些坑？欢迎在评论区聊聊。*
