# Design Constraints

## Cross-StructError From Conversion: Orphan Rule Limitation

### Problem

Cross-layer error conversion (`StructError<ParseReason>` → `StructError<OrderReason>`) requires an explicit `.upcast()` call. A blanket `From` to make `?` work automatically is blocked by Rust's orphan rule.

```rust
// Desired but impossible:
fn place_order() -> Result<OrderDraft, StructError<OrderReason>> {
    let draft = parse_order()?;  // expected auto From<ParseError> → OrderError
    Ok(draft)
}

// Actual:
fn place_order() -> Result<OrderDraft, StructError<OrderReason>> {
    let draft = parse_order().upcast()?;  // explicit conversion
    Ok(draft)
}
```

### Root Cause

Rust's orphan rule prohibits implementing `From<Foreign<Local>> for Foreign<Local2>` from a downstream crate:

```rust
impl From<orion_error::StructError<UserLocalReason>>   // Foreign<Local>
    for orion_error::StructError<UserLocalReason2>      // Foreign<Local2>
```

- `From` = std trait (foreign)
- `StructError` = foreign type (from orion-error)
- Even though `LocalReason` and `LocalReason2` are local types

The orphan rule requires at least one local anchor in either the trait or the implementing type. Neither `From` nor `StructError<_>` satisfy this when the impl is written in a downstream crate.

### Attempted Workarounds

| Approach | Result |
|----------|--------|
| Direct `impl From<StructError<A>> for StructError<B>` in downstream | ❌ orphan rule |
| Derive attribute `upcast_from(SubReason)` on target type | ❌ orphan rule |
| Derive attribute `upcast_to(MainReason)` on source type | ❌ orphan rule |
| Make `?` auto-convert across reasons | ❌ can't use `From` |
| newtype `struct AppError(StructError<T>)` | ✅ works, but changes every return type |

### Conclusion

`.upcast()` is the recommended path. `err_conv()` is retained as a deprecated alias for backward compatibility. The newtype wrapper can technically bypass the orphan rule but the cost (wrapping every function return type) far outweighs the benefit of saving one explicit call. Rust's orphan rule is a core guarantee for ecosystem compatibility and is unlikely to change for this use case in the foreseeable future.
