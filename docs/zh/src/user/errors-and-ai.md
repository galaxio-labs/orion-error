# 结构化错误如何改善 AI 代码生成

## 错误处理的成本不在代码量

行业数据显示，缺陷定位和修复占据 40-60% 的工程成本（Hamill & Goseva-Popstojanova 引用 Cambridge 报告估计开发者约 50% 时间用于 finding and fixing bugs；Capers Jones 总结该比例常超过 60%）。但另一组数据更有意思：Cabral 和 Marques 对 32 个 Java/.NET 应用的 field study 显示，异常处理代码只占源码的 3-7%。

成本不在"写错误处理代码"，而在**失败发生后，缺乏结构化信息去定位、分类和决策**。Rust 中没有异常，没有 `try-catch`，每一处 `?` 都是一个传播决策点。如果这些决策没有结构，错误路径随代码规模一起失控。

```text
这个调用会返回什么错误？→ 我应该拦截还是传播？→
如果拦截，新错误属于什么分类？→ 要不要保留原始错误？→
要到边界了吗？→ 暴露给调用方的正确格式是什么？
```

每一层都重新做这些决策，不同开发者的答案往往不一致。

## 为什么结构化错误更适配 LLM

orion-error 解决错误治理核心矛盾（收敛 vs. 诊断）的方式是把分类信息收敛到 reason，诊断信息保留在 source chain + context。详见《双通道：工业级系统的错误治理模型》。

对 AI 编程而言，这个分离有四个直接收益。

### 结构即提示

LLM 通过模式识别生成代码。当错误处理是结构化的，模型更容易推断正确输出。

**无结构：**

```rust
// AI 难以判断这里该用什么错误类型
fn load_config() -> Result<Config, Box<dyn Error>> {
    let text = std::fs::read_to_string("config.toml")?;
    let cfg = toml::from_str(&text)?;
    Ok(cfg)
}
```

模型需要猜测：`Box<dyn Error>` 里具体是什么？调用方怎么处理？

**有结构：**

```rust
fn load_config() -> Result<Config, StructError<ConfigReason>> {
    let text = std::fs::read_to_string("config.toml")
        .source_err(ConfigReason::ReadFailed, "read config file")
        .doing("load config")?;
    let cfg = toml::from_str(&text)
        .source_err(ConfigReason::ParseFailed, "parse config")
        .doing("parse config")?;
    Ok(cfg)
}
```

reason 变体是显式的，模型和开发者都围绕有限分类做选择。`source_err` + `doing` 是固定模式，比自由拼接错误字符串更容易生成、检查和 review。

### 分类空间收敛

`UnifiedReason` 提供了预置分类（validation / system / network / timeout / config...）。通用技术失败先有默认分类，领域失败再补充项目自己的 reason。模型和开发者不必从零设计分类体系，而是在预置集合和领域 reason 中做受约束的选择。

```rust
#[derive(OrionError)]
enum AppReason {
    #[orion_error(identity = "biz.xxx")]  // 领域特有
    SpecificError,
    #[orion_error(transparent)]
    General(UnifiedReason),  // 通用兜底
}
```

这给代码生成留下了稳定模板：业务变体补 identity，通用失败通过透明变体复用 `UnifiedReason`。真正的业务语义仍需人 review。

### 边界投影消除最后一层决策

AI 代码最难做对的部分是协议边界的错误输出：把内部 detail 暴露给用户、HTTP 状态码给错、不同协议输出不一致。`ExposurePolicy` 把这个决策集中到一处：

```rust
impl ExposurePolicy for MyPolicy {
    fn http_status(&self, identity: &ErrorIdentity) -> u16 {
        match identity.code.as_str() {
            "biz.not_found" => 404,
            "biz.invalid" => 400,
            _ => 500,
        }
    }
}
```

AI 生成代码时只需遵循同一个 policy 调用模式；状态码、visibility、retryable、hints 仍由团队定义和 review。

### 测试路径可推断

结构化错误的测试断言也是可推断的：

```rust
// AI 可以可靠生成这种测试
let err = function_that_fails().unwrap_err();
assert_err_identity(&err, "biz.not_found", ErrorCategory::Biz);
assert_err_operation(&err, "load config");
```

而不是：

```rust
// AI 需要猜测错误消息的确切字符串
let err = function_that_fails().unwrap_err();
assert!(err.to_string().contains("not found"));  // 脆弱
```

## LLM 在两种范式下的表现

| 任务 | 传统方式（自由字符串/即兴决策） | 结构化方式（枚举/分类空间/Policy） |
|------|-------------------------------|----------------------------------|
| 选择错误类型 | 需猜测，易出错 | 从有限枚举中选择 |
| 写错误文案 | 每处不同，不可控 | 模板化模式 |
| 边界输出 | 各 handler 自行决定 | Policy 集中决策 |
| 测试错误路径 | 依赖字符串匹配，脆弱 | 断言 identity，稳定 |

核心差异：结构化错误把"让模型自由发挥"转为"让模型在有限选项中做选择，并受类型、测试和 review 约束"。

## 更深的影响

**从生成代码到生成决策。** 当错误路径是枚举变体而非自由字符串时，AI 不再只是写错误文案，而是在受限集合里选择分类。分类选择仍可能出错，但比自由生成更可控，更容易通过类型、测试和 review 约束。

**错误路径覆盖率。** AI 代码最易被忽视的就是错误路径——训练数据中错误路径占比远低于正常路径。结构化错误通过固定模式（`source_err` + `doing` + `conv_err`）把错误路径写成可重复模板。模型识别出"这个调用可能失败"后，有明确的 API 路径可走。

**跨层一致性。** 多人协作的代码库中，不同开发者对同一错误的处理方式往往不一致——AI 在不同上下文中也可能给出不同风格。结构化治理通过集中决策（reason 定义 + policy 实现）把一致性要求前移，人写的代码和模型生成的代码围绕同一套约束工作。

## 局限

1. **前期建模仍需人工。** Reason 分类和 Policy 定义是契约，不能由 AI 生成——它们需要人对业务和架构的判断。
2. **存量迁移不在 AI 当前能力范围内。** 把 L0 的字符串错误重构成结构化体系涉及类型变更和语义判断，AI 只能辅助不能主导。
3. **领域语义选择仍会出错。** 模型可能在"这个失败属于 config 还是 system"上判断失误，需要 review 和测试约束来兜底。

## 总结

orion-error 的结构化错误模型与 AI 编程的匹配不是偶然的。两者都受益于同一个原则：**把隐式决策变成显式结构。** 隐式决策依赖上下文理解（人和模型都容易犯错），显式结构依赖模式识别（结构化数据 + 固定分类空间对 LLM 是理想场景）。

这可能是 Rust 错误治理的一个方向：不是设计更聪明的错误类型，而是设计让错误处理决策变得更可预测、可枚举的系统——对人如此，对 AI 也是如此。
