# Auto-Log Guard 使用指南

`orion-error` 0.9 把操作日志拆成**数据**与 **guard** 两个角色。本页给出推荐用法；
完整参考见[日志说明](./LOGGING.md)。

## 1. 两个角色，各司其职

| 类型 | 角色 | 可 `Clone`？ | 有 Drop 副作用？ |
| --- | --- | --- | --- |
| `OperationContext` | 操作数据（字段、路径、结果） | 是 | **否** |
| `AutoLogGuard` | 持有数据并在 Drop 时写结果日志 | **否** | 是（恰好一次） |

只有 guard 带 `Drop` 副作用，因此把操作当作数据复制或附到错误，永远不会重复或延迟日志。
这就是 [issue #64](https://github.com/galaxio-labs/orion-error/issues/64) 的修复。

## 2. 快速开始

```rust
use orion_error::OperationContext;

// 1. 构建纯数据。
let ctx = OperationContext::doing("sync_user").with_field("user_id", "42");

// 2. 需要生命周期日志时，武装一个 guard。
let guard = ctx.with_auto_log();

do_sync()?;           // 若这里提前返回，没人标记结果……
guard.mark_success(); // ……于是只有走到这里才记成功。
```

guard 在 Drop 时输出 `suc!` / `fail!` / `cancel!`，默认结果是失败：没有显式标记就会记 `fail!`。

## 3. 推荐：`scope()` + `?`

`scope()` 从**失败**状态起步，因此与 `?` 天然兼容：提前返回会跳过 `mark_success()`，失败被正确记录。

```rust
use orion_error::OperationContext;

fn renew(gateway_id: &str) -> Result<(), MyError> {
    let mut guard = OperationContext::doing("renew credential")
        .with_auto_log()
        .with_field("gateway_id", gateway_id);

    let mut scope = guard.scope();   // 默认失败
    let cred = fetch_credential(gateway_id)?; // 提前返回 -> Drop 时 fail!
    store_credential(cred)?;                  // 提前返回 -> Drop 时 fail!
    scope.mark_success();                     // 只有成功才走到这里 -> suc!
    Ok(())
}
```

凡是使用 `?` 的逻辑都应优先用它，而不是 `scoped_success()`。后者一创建就默认成功，
`?` 提前返回会被误记成 `suc!`：

```rust
// 仅当每个失败分支都显式 mark_failure() 时才这么用。
let mut scope = guard.scoped_success();
if !validate() {
    scope.mark_failure();
}
```

## 4. 安全地把操作数据附到错误上

把操作附到错误、供边界渲染是很常见的动作。由于数据是纯的、guard 不 `Clone`，
**借用** guard 即可 —— 只复制数据、**不产生任何日志**：

```rust
use orion_error::OperationContext;

let guard = OperationContext::doing("renew")
    .with_auto_log()
    .with_field("gateway_id", gateway_id);

let value = fetch(gateway_id)
    .map_err(|e| e.with_context(&guard))?; // 复制数据；不打日志
guard.mark_success();
```

刻意**没有**提供按值 `with_context(guard)`：把 guard move 进错误会让日志延迟到错误 Drop
（或重复）。若确实想停止日志、只保留数据，请显式解除武装：

```rust
let data = guard.into_context(); // 不打日志；得到普通的 OperationContext
```

## 5. 从 0.8 迁移

`with_auto_log()` 过去返回 `OperationContext`，现在返回 `AutoLogGuard`。由于 guard 会
`Deref` 到 `OperationContext`，大多数调用点无需改动：

```rust
// 0.8 与 0.9 同样的代码、同样的行为。
let mut ctx = OperationContext::doing("download").with_auto_log();
ctx.record("url", url);
ctx.mark_suc();          // 经 DerefMut 仍然可用
```

真正会受影响的地方：

- 把 `with_auto_log()` 的结果**当作 `OperationContext`** 存储或返回，将不再编译。
  用 `into_context()` 取回纯数据，或让 guard 随操作作用域一起结束。
- 依赖“上下文自身 Drop 时打日志”的代码（如把已武装的 `OperationContext` 附到错误），
  应改为附 `&guard`。

## 6. 常见坑

- **不要**把 `AutoLogGuard` 存进长命的结构体只为“稍后打日志”；把它绑定到它描述的操作作用域。
- **不要**在 `?` 很多的代码里用 `scoped_success()`（见 §3）。
- 附上下文请用 `&guard`，不要按值传。
- guard 标了 `#[must_use]`：误写裸 `OperationContext::doing(..).with_auto_log();` 会有告警
  —— 若本就不想记日志，去掉 `with_auto_log()` 即可。

## 7. 后端说明

同一套 API 适用于任一后端：

- `log`（默认）：日志经 `log` crate 输出。
- `tracing`：启用 `tracing` feature 后走 `tracing`（两者同时开启时优先）。

feature 配置见[日志说明](./LOGGING.md#1-feature)。
