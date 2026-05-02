# Comparison with thiserror

`orion-error` and `thiserror` are not mutually exclusive, but their positioning differs.

## Positioning

**thiserror**: Define standard Rust error types, serving the `std::error::Error` ecosystem.

**orion-error**: Define runtime structured error carriers, managing context, source frames, snapshots, and protocol projections.

## Capability Comparison

| Capability | thiserror | orion-error |
|-----------|-----------|-------------|
| Define standard error types | Strong | Not primary goal |
| Domain reason derive | Needs extra identity | `OrionError` is recommended |
| Runtime structured context | No | Yes |
| Source frame tracking | No | Yes |
| stable code / category | No | Yes |
| snapshot / report / projection | No | Yes |

## When to Use thiserror

- Exposing standard `std::error::Error` types
- Library APIs requiring standard error types
