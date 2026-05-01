# 兼容与迁移

## API 重命名历史

| 旧名称 | 新名称 | 简介 |
|--------|--------|------|
| `into_as(reason, detail)` | `source_err(reason, detail)` | 原始错误包进来成为 source |
| `wrap_as(reason, detail)` | `source_err(reason, detail)` | 同上，统一入口 |
| `upcast()` | `conv_err()` | 跨层 reason 转换 |
| `err_conv()` | `conv_err()` | 同上 |

旧名称不再可用。如果遇到编译错误，直接替换为新名称即可，参数不变。

## 0.7 → 0.8 迁移

0.8 删除了以下 0.7 的兼容路径：

- `compat_prelude` / `compat_traits` 模块
- `ErrorOwe` 系列 trait（`owe()` / `owe_source()` 等）
- `ErrorWith` 上的 `want()` / `attach_context()` / `with()`
- `OperationContext::with_want()`
