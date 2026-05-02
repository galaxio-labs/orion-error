# Error Governance and AI Programming: orion-error's Structured Path

## Why Error Handling Becomes a Bottleneck

In industrial software, fault localization, bug fixing, and avoidable rework consume a large amount of engineering effort. Studies and industry reports commonly place finding/fixing bugs and avoidable rework in the 40-60% range; this does not mean that "error-handling code itself is 40-60% of the code or effort." Error governance is concerned with what happens when failure occurs: whether classification is stable, context is preserved, boundary output is consistent, and the diagnostic path is complete.

Useful data points:

- Hamill and Goseva-Popstojanova, in a NASA fault-fix effort study, cite a Cambridge University report that developers spend about 50% of their time finding and fixing bugs; the same passage cites Boehm/Basili's 40-50% effort on avoidable rework.
- Capers Jones, in an ASQ / Software Quality Professional article, summarizes that finding and fixing bugs often exceeds 60% of total software effort.
- Cabral and Marques, in a field study of 32 Java/.NET applications, show that exception-handling code itself is much smaller: about 5% on average for Java, about 3% on average for .NET, and up to about 7%.

So this article is not about "writing more error-handling code." It is about using structured error mechanisms to reduce the cost of fault localization, boundary governance, and cross-layer diagnostics. Rust has no exception mechanism and no `try-catch`; every `?` is a propagation decision. Without structure, those decisions become harder to govern as the codebase grows.

The problem is not simply "more code." The problem is that these decisions lack structure. A typical error-handling decision tree looks like this:

```text
What error can this call return? -> Should I intercept it or propagate it?
If I intercept it, what category should the new error use?
Should I preserve the original error?
Am I at a boundary?
What is the correct format for the caller?
```

Every layer repeats these decisions, and different developers often answer them differently. The larger the codebase, the more fragmented error handling becomes.

## The Core Tension

Every error-governance approach has to handle one tension:

- **Convergence**: concrete technical errors need to be abstracted into a small, stable set of upper-layer categories, otherwise callers cannot govern retry, fallback, alerting, or user-facing output.
- **Diagnostics**: the convergence process must not lose the information needed for troubleshooting.

In code, the tension often looks like this:

```rust
// Converges, but loses diagnostics.
Err(AppError::SystemError)

// Preserves some detail, but gives up governance.
Err(anyhow::format_err!("concrete error: {e}"))
```

orion-error handles this by separating the two dimensions: **classification converges into a reason, while diagnostics stay in the source chain and context**.

## What This Means for AI Programming

### Structure as Prompt

AI models, especially LLMs, generate code by following patterns. When error handling is structured, the model has a clearer pattern to follow.

**Unstructured pattern:**

```rust
// The model has to guess which error type matters here.
fn load_config() -> Result<Config, Box<dyn Error>> {
    let text = std::fs::read_to_string("config.toml")?;
    let cfg = toml::from_str(&text)?;
    Ok(cfg)
}
```

The model has to infer what is inside `Box<dyn Error>` and how callers should handle it.

**Structured pattern:**

```rust,ignore
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

The reason variant is explicit, so both the model and the developer choose from a constrained classification space. The `source_err` + `doing` pattern is easier to generate, inspect, and review than free-form string wrapping.

### Constrained Classification Space

`UnifiedReason` provides built-in categories such as validation, system, network, timeout, and config. Common technical failures get default categories first; domain-specific failures can then add project-specific reasons. This means many error paths do not need a new classification scheme from scratch.

For project-specific reasons, the transparent-variant pattern is stable:

```rust
#[derive(Debug, Clone, PartialEq, OrionError)]
enum AppReason {
    #[orion_error(identity = "biz.xxx")]
    SpecificError,
    #[orion_error(transparent)]
    General(UnifiedReason),
}
```

This gives code generation a stable template: business variants need a stable `identity`, and common failures reuse `UnifiedReason` through a transparent variant. The actual business semantics still need human review.

### Boundary Projection Becomes Centralized

Protocol-boundary output is one of the easiest places for generated code to get things wrong:

- exposing internal detail directly to users
- choosing the wrong HTTP status
- producing inconsistent shapes across protocols

orion-error's `ExposurePolicy` centralizes that decision:

```rust
impl ExposurePolicy for MyPolicy {
    fn http_status(&self, identity: &ErrorIdentity) -> u16 {
        match identity.code.as_str() {
            "biz.not_found" => 404,
            "biz.invalid" => 400,
            _ => 500,
        }
    }
    // visibility, retryable, and hints have defaults.
}
```

Boundary projection no longer has to be hand-written in every handler. Generated code can follow one policy invocation pattern; exact status codes, visibility, retryability, and hints remain team-defined and reviewable.

### Test Paths Become Easier to Infer

Structured errors also make test assertions more direct:

```rust
// Easier for generated code to produce, and easier for reviewers to check.
let err = function_that_fails().unwrap_err();
assert_err_identity(&err, "biz.not_found", ErrorCategory::Biz);
assert_err_operation(&err, "load config");
```

Instead of:

```rust
// Requires guessing exact display text.
let err = function_that_fails().unwrap_err();
assert!(err.to_string().contains("not found"));
```

## Deeper Impact on AI Programming

### From Code Generation to Decision Structuring

Most AI programming tools operate at the level of generating code snippets. Structured error governance turns part of the decision into data: when an error path is represented by an enum variant rather than a free-form string, the model is no longer only writing prose; it is choosing from a constrained set. That choice can still be wrong, but it is easier to constrain with types, tests, and review.

### Error-Path Coverage

Generated code often underinvests in error paths. Error paths appear less frequently than happy paths in training data and examples. A structured system turns error paths into repeatable patterns such as `source_err` + `doing` + `conv_err`. Once the model recognizes that a call can fail, there is a clearer API path to follow, and the result is easier to cover with tests.

### Cross-Layer Consistency

In multi-person codebases, different developers often handle the same kind of failure differently. Generated code can make this worse because the model may produce different styles in different contexts. Structured governance moves consistency requirements into shared definitions: reason enums and exposure policies. Both human-written and generated code then work under the same constraints.

## LLMs and Error Governance

| Traditional error handling | Constraint quality | Structured error governance |
|----------------------------|--------------------|-----------------------------|
| Free-form strings | Hard to constrain | Enum variants |
| Ad-hoc classification | Hard to review | Fixed classification space |
| Local error-message decisions | Inconsistent | Repeatable API patterns |
| Boundary output decided per handler | Fragmented | Centralized policy |

The reliability benefit is not that the model becomes "smart enough" to understand every failure. The benefit is that the task is shifted away from free-form generation and toward choosing from constrained options that can be checked by types, tests, and review.

## Limits

1. **Up-front modeling cost remains.** Reason categories and exposure policies still require human design.
2. **Small projects may not need this.** A short script or prototype is often better served by `thiserror` or `anyhow`.
3. **Business semantics are still hard.** Choosing between `validation_error` and `business_error` still requires domain judgment.

## Summary

orion-error's structured error model fits AI-assisted programming because both benefit from the same principle: **turn implicit decisions into explicit structure**. Implicit decisions rely on context interpretation, where both humans and models make mistakes. Explicit structure gives the system enums, source chains, context, policies, and tests.

This is one possible direction for Rust error governance: not a smarter error type by itself, but a system that makes error-handling decisions more predictable, enumerable, and reviewable for both humans and AI-assisted tools.
