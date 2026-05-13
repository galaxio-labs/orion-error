# Seeking feedback on `orion-error`: structured error governance for Rust systems

I have been working on a Rust crate called `orion-error`, and I would like feedback on whether the design direction is sound for larger Rust systems.

The problem I am trying to solve is not just "how to propagate errors ergonomically." Rust already has strong tools for that. The harder problem is how to design failure paths so that:

- callers get stable contracts they can act on,
- troubleshooters keep enough detail to find root cause,
- and boundary behavior stays consistent across handlers, services, and protocols.

I have been describing that direction internally as the **Wukong Error Governance Model**, and `orion-error` is the Rust crate built around it.

---

## The design problem

In many systems, error handling starts locally and stays local for too long.

One layer returns a business enum. Another passes through a lower-level error. Another turns everything into strings. Then HTTP handlers, RPC endpoints, or CLI entry points each decide their own status codes, visibility rules, and messages.

That can work in small programs. It scales badly in long-lived systems.

The core tension is simple:

- upper layers need stable, finite classifications so they can retry, degrade, alert, or map responses;
- operators and developers need detailed diagnostic information so they can see what actually failed and where.

Most ecosystems give us useful bricks for this. Rust gives especially good ones: `Result<T, E>`, enums, `?`, `match`, and a strong type system. But those bricks still do not define:

- stable error identity,
- semantic boundaries between layers,
- centralized output policy,
- or a distinction between contract-facing information and diagnostic-facing information.

That gap is what `orion-error` is trying to address.

---

## The model in one sentence

The basic idea is:

**make errors converge for governance, while preserving useful information for diagnosis.**

In practice, that means treating an internal error as:

```text
contract channel + diagnostic channel + policy-driven boundary output
```

The contract channel carries stable identity and classification.  
The diagnostic channel carries source chains, context, and detail.  
Boundary output is derived from policy rather than rebuilt ad hoc in each handler.

---

## What this looks like in Rust

The core shape in `orion-error` is roughly:

```text
Result<T, StructError<R>>

R              -> reason / identity / category
StructError<R> -> detail / context / source chain
policy         -> exposure / output decisions
```

The main design choices are:

1. **Reason types are domain-scoped.**  
   Instead of one global mega-enum, layers define semantic-domain reason types such as repository, service, or parser reasons.

2. **Errors are structured at first entry.**  
   When lower-level failures enter the system, the current layer classifies them, adds current-layer detail, and preserves the lower-level error as source.

3. **Crossing semantic domains creates a new boundary.**  
   Upper layers do not simply leak lower-layer classifications. They create a new semantic error while preserving the lower layer in the diagnostic chain.

4. **Boundaries output instead of reinterpret.**  
   Handlers and endpoints do not each invent response policy. They ask a centralized policy to project the internal error into the right external view.

5. **Tests assert identity and policy, not prose.**  
   Error messages change. Stable contracts should not.

This means the crate is not mainly trying to compete on ergonomics with `thiserror` or `anyhow`. It is trying to provide a stronger architectural model for systems where failure paths need long-term structure.

---

## Where I think this is useful

I think this model is most useful when errors need to cross multiple layers or boundaries and remain meaningful over time:

- services with retry / degradation / exposure rules,
- systems with long-lived compatibility contracts,
- platforms where operators and developers need better failure observability,
- internal frameworks that want consistent boundary behavior.

I do **not** think every Rust project needs this. For small tools, scripts, or local apps, `anyhow` plus good context is often enough.

---

## What I would like feedback on

I would especially value feedback from people who have built Rust services or internal platforms around error handling.

Two questions I am interested in:

1. In your systems, do you treat error classification as a stable contract, or is it still mostly an implementation detail?
2. How do you keep boundary behavior consistent across handlers and services without scattering policy decisions everywhere?

If this design direction sounds reasonable, I can also share the longer article version and the crate docs. Right now I am mainly trying to validate whether the model itself is worth pushing further.
