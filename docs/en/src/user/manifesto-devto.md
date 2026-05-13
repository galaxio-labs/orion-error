# Error Handling Stops Scaling When It Is Treated as a Local Coding Habit

Many systems become hard to maintain later not because the business is inherently complex, but because error handling was never designed as part of the system.

At prototype stage, error handling is often just "bubble it up if something fails." But once a system enters long-term evolution, failure paths stop being local control flow. They carry retry decisions, degradation behavior, alert routing, user responses, troubleshooting context, and compatibility expectations. If errors are still just ad hoc strings, local enums, or incidental wrappers, the system becomes brittle.

From a system builder's perspective, the central question is not "how do we throw or return errors?" It is this:

**How do we make errors converge for governance while preserving useful information for diagnosis?**

This is the problem I have been trying to solve with what I call the **Wukong Error Governance Model**, and in Rust, with the `orion-error` crate.

---

## The Core Tension: Governance vs. Diagnostics

Any serious error model has to satisfy two different consumers:

- Callers need stable, finite classifications so they can retry, degrade, alert, or map boundary responses.
- Troubleshooters need detailed, reliable information so they can find root cause.

Both are legitimate. They also pull in opposite directions.

If you expose too much low-level detail, upper layers become coupled to the exact failure shapes of databases, network libraries, filesystems, or third-party SDKs. Refactoring the lower layer breaks contracts.

If you only keep top-level business classifications, debugging loses the path: what actually failed, where it failed, what each layer added, and why the final response was produced.

That is why the real problem is not "should we wrap errors?" but:

**How can errors converge for governance without collapsing for diagnostics?**

---

## Languages Give You Bricks, Not the Building

Every major language ecosystem has explored this problem:

- Java has exceptions and error codes.
- Go has explicit `error` returns, wrapping, and `errors.Is` / `errors.As`.
- Rust has `Result<T, E>`, `?`, enums, and a powerful type system.

These are all useful building blocks. None of them automatically give you an error architecture.

They help you express failure. They do not, by themselves, define:

- which failures should share one stable identity,
- which failures should become one governance class,
- which details should stay internal,
- which outputs should be exposed at the boundary,
- and which decisions should be centralized instead of repeated in every handler.

That is the gap between "using error features" and "having error governance."

---

## The Wukong Model

The core idea is simple:

**Use stable contracts to make failures governable, reliable diagnostics to preserve root cause, and adaptive output to generate the right view for each receiver.**

Internally, that means treating an error as two channels plus a projection step:

```text
internal error model = contract channel + diagnostic channel
adaptive output      = boundary-specific view generated from the internal model
```

The contract channel carries things like:

- stable error identity,
- stable classification,
- governance attributes such as retryability or exposure level.

The diagnostic channel carries things like:

- source / cause chain,
- operation context,
- structured detail,
- lower-level failures.

Then boundary outputs such as HTTP responses, CLI output, logs, metrics, or RPC errors are produced from policy, not handcrafted independently in every endpoint.

The key is not making errors more complicated. The key is separating information that should be stable from information that should remain detailed and dynamic.

---

## Why Rust Is a Good Fit

Rust is a strong fit for this model because it already makes failure paths explicit.

- `Result<T, E>` keeps error flow in the type system.
- enums are a natural fit for finite classification spaces.
- `match` gives exhaustive handling.
- generic carriers make it possible to reuse one structured runtime model across domains.

But Rust still does not solve governance automatically.

`thiserror`, `anyhow`, and `eyre` are useful. They help with type generation, propagation ergonomics, and diagnostics. They do not define stable error identity, semantic boundary rules, or centralized output policy for you.

Without those extra constraints, many Rust systems still drift into a familiar pattern:

- some places return business enums,
- some places pass lower-level errors through unchanged,
- some places flatten everything into strings,
- every handler decides status codes and messages a little differently.

That is error handling, but not yet error governance.

---

## Five Practical Rules for Rust

## 1. Define reasons by semantic domain, not as one global mega-enum

Do not make one `AppError` own every failure in the system. Use domain-level reason types instead: `RepositoryReason`, `OrderReason`, `ParserReason`, and so on.

```rust
#[derive(Debug, Clone, OrionError)]
enum OrderReason {
    #[orion_error(identity = "order.submit_dependency_unavailable")]
    SubmitDependencyUnavailable,

    #[orion_error(identity = "order.invalid_state")]
    InvalidState,
}
```

What stays stable is the error identity, not the message wording and not necessarily the enum name itself.

## 2. Structure errors at first entry

When I/O, network, database, or parse failures enter your own system, that is the moment to do three things at once:

- choose the current-layer classification,
- explain the current-layer failure,
- preserve the lower-level error as source.

If you flatten the lower layer into a string first and reinterpret it later, both governance and diagnostics degrade.

## 3. Create a new boundary when crossing semantic domains, but preserve the lower layer

Inside one semantic domain, classification can converge. Across domains, a new semantic boundary should usually be created.

For example, a repository connection failure should not leak directly as a repository-level failure to an order service boundary. At the upper layer it may become `order.submit_dependency_unavailable`, while the repository failure remains preserved in the diagnostic chain.

## 4. Boundaries should output, not reinterpret

HTTP handlers, RPC endpoints, and CLI entry points should not each decide status codes, messages, and visibility on their own. The boundary should hand the internal error to centralized policy and receive the appropriate output view.

That is how one stable error identity leads to one consistent external behavior.

## 5. Test error identities, not message prose

Messages change. Wording improves. Text gets redacted or localized. Stable contracts should not depend on that.

```rust
assert_eq!(
    err.identity_snapshot().code,
    "order.submit_dependency_unavailable"
);
```

If tests lock onto prose, every wording change becomes a compatibility hazard. If tests lock onto identity and policy outcomes, the contract becomes enforceable.

---

## What `orion-error` Is Trying to Do

`orion-error` is not mainly about saving a few lines of Rust error boilerplate.

It is an attempt to make error governance in Rust operational:

```text
Result<T, StructError<R>>

R              -> contract channel: reason / identity / category
StructError<R> -> diagnostic channel: detail / context / source chain
policy         -> boundary output decisions
```

In that structure:

- `R` expresses stable classification and identity,
- `StructError<R>` carries diagnostic information,
- exposure, report, and snapshot APIs project the internal model into boundary-specific views.

The point is not one clever API. The point is to give industrial Rust systems a consistent model for error propagation, compatibility, exposure, and diagnosis.

---

## Closing Thought

Rust already gives us strong primitives for error handling. But industrial systems need more than `Result`, `?`, and a few convenient derive macros.

They need a durable governance structure for failure paths: one that gives callers stable contracts, gives troubleshooters useful information, keeps boundary behavior consistent, and helps systems resist long-term decay.

That is why I describe `orion-error` as the Rust implementation of the Wukong Error Governance Model.
