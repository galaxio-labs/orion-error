# Compatibility & Migration

## API Renames

| Old Name | New Name | Description |
|----------|----------|-------------|
| `into_as(reason, detail)` | `source_err(reason, detail)` | Unified error entry point |
| `wrap_as(reason, detail)` | `source_err(reason, detail)` | Same, unified |
| `upcast()` | `conv_err()` | Cross-layer reason conversion |
| `err_conv()` | `conv_err()` | Same |

Old names are no longer available. If you see a compilation error, replace with the new name — parameters are unchanged.

## 0.7 → 0.8 Migration

0.8 removed the following 0.7 compatibility paths:

- `compat_prelude` / `compat_traits` modules
- `ErrorOwe` family of traits (`owe()` / `owe_source()` etc.)
- `ErrorWith` methods `want()` / `attach_context()` / `with()`
- `OperationContext::with_want()`
