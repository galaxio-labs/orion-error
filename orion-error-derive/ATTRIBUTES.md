# OrionError derive 属性参考

`#[derive(OrionError)]` 支持以下 `#[orion_error(...)]` 属性。

## 属性总表

| 属性 | 适用位置 | 作用 | 必需的 |
|------|---------|------|--------|
| `identity = "..."` | 变体或结构体 | 设置 `stable_code()` 返回值 | 是（除非 `transparent`） |
| `category = ...` | 变体或结构体 | 覆盖 `error_category()` 分类 | 否（从 `identity` 前缀推断） |
| `transparent` | 变体或结构体 | 委托给内部包装类型 | 二选一：`identity` 或 `transparent` |
| `message = "..."` | 变体或结构体 | 覆盖 Display 文案 | 否（从 `identity` 自动生成） |
| `code = ...` | 变体或结构体 | 设置 `error_code()` 返回值（legacy 数值码） | 否（默认 500） |

## `identity` — 稳定错误码

```rust
#[derive(OrionError)]
enum AppReason {
    #[orion_error(identity = "biz.invalid_input")]
    InvalidInput,
}
```

`identity` 的**前缀**自动决定 `ErrorCategory`：

| 前缀 | ErrorCategory | 示例 |
|------|---------------|------|
| `biz` | `Biz` | `biz.invalid_input` |
| `sys` | `Sys` | `sys.io_error` |
| `conf` | `Conf` | `conf.core_invalid` |
| `logic` | `Logic` | `logic.unreachable` |

## `category` — 显式分类

覆盖前缀推断：

```rust
#[derive(OrionError)]
enum AppReason {
    #[orion_error(identity = "biz.invalid", category = Logic)]
    InvalidInput,
}
```

支持字符串和路径两种写法：`category = "sys"` 或 `category = Sys`。

## `transparent` — 委托

将 `stable_code()`、`error_category()`、`Display` 委托给内部字段：

```rust
#[derive(OrionError)]
enum AppReason {
    #[orion_error(transparent)]
    General(UnifiedReason),           // stable_code() 委托给 UnifiedReason
}
```

常用于包含通用 reason 类型的变体。

## `message` — Display 文案

```rust
#[orion_error(identity = "biz.invalid", message = "invalid input")]
```

如果不指定 `message`，会自动从 `identity` 的最后一个 segment 生成：
`biz.invalid_input` → `"invalid input"`（下划线替换为空格）。

## `code` — 传统数值码（legacy）

```rust
#[orion_error(identity = "biz.invalid", code = 400)]
```

`ErrorCode::error_code()` 返回该值。

> **注意：`identity` / `stable_code()` 才是权威的机器身份。** 数值 `code` 仅用于向后
> 兼容旧的数值码集成（如 HTTP 状态映射、历史监控）。新代码应只依赖 `identity`，
> 不要基于 `code` 引入新的分类语义。
