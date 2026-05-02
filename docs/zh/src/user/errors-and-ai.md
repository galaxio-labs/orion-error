# 错误治理与 AI 编程：orion-error 的结构化路径

## 错误处理为什么是瓶颈

工业级代码中，错误处理通常占总开发工作量的 40-60%。这个比例在 Rust 中尤其明显——没有异常机制，没有 `try-catch`，每一处 `?` 都是一个人工决策点。

但问题不在于"需要写更多代码"，而在于**这些决策缺乏结构。** 一个典型的错误处理决策树：

```
这个调用会返回什么错误？→ 我应该拦截还是传播？→ 
如果拦截，新错误属于什么分类？→ 要不要保留原始错误？→
要到边界了吗？→ 暴露给调用方的正确格式是什么？
```

每一层都要重新做这些决策，而且不同开发者的答案往往不一致。代码库越大，错误处理风格越散。

## 错误治理的核心矛盾

任何错误治理方案都要面对一对矛盾：

- **收敛** — 具体的技术错误需要抽象成少的、稳定的上层分类，否则调用方无法治理
- **诊断** — 收敛过程中不能丢失排障所需的信息，否则运维无法排障

这对矛盾在代码中表现为：

```rust
// 倾向收敛，但丢失诊断
Err(AppError::SystemError)  // 技术细节全丢了

// 倾向诊断，但放弃治理
Err(anyhow::format_err!("具体错误: {e}"))  // 调用方只能读字符串
```

orion-error 解决这对矛盾的方式是：**分类收敛到 reason，诊断保留在 source chain + context。** 不是二选一，而是分离到两个维度。

## 这对 AI 编程意味着什么

### 结构即提示

AI 模型（特别是 LLM）通过模式识别生成代码。当错误处理是结构化的，模型更容易推断"正确的事"：

**无结构模式：**

```rust
// AI 难以判断这里该用什么错误类型
fn load_config() -> Result<Config, Box<dyn Error>> {
    let text = std::fs::read_to_string("config.toml")?;
    let cfg = toml::from_str(&text)?;
    Ok(cfg)
}
```

模型需要猜测：`Box<dyn Error>` 里具体是什么？调用方怎么处理？

**有结构模式：**

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

reason 变体是显式的，模型和开发者都可以围绕有限分类做选择。`source_err` + `doing` 是固定模式，比自由拼接错误字符串更容易生成、检查和 review。

### 分类空间收敛

`UnifiedReason` 提供了一套预置分类（validation / system / network / timeout / config...）。通用技术失败先有默认分类，领域失败再补充项目自己的 reason。对模型和开发者来说，很多错误路径不必从零设计分类体系，而是先在预置集合和领域 reason 中做受约束的选择。

当项目有自定义 reason 时，透明变体模式也是固定的：

```rust
#[derive(OrionError)]
enum AppReason {
    #[orion_error(identity = "biz.xxx")]  // 领域特有
    SpecificError,
    #[orion_error(transparent)]
    General(UnifiedReason),  // 通用兜底
}
```

这给代码生成留下了稳定模板：业务变体需要补稳定 `identity`，通用失败通过透明变体复用 `UnifiedReason`。真正的业务语义仍然需要人 review。

### 边界投影消除最后一层决策

AI 生成的代码最难做对的部分是协议边界的错误输出。常见错误：

- 把内部 detail 直接暴露给用户
- HTTP 状态码给错
- 不同协议输出了不一致的结构

orion-error 的 `ExposurePolicy` 把这个决策集中到一处：

```rust
impl ExposurePolicy for MyPolicy {
    fn http_status(&self, identity: &ErrorIdentity) -> u16 {
        match identity.code.as_str() {
            "biz.not_found" => 404,
            "biz.invalid" => 400,
            _ => 500,
        }
    }
    // visibility、retryable、hints 都有默认值，不需要每处写
}
```

这样，边界投影不再散落在每个 handler 里。AI 生成代码时只需要遵循同一个 policy 调用模式；具体状态码、visibility、retryable、hints 仍由团队定义和 review。

### 测试路径可推断

结构化错误的另一个优势：测试中的断言路径也是可推断的。

```rust
// AI 可以可靠地生成这种测试
let err = function_that_fails().unwrap_err();
assert_err_identity(&err, "biz.not_found", ErrorCategory::Biz);
assert_err_operation(&err, "load config");
```

而不是：

```rust
// AI 需要猜测错误信息的确切字符串
let err = function_that_fails().unwrap_err();
assert!(err.to_string().contains("not found"));  // 脆弱
```

## 对 AI 编程的更深影响

### 从"生成代码"到"生成决策"

绝大多数 AI 编程工具目前停留在"生成代码片段"阶段。结构化错误治理把一部分"决策"也结构化了——当错误路径是枚举变体而不是自由字符串时，AI 不再只是在写错误文案，而是在受限集合里选择错误分类。分类选择仍然可能出错，但通常比自由生成错误文本更可控，也更容易通过类型、测试和 review 约束。

### 错误路径覆盖率

AI 生成的代码最容易被忽视的部分就是错误路径。原因是错误路径在训练数据中的占比远低于正常路径。结构化错误体系通过固定的模式（`source_err` + `doing` + `conv_err`）把错误路径写成了可重复模板。模型识别出"这个调用可能失败"之后，有更明确的 API 路径可走，生成结果也更容易被测试覆盖。

### 跨层一致性

多人协作的代码库中，不同开发者对同一类错误的处理方式往往不一致。AI 生成时这个问题更明显——模型在不同的上下文中可能给出不同的处理风格。结构化治理通过集中决策（reason 定义 + policy 实现）把一致性要求前移，让人写的代码和模型生成的代码都围绕同一套约束工作。

## 大语言模型与错误治理的匹配度

| 传统错误处理 | LLM 擅长 | 结构化错误治理 |
|-------------|----------|---------------|
| 自由字符串 | 否 | 枚举变体 |
| 即兴分类决策 | 否 | 固定分类空间 |
| 每处独立写错误信息 | 否 | 模板化模式 |
| 协议输出自行决定 | 否 | Policy 集中决策 |

AI 生成结构化错误代码的可靠性，本质上是把"让模型自由发挥"的问题，尽量转化为"让模型在有限选项中选择，并接受类型、测试和 review 约束"的问题。后者更适合工程化治理。

## 局限

1. **前期建模成本仍然存在。** Reason 分类和 Policy 定义需要人工完成。
2. **小型项目不适合。** 三五层的函数链用 `thiserror` 或 `anyhow` 更轻量。
3. **AI 对业务语义的理解有限。** 选择正确的业务分类（"这是 validation_error 还是 business_error？"）仍然需要人判断。

## 总结

orion-error 的结构化错误模型与 AI 编程的匹配不是偶然的。两者都受益于同一个原则：**把隐式的决策变成显式的结构。** 隐式决策依赖上下文理解（人和模型都容易犯错），显式结构依赖模式识别（结构化数据 + 固定分类空间对 LLM 是理想场景）。

这可能是 Rust 错误治理的一个方向：不是设计一个更聪明的错误类型，而是设计一个让错误处理决策变得更加可预测、可枚举的系统——对人如此，对 AI 也是如此。
