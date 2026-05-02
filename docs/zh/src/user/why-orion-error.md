# 为什么需要 orion-error

`orion-error` 不是为了把错误打印得更漂亮，而是为了让 Rust 服务中的错误成为可治理、可追踪、可暴露、可演进的结构化契约。

普通错误处理回答的是“这段代码如何返回失败”。大型服务还需要回答：

- 出错时如何携带关键环境信息？
- 技术细节错误如何抽象成上层合理语义，同时不丢诊断信息？
- 排错时如何看到跨层错误传递链？
- 如何在失败边界输出有效日志，而不是到处写日志？
- 同一个错误如何给用户、运维、开发者和协议客户端呈现不同视图？

`orion-error` 的核心价值是：让错误在跨层传播时保留结构，而不是退化成字符串。

---

## 1. 诊断

### 1.1 错误本身经常缺少关键环境信息

很多底层错误只告诉你“发生了什么技术失败”，但不会告诉你“失败发生在哪个业务环境里”。

例如：

```rust
let content = std::fs::read_to_string(path)?;
```

如果读取失败，底层 `std::io::Error` 可能只告诉你：

```text
No such file or directory
```

但排障真正需要的问题通常是：

- 读取的是哪个路径？
- 当前正在执行什么操作？
- 这个路径属于哪个租户、订单、请求或组件？
- 它是配置文件、订单记录、缓存文件，还是临时文件？
- 这个失败应该被归类为配置错误、系统错误，还是业务校验错误？

### 不够好的做法：只在日志里补字符串

```rust
match std::fs::read_to_string(path) {
    Ok(content) => Ok(content),
    Err(err) => {
        log::error!("read config failed, path={path}, error={err}");
        Err(err)
    }
}
```

这种方式的问题是：

- 日志和错误对象是两套信息。
- 上层拿到的错误仍然没有结构化上下文。
- 如果边界还要输出 HTTP/RPC/CLI 错误，仍然需要重新组织字段。
- 多层代码很容易重复打印同一个失败。

### 推荐做法：错误产生时就携带结构化上下文

```rust
use orion_error::prelude::*;
use orion_error::runtime::OperationContext;

let ctx = OperationContext::doing("load config")
    .with_field("path", path.display().to_string())
    .with_meta("component.name", "config_loader");

let content = std::fs::read_to_string(path)
    .source_err(AppReason::system_error(), "read config failed")?
    .with_context(&ctx);
```

这里的语义是：

- `source_err(...)` 把底层 `std::io::Error` 接入结构化错误系统。
- `AppReason::system_error()` 是上层可以理解的稳定错误语义。
- `"read config failed"` 是当前层对失败的解释。
- `OperationContext` 携带 `path` 和 `component.name` 等关键环境信息。

### 1.2 技术细节错误需要抽象，但不能丢诊断信息

跨层传播技术错误时，常见两个坏选择：

1. 在下层丢弃具体错误。
2. 把具体错误直接暴露给上层，让 service / API 层依赖数据库、HTTP client、文件系统或解析器实现。

这两种方案都不合适。

正确做法是：**在层边界把下层错误转换/抽象成当前层合理的错误语义，同时保留排障所需的 source、detail 和 context，并切断上层对具体技术实现的依赖。**

#### 不够好的做法：让 service 层依赖 repository 的技术错误

```rust
async fn submit_order(order: Order) -> Result<(), sqlx::Error> {
    repository::insert_order(order).await?;
    Ok(())
}
```

这会让 service/API 层知道底层使用了 `sqlx`。将来 repository 从 PostgreSQL 改成对象存储、消息队列或远程服务时，上层错误契约也会被迫变化。

#### 不够好的做法：丢掉底层错误

```rust
async fn submit_order(order: Order) -> Result<(), StoreError> {
    if repository::insert_order(order).await.is_err() {
        return Err(StoreReason::Unavailable.to_err());
    }

    Ok(())
}
```

这看起来切断了技术依赖，但也丢掉了根因。排障时只知道“存储不可用”，不知道是连接超时、唯一键冲突、序列化失败，还是磁盘满。

#### 推荐做法：抽象到当前层 reason，同时保留下层 source

```rust
use orion_error::prelude::*;

async fn write_order(order: Order) -> Result<(), StructError<StoreReason>> {
    repository::insert_order(&order)
        .await
        .source_err(StoreReason::Unavailable, "insert order failed")?
        .with_field("order_id", order.id.to_string())
        .with_meta("component.name", "order_store");

    Ok(())
}
```

这里：

- 上层看到的是 `StructError<StoreReason>`，不是 `sqlx::Error`。
- `StoreReason::Unavailable` 是存储层合理的稳定语义。
- 原始数据库错误仍然作为 source 保留在内部诊断链里。
- `order_id`、`component.name` 等字段用于排障和日志投影。

如果上层只需要把 `StoreReason` 收敛成 `AppReason`，但不想创建新的语义边界，可以使用 `conv_err()`：

```rust
async fn submit_order(order: Order) -> Result<(), StructError<AppReason>> {
    write_order(order).await.conv_err()?;
    Ok(())
}
```

如果上层确实要建立新的业务语义边界，例如“提交订单失败”，则使用 `source_err(...)` 保留下层结构化错误作为 source：

```rust
async fn submit_order(order: Order) -> Result<(), StructError<AppReason>> {
    write_order(order)
        .await
        .source_err(AppReason::system_error(), "submit order failed")?;

    Ok(())
}
```

这两个方法表达的语义不同：

- `conv_err()`：只做 reason 映射，不新增边界。
- `source_err(reason, detail)`：建立新的语义边界，并保留下层错误链。

### 1.3 排错需要错误传递链，而不是孤立错误

真实故障很少只发生在一个函数里。一个最终错误通常经历多层传递：

```text
HTTP handler
  -> service
    -> repository
      -> database / filesystem / remote API
```

如果每一层都只是替换成新的字符串，排错时只能看到最终错误：

```text
submit order failed
```

但真正有价值的是它如何一路变成这个错误：

```text
submit order failed
  caused by: insert order failed
  caused by: database request failed
  caused by: connection timed out
```

错误传递链能回答这些问题：

- 最初的技术失败是什么？
- 失败经过了哪些业务层？
- 哪一层建立了新的语义边界？
- 每一层添加了哪些上下文？
- 最终对外暴露的错误，与内部真实原因是什么关系？

#### 推荐做法：每个边界都保留 source chain

```rust
async fn adapter_call(req: Request) -> Result<Response, StructError<AdapterReason>> {
    client.send(req)
        .await
        .source_err(AdapterReason::RemoteUnavailable, "remote call failed")
}

async fn load_quote(id: QuoteId) -> Result<Quote, StructError<ServiceReason>> {
    adapter_call(Request::quote(id))
        .await
        .source_err(ServiceReason::QuoteLoadFailed, "load quote failed")?
        .with_field("quote_id", id.to_string());

    todo!("map response")
}
```

这不是简单的“包装一层字符串”。它保留了：

- service 层语义：`QuoteLoadFailed`
- adapter 层语义：`RemoteUnavailable`
- 底层 source：HTTP client / IO / timeout 等具体错误
- 结构化字段：`quote_id`

排错时可以从最终错误沿 source chain 追溯到根因；协议边界则可以只暴露安全、稳定的上层身份。

---

## 2. 运维

### 有效日志不是大量日志，而是边界统一记录

很多系统排错困难，不是因为日志太少，而是因为日志方式不对：

- 每一层都 `error!` 一次，产生重复日志。
- 日志只有字符串，没有稳定字段。
- 为了补上下文，到处写 `path={path}`、`tenant={tenant}`、`order_id={order_id}`。
- 业务代码被日志拼接污染。
- 日志和错误对象携带的信息不一致。

更合理的方式是：**错误在传播过程中携带结构化 context、source chain、reason 和 stable identity；日志只在 handler、worker、任务边界统一记录一次。**

#### 不够好的做法：每一层都打印一次

```rust
log::error!("repository insert failed: {err}");
log::error!("service submit failed: {err}");
log::error!("http request failed: {err}");
```

这种方式会带来大量重复日志。排障人员需要从多条日志里重新拼出一条错误路径，字段也容易不统一。

#### 推荐做法：错误携带信息，边界统一投影

```rust
async fn handle_submit(order: Order) -> Result<HttpResponse, StructError<AppReason>> {
    submit_order(order)
        .await
        .source_err(AppReason::system_error(), "handle submit order failed")?;

    Ok(HttpResponse::ok())
}
```

边界处可以基于同一个错误对象输出不同形式：

```rust
let report = err.report();
let exposure = err.exposure(&policy);
```

这意味着：

- 业务层不需要到处拼日志字符串。
- 日志可以统一包含 `identity`、`reason`、`detail`、`context`、`source chain`。
- 边界只记录一次失败，避免重复噪声。
- 结构化字段可以被日志系统、监控系统和告警系统查询。

### `OperationContext` 日志

`OperationContext` 提供结构化日志方法，输出时自动带上当前操作的 field 和 metadata：

```rust
use orion_error::OperationContext;

let ctx = OperationContext::doing("order_processing")
    .with_field("order_id", "123")
    .with_meta("component.name", "order_service");

ctx.info("start");
ctx.warn("slow upstream");
ctx.error("final failure");
```

对于需要生命周期日志的作用域，使用 `with_auto_log()`：

```rust
let mut ctx = OperationContext::doing("sync_user")
    .with_auto_log()
    .with_field("user_id", "42");

do_sync()?;
ctx.mark_suc();
```

如果作用域在 Drop 前没有标记成功或取消，自动输出失败日志。更详细的用法参考 [日志说明](./LOGGING.md)。

推荐原则：**少量生命周期日志 + 边界错误投影**，而不是每层重复 `error!`。

---

## 3. 呈现

### 同一个错误需要面向不同对象呈现不同视图

真实系统里的错误不是只给一种人看的。

至少有几类接收者：

- 最终使用者：需要安全、可理解、可行动的信息。
- 系统调整者 / 运维 / SRE：需要组件、环境、分类、重试和影响判断。
- 开发者：需要 source chain、detail、上下文和底层错误。
- 客户端 / 上游系统：需要稳定 `code`、字段结构、retry hint 和协议形状。
- 日志 / 监控 / 告警系统：需要结构化字段，而不是长字符串。

如果只有一个错误字符串，很难同时满足这些对象。

例如：

```text
database connection failed: timeout from sqlx pool
```

这个信息：

- 给用户看太技术化。
- 给开发者看又缺少业务上下文。
- 给协议客户端看不稳定。
- 给日志系统看不可结构化查询。

### 推荐做法：内部保留完整结构，边界投影不同视图

`orion-error` 把“错误内部结构”和“外部呈现”分开：

- 内部保留 `reason`、`ErrorIdentity.code`、`detail`、`context`、`source chain`
- 面向用户时，只暴露安全、可理解、可行动的信息
- 面向系统调整者时，暴露组件、操作、分类、重试、严重性等治理信息
- 面向开发者时，使用 report 查看完整诊断链
- 面向协议时，使用 exposure 形成稳定字段结构

```rust
let report = err.report();
let exposed = err.exposure(&DefaultExposurePolicy::default());
```

同一个错误对象可以投影成不同用途：

| 对象 | 需要的信息 | 推荐投影 |
|------|------------|----------|
| 用户 | 安全 message、可行动提示 | exposure view |
| 运维 / SRE | component、operation、retryable、severity | exposure snapshot / log JSON |
| 开发者 | source chain、detail、context | report |
| 协议客户端 | stable code、字段结构、retry hint | HTTP/RPC/CLI error JSON |
| 测试 / 回归 | 稳定结构快照 | stable snapshot |

这也是 `orion-error` 与单纯展示层工具的关键区别：它不是只把错误显示得更漂亮，而是保留一个结构化错误，再按边界需求投影不同视图。

---

## 总结

`orion-error` 适合的不是“最少代码返回一个错误”，而是“让错误成为系统契约”。

它解决的核心问题是：

1. **补齐错误发生环境**：底层错误不会自动携带 path、tenant、order_id、operation 等关键上下文。
2. **抽象技术细节而不丢诊断信息**：在层边界把技术失败转换成稳定领域语义，同时保留 source 和 context。
3. **保留跨层错误传递链**：让排错看到失败如何从底层一路被解释成最终边界错误。
4. **让日志有效而克制**：错误对象携带结构化信息，边界统一记录，减少重复日志和业务代码里的日志拼接。
5. **按对象呈现不同错误视图**：同一个错误，面向用户、运维、开发者、协议客户端和日志系统，应有不同粒度和安全等级的输出。

一句话：

> `orion-error` 让错误在跨层流转中保持结构化，并在不同边界呈现正确视图。
