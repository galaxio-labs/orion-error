# orion-error-derive

Derive macros for [`orion-error`](https://crates.io/crates/orion-error).

This crate provides:

- `#[derive(ErrorCode)]`
- `#[derive(ErrorIdentityProvider)]`
- `#[derive(OrionError)]`

Most users should depend on `orion-error` and use its default `derive` feature:

```toml
[dependencies]
orion-error = "0.7.0"
```

Use this crate directly only when you need to pin or inspect the proc-macro crate separately.

## Example

```rust
use derive_more::From;
use orion_error::{OrionError, UvsReason};

#[derive(Debug, Clone, PartialEq, From, OrionError)]
enum AppReason {
    #[orion_error(identity = "biz.invalid_request")]
    InvalidRequest,
    #[orion_error(transparent)]
    Uvs(UvsReason),
}
```
