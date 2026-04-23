# `order_case` 示例说明

这个示例现在保留两件事：

1. 多层错误体系与跨层转换
2. 当前已落地的消费协议

也就是说，它不是一个“最短 hello world”，而是一个“复杂度受控，但仍能看出分层价值”的示例。

## 这个示例展示什么

示例里保留了四层错误语义：

- `ParseReason`
- `UserReason`
- `StoreReason`
- `OrderReason`

对应一条短业务链：

1. `parse order`
2. `load user`
3. `check balance`
4. `save order`

其中：

- 解析层返回 `StructError<ParseReason>`
- 用户层返回 `StructError<UserReason>`
- 存储层返回 `StructError<StoreReason>`
- 服务层最终收敛成 `StructError<OrderReason>`

## 这个示例的核心价值

### 1. 结构化错误第一次进入体系

存储层对 `std::io::Error` 使用：

```rust
persist_order(...)
    .into_as(UvsReason::system_error(), "write order record failed")
```

这展示的是“普通错误第一次进入结构化体系”的主路径。

### 2. 跨层 reason 收敛

服务层把子层错误往上收敛时，使用：

```rust
.doing("parse order")
.with_context(&ctx)
.err_conv()?
```

这展示的是：

- 下层保留自己的局部 reason
- 上层通过 `From<R1> for R2` 做 reason 收敛
- context/path 继续向上保留

### 3. 最终只暴露上层领域错误

对调用者来说，最终只接触：

```rust
Result<T, StructError<OrderReason>>
```

这比直接把所有底层 reason 暴露给上层更符合分层边界。

### 4. 消费协议

每个失败场景最后都会直接展示一组“摘要优先”的结果：

- `identity_snapshot()`
- `policy_snapshot(...).decision`
- `http_response(...)`
- `render_user_debug(...)`

这个示例最终选择 `render_user_debug(...)` 作为“给人读”的默认展示出口，而不是直接把
`Debug` 结构体或完整 verbose report 打出来。

这说明多层错误体系和消费协议不是二选一，而是可以叠加的：

- 运行时传播时保留结构化层次
- 出口消费时统一走稳定协议

## 推荐关注点

### 入口上下文

入口上下文统一写在服务层：

```rust
let mut ctx = OperationContext::doing("place_order");
ctx.record_field("user_id", user_id.to_string());
ctx.record_field("order.raw", raw_order);
ctx.record_meta("component.name", "order_service");
```

它展示的是：

- `record_field(...)` 适合展示型 field
- `record_meta(...)` 适合结构化 metadata

最终摘要输出大致会收敛成：

```text
code          : biz.order_invalid (Biz)
detail        : order text must not be empty
http          : 400 Public retryable=false
path          : place_order / parse order
context       : user_id="42", order.raw=""
component     : order_service
```

### `From` 映射的边界意义

示例里保留了：

- `From<ParseReason> for OrderReason`
- `From<UserReason> for OrderReason`
- `From<StoreReason> for OrderReason`

这不是为了写样板代码，而是为了明确：

- 子层 reason 可以更细
- 上层领域 reason 可以更稳

### `StorageFull` 的例子

这个例子把存储层边界收得更清楚：

- `write_impl(...) -> std::io::Error`
- `persist_order(...) -> StoreError`
- `save_order(...) / place_order(...) -> OrderError`

它表达的是：

- 原始 I/O 只存在于更底层 helper
- 存储层公开边界直接返回 `StoreError`
- 服务层再通过 `err_conv()` 把 `StoreError` 收敛成 `OrderError`

## 为什么不再用更复杂的老版本写法

旧版本示例的问题是层次很多，但阅读负担太大，容易把注意力都耗在样板代码上。

当前版本的取舍是：

- 保留多层 reason 和转换
- 缩短每层逻辑
- 直接把重点落在“分层传播 + 协议消费”上

## 如果你只想运行

```bash
cargo run --example order_case
```

如果你想看协议本身的正式说明，直接看：

- `docs/protocol-contract.md`
