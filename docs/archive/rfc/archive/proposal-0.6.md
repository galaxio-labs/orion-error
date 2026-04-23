# orion-error 0.6 改进提案

> 历史提案说明：
> 本文基于更早期源码讨论 `ErrorOweBase` / `ErrorOwe` 拆分。
> 当前代码状态已经继续演进：只保留兼容态 `ErrorOweBase::owe(...)`，`ErrorOwe::owe_*()` 已移除。

> 基于 orion-error 0.5.6 源码的逐项验证报告

---

## 一、现状分析

### 1.1 核心架构（保持不变）

orion-error 的四维度模型经过验证是正确的：

- **reason: R** — WHAT（可 match 的类型化原因）
- **detail: Option\<String\>** — WHY（原始错误信息）
- **position: Option\<String\>** — WHERE（灵活的位置描述）
- **context: Arc\<Vec\<OperationContext\>\>** — WHICH（操作上下文栈）

UvsReason 的收敛机制也是正确的——它是**类型转换目标**，解决多 crate 错误在上层统一的接口类型问题。

### 1.2 需要改进的问题

---

#### 问题 1: DomainReason 的 blanket impl 强制耦合 Serialize + From\<UvsReason\>

**源码证据**:

`src/core/domain.rs:9` — trait 定义：
```rust
pub trait DomainReason: PartialEq + Display + Serialize {}
```
`Serialize` 是硬约束。

`src/core/domain.rs:11` — blanket impl：
```rust
impl<T> DomainReason for T where T: From<UvsReason> + Display + PartialEq + Serialize {}
```
额外绑定了 `From<UvsReason>`。不满足 `From<UvsReason>` 的类型必须手动 `impl DomainReason`。

**连锁影响**: `Serialize` 硬约束波及所有以 `DomainReason` 为 bound 的泛型代码：

| 文件 | 行号 | 受影响类型/trait |
|------|------|-----------------|
| `src/core/error.rs` | 36 | `StructError<T: DomainReason>` |
| `src/core/error.rs` | 84 | `StructErrorImpl<T: DomainReason>` |
| `src/traits/owenance.rs` | 11 | `ErrorOwe<T, R: DomainReason>` |
| `src/traits/conversion.rs` | 3, 7 | `ErrorConv`, `ConvStructError` |
| `src/traits/contextual.rs` | 3-7 | `ErrorWith` |

即使用户只想定义一个最简单的 `StructError<MyReason>`，`MyReason` 也必须 `derive(Serialize)`，进而拉入 `serde` 依赖。

**结论**: 论断成立。`Serialize` 应改为 feature gate。

---

#### 问题 2: ErrorOwe 的 trait bound 硬绑 From\<UvsReason\>

**源码证据**:

`src/traits/owenance.rs:9-11` — trait 定义本身只约束 `R: DomainReason`：
```rust
pub trait ErrorOwe<T, R>
where
    R: DomainReason,
```

`src/traits/owenance.rs:26-29` — 但 impl 块绑定了 `From<UvsReason>`：
```rust
impl<T, E, R> ErrorOwe<T, R> for Result<T, E>
where
    E: Display,
    R: DomainReason + From<UvsReason>,
```

**关键发现**: `.owe(self, reason: R)`（`owenance.rs:31-37`）本身不需要 `From<UvsReason>`——它接受用户传入的 `reason`，不涉及 UvsReason 转换：

```rust
fn owe(self, reason: R) -> Result<T, StructError<R>> {
    match self {
        Ok(v) => Ok(v),
        Err(e) => {
            let msg = e.to_string();
            Err(StructError::from(reason).with_detail(msg))
        }
    }
}
```

只有 `.owe_sys()` 等便捷方法（`owenance.rs:41-70`）才真正需要 `From<UvsReason>`，因为它们内部构造 `UvsReason` 再通过 `From` 转换。

但 `.owe()` 和 `.owe_sys()` 在**同一个 impl 块**里，被同一个 `From<UvsReason>` bound 覆盖，导致 `.owe()` 被无辜牵连——不需要 UvsReason 收敛的底层 crate 也无法使用 `.owe()`。

**结论**: 论断成立。拆分为 `ErrorOweBase`（只含 `.owe()`）和 `ErrorOwe`（含 `.owe_xxx()`）是合理的。

---

#### 问题 3: owe_xxx 消息重复存储

**源码证据**:

`src/traits/owenance.rs:73-84` — `map_err_with` 函数：
```rust
fn map_err_with<T, E, R, F>(result: Result<T, E>, f: F) -> Result<T, StructError<R>>
where
    E: Display,
    R: DomainReason,
    F: FnOnce(String) -> R,
{
    result.map_err(|e| {
        let msg = e.to_string();
        let reason = f(msg.clone());       // msg 传给闭包，进入 reason 变体
        StructError::from(reason).with_detail(msg)  // msg 再存入 detail
    })
}
```

以 `owe_sys`（`owenance.rs:68-70`）为例追踪完整链路：

1. `owe_sys` 调用 `map_err_with(self, |msg| R::from_sys(msg))`
2. `from_sys`（`universal.rs:250-256`）调用 `UvsReason::system_error(info)`
3. `system_error`（`universal.rs:128-130`）构造 `Self::SystemError(msg.into())`

最终结果：错误消息同时存在于：
- `UvsReason::SystemError("connection refused")` — reason 变体内
- `detail: Some("connection refused")` — detail 字段

**Display 输出验证**: `src/core/error.rs:178-209` 同时打印 reason（行 181: `{reason}`）和 detail（行 194-196: `{detail}`）。

UvsReason 的 Display（`universal.rs:59`）：`#[error("system error << {0}")]`

用户看到的输出：
```
[201] system error << connection refused
  -> Details: connection refused
```

消息出现两次。

**结论**: 论断成立。冗余影响 Display 输出可读性。

---

#### 问题 4: mod_path 始终指向 orion_error 自身

**源码证据**:

`src/core/context.rs:19` — 编译期常量：
```rust
const DEFAULT_MOD_PATH: &str = module_path!();
```
`module_path!()` 在**定义处**展开，值为 `"orion_error::core::context"`。

所有构造函数都使用此默认值：

| 构造函数 | 行号 | mod_path 值 |
|---------|------|------------|
| `default()` | 35 | `DEFAULT_MOD_PATH` |
| `want()` | 143 | `DEFAULT_MOD_PATH` |
| `new()` | 134 | `DEFAULT_MOD_PATH` |
| `From<CallContext>` | 47 | `DEFAULT_MOD_PATH` |
| `From<String>` | 322 | `DEFAULT_MOD_PATH` |
| `From<&str>` | 358 | `DEFAULT_MOD_PATH` |
| `From<(&str, &str)>` | 369 | `DEFAULT_MOD_PATH` |
| `From<(&str, String)>` | 380 | `DEFAULT_MOD_PATH` |
| `From<(String, String)>` | 419 | `DEFAULT_MOD_PATH` |
| `From<&PathBuf>` | 333 | `DEFAULT_MOD_PATH` |
| `From<&Path>` | 344 | `DEFAULT_MOD_PATH` |

`with_mod_path` 方法存在（`context.rs:155-158`），但需要调用方手动传 `module_path!()`。`module_path!()` 必须在**调用方源码中**展开才能获得调用方的模块路径，函数调用无法做到，只有宏可以。

**Drop 中的影响**（`context.rs:52-68`）：所有使用 `with_auto_log()` 但没手动调 `with_mod_path()` 的 OperationContext，日志 target 都是 `"orion_error::core::context"`。

**结论**: 论断成立。需要 `op_context!` 宏在调用处展开 `module_path!()`。

---

#### 问题 5: UvsXxxFrom trait 过度膨胀

**源码证据（`src/core/universal.rs`）**:

12 个 trait 定义（`universal.rs:155-201`）：

| 行号 | Trait | 方法 |
|------|-------|------|
| 155-157 | `UvsConfFrom<S>` | `from_conf(info: S)` |
| 159-161 | `UvsDataFrom<S>` | `from_data(info: S, pos: Option<usize>)` |
| 163-165 | `UvsSysFrom<S>` | `from_sys(info: S)` |
| 167-169 | `UvsBizFrom<S>` | `from_biz(info: S)` |
| 170-172 | `UvsLogicFrom<S>` | `from_logic(info: S)` |
| 174-176 | `UvsResFrom<S>` | `from_res(info: S)` |
| 178-180 | `UvsNetFrom<S>` | `from_net(info: S)` |
| 182-184 | `UvsTimeoutFrom<S>` | `from_timeout(info: S)` |
| 187-189 | `UvsValidationFrom<S>` | `from_validation(info: S)` |
| 191-193 | `UvsNotFoundFrom<S>` | `from_not_found(info: S)` |
| 195-197 | `UvsPermissionFrom<S>` | `from_permission(info: S)` |
| 199-201 | `UvsExternalFrom<S>` | `from_external(info: S)` |

blanket impl（`universal.rs:205-429`）：每个 trait 有 2-3 个 impl（String、&str，`UvsConfFrom` 额外有 `ConfErrReason`），共 **25 个 blanket impl**。

所有 impl 模式完全相同，以 `UvsSysFrom` 和 `UvsNetFrom` 为例：
```rust
// universal.rs:250-256
impl<T> UvsSysFrom<String> for T where T: From<UvsReason> {
    fn from_sys(info: String) -> Self { T::from(UvsReason::system_error(info)) }
}
// universal.rs:304-310
impl<T> UvsNetFrom<String> for T where T: From<UvsReason> {
    fn from_net(info: String) -> Self { T::from(UvsReason::network_error(info)) }
}
```

唯一差异是调用不同的 `UvsReason` 构造函数。所有 trait 共享约束 `T: From<UvsReason>`，完全可以合并。

**结论**: 论断成立。统一为 `UvsFrom` trait 可以用 1 个 trait + 1 个 blanket impl 替代 12 个 trait + 25 个 blanket impl。

---

#### 问题 6: log vs tracing

**源码证据**:

`Cargo.toml:17`:
```toml
tracing = []  # 空 feature，无 dep:tracing 依赖
```

搜索整个源码 `#[cfg(feature = "tracing")]`：**零处**。

对比 `#[cfg(feature = "log")]` 的使用：

| 文件 | 行号 | 用途 |
|------|------|------|
| `context.rs:2-3` | import log 宏 |
| `context.rs:55` | Drop impl 中的日志 |
| `context.rs:217-222` | `info()` 方法 |
| `context.rs:224-229` | `debug()` 方法 |
| `context.rs:231-236` | `warn()` 方法 |
| `context.rs:238-243` | `error()` 方法 |
| `context.rs:245-250` | `trace()` 方法 |

tracing feature 声明了但完全没有对应代码分支，且 `tracing` crate 不在 `[dependencies]` 中。

**结论**: 论断成立。tracing feature 是空壳。

---

#### 问题 7: OperationContext 的 exit_log 耦合

**源码证据**:

`OperationContext`（`context.rs:21-27`）同时承担两个角色：

**角色 1 — 错误上下文元素**:
```rust
// error.rs:88
context: Arc<Vec<OperationContext>>,
```

**角色 2 — RAII 日志守卫**:
```rust
// context.rs:52-68 Drop impl
impl Drop for OperationContext {
    fn drop(&mut self) {
        if self.exit_log { ... }
    }
}
```

**潜在问题**: 当 OperationContext 被 clone 附加到 StructError 时（`error.rs:169`）：
```rust
fn add_context(&mut self, ctx: &OperationContext) {
    Arc::make_mut(&mut self.imp.context).push(ctx.clone());
}
```

clone 副本保留了 `exit_log` 字段值。如果原始 ctx 的 `exit_log == true`，clone 副本 drop 时也会触发日志。原始 ctx 可能已 `mark_suc()`，但副本保持默认 `Fail` 状态，会产生一条误导性的 "fail!" 日志。

**严重程度**: 中低。取决于实际场景中 `with_auto_log()` 的 `OperationContext` 是否会通过 `.attach_context(&ctx)` 传入 `StructError`。

**结论**: 耦合存在，隐患真实但非高频。**可选改进**，不是必须。

---

#### 问题 8: std::error::Error 缺少 source()

**源码证据**:

`src/core/error.rs:35-38`:
```rust
#[derive(Error, Debug, Clone, PartialEq, Getters)]
pub struct StructError<T: DomainReason> {
    imp: Box<StructErrorImpl<T>>,
}
```

`thiserror` 的 `#[derive(Error)]` 在没有 `#[source]` 标注时 `source()` 返回 `None`。

`StructErrorImpl`（`error.rs:83-89`）无 source 字段。原始错误在 `map_err_with`（`owenance.rs:79-83`）中被 `.to_string()` 后存入 `detail`，原始 `Error` 对象丢弃。

**约束冲突**: 增加 `source: Option<Box<dyn Error + Send + Sync>>` 会导致：
- `StructError` 失去 `Clone`（`dyn Error` 不能 Clone）
- `StructError` 失去 `PartialEq`（`dyn Error` 不能 PartialEq）
- 序列化丢失 source（`dyn Error` 不能 Serialize）

当前 `Clone` 和 `PartialEq`（`error.rs:35`）在测试断言和错误传递中被广泛使用。

**结论**: 论断成立，但代价过高。**建议不改**，detail 字符串足够。

---

#### 问题 9: 依赖过重

**源码证据**:

**serde** (`Cargo.toml:22`): `serde = { version = "1.0", features = ["derive", "rc"] }`
- `features = ["rc"]` 用于序列化 `Arc<Vec<OperationContext>>`（`error.rs:88`）
- 无条件强制依赖

**derive_more** (`Cargo.toml:23`): `derive_more = { version = "2.1", features = ["full"] }`

实际使用搜索：

| 文件 | 行号 | 使用 |
|------|------|------|
| `src/core/domain.rs` | 3 | `use derive_more::From;`（`NullReason`） |
| `src/core/error.rs` | 274 | `use derive_more::From;`（测试代码） |

只用了 `From`，但 `features = ["full"]` 拉入了 Display, Add, Mul, Not, Index, Deref 等几十个未使用的 derive 宏。

**derive-getters** (`Cargo.toml:21`): `derive-getters = "0.5"`

| 文件 | 行号 | 生成的 getter 数 |
|------|------|-----------------|
| `src/core/context.rs` | 1, 20 | 5 个（context, result, exit_log, mod_path, target） |
| `src/core/error.rs` | 10, 83 | 4 个（reason, detail, position, context） |

共 9 个 getter，手写约 30 行代码即可替代一整个外部依赖。

**结论**: 论断成立。serde 改 optional，derive_more 改 `features = ["from"]`，derive-getters 删除。

---

## 二、改进方案

### 2.1 不改的部分

| 设计 | 源码位置 | 保持理由 |
|------|---------|---------|
| `StructError<T>` 四维度模型 | `error.rs:83-89` | reason/detail/position/context 分离合理 |
| `position: Option<String>` | `error.rs:87` | 灵活——"file.wfl:5:10"、"row 200"、"$.data[3]" 均可 |
| `Arc<Vec<OperationContext>>` | `error.rs:88` | 错误转换传递时避免 clone 整个 context 栈 |
| UvsReason 收敛机制 | `domain.rs:11` | 跨 crate 边界的接口类型统一 |
| ErrorOwe / ErrorConv / ErrorWith | `traits/` | 核心转换 API |
| OperationContext exit_log | `context.rs:52-68` | RAII 日志解决 `?` 传播中的分级日志问题 |

### 2.2 改进 A: 依赖瘦身 + feature gate

**涉及文件**: `Cargo.toml`, `domain.rs`, `universal.rs`, `context.rs`, `error.rs`

```toml
# Cargo.toml 变更
[dependencies]
thiserror = "2"
derive_more = { version = "2.1", features = ["from"] }   # 原 features = ["full"]
log = { version = "0.4", optional = true }
tracing = { version = "0.1", optional = true }             # 新增实际依赖
serde = { version = "1.0", features = ["derive", "rc"], optional = true }  # 原为强制

# 删除 derive-getters = "0.5"

[features]
default = ["log"]
serde = ["dep:serde"]
tracing = ["dep:tracing"]
```

**代码变更**:

```rust
// domain.rs:9 — 去掉 Serialize
pub trait DomainReason: PartialEq + Display {}

// 所有 Serialize derive 改为 cfg_attr，示例：
#[derive(Debug, Error, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum UvsReason { ... }

// error.rs — derive-getters 替换为手写 getter（约 30 行）
impl<T: DomainReason> StructErrorImpl<T> {
    pub fn reason(&self) -> &T { &self.reason }
    pub fn detail(&self) -> &Option<String> { &self.detail }
    pub fn position(&self) -> &Option<String> { &self.position }
    pub fn context(&self) -> &Arc<Vec<OperationContext>> { &self.context }
}
```

**影响**: Breaking change（DomainReason trait 定义变更），适合 0.6.0。

---

### 2.2 改进 B: UvsFrom 统一 trait

**涉及文件**: `universal.rs`, `core/mod.rs`, `lib.rs`

```rust
// universal.rs — 新增统一 trait
pub trait UvsFrom: From<UvsReason> + Sized {
    fn from_sys(msg: impl Into<String>) -> Self {
        Self::from(UvsReason::system_error(msg))
    }
    fn from_conf(msg: impl Into<String>) -> Self {
        Self::from(UvsReason::core_conf(msg))
    }
    fn from_data(msg: impl Into<String>, pos: Option<usize>) -> Self {
        Self::from(UvsReason::data_error(msg, pos))
    }
    fn from_net(msg: impl Into<String>) -> Self {
        Self::from(UvsReason::network_error(msg))
    }
    fn from_timeout(msg: impl Into<String>) -> Self {
        Self::from(UvsReason::timeout_error(msg))
    }
    fn from_res(msg: impl Into<String>) -> Self {
        Self::from(UvsReason::resource_error(msg))
    }
    fn from_biz(msg: impl Into<String>) -> Self {
        Self::from(UvsReason::business_error(msg))
    }
    fn from_logic(msg: impl Into<String>) -> Self {
        Self::from(UvsReason::logic_error(msg))
    }
    fn from_validation(msg: impl Into<String>) -> Self {
        Self::from(UvsReason::validation_error(msg))
    }
    fn from_not_found(msg: impl Into<String>) -> Self {
        Self::from(UvsReason::not_found_error(msg))
    }
    fn from_permission(msg: impl Into<String>) -> Self {
        Self::from(UvsReason::permission_error(msg))
    }
    fn from_external(msg: impl Into<String>) -> Self {
        Self::from(UvsReason::external_error(msg))
    }
}
impl<T: From<UvsReason>> UvsFrom for T {}
```

用户导入：`use orion_error::UvsFrom;`（1 行）替代 `use orion_error::{UvsSysFrom, UvsConfFrom, ...};`（12 行）。

**影响**: 非 breaking（纯新增）。0.5.7 可引入，同时 deprecate 旧 12 个 trait。0.6.0 删除旧 trait。

---

### 2.3 改进 C: ErrorOwe 解耦 From\<UvsReason\>

**涉及文件**: `traits/owenance.rs`, `traits/mod.rs`, `lib.rs`

```rust
// owenance.rs — 拆分为两个 trait

/// 基础 owe：只需 DomainReason，不需要 From<UvsReason>
pub trait ErrorOweBase<T, R: DomainReason> {
    fn owe(self, reason: R) -> Result<T, StructError<R>>;
}

impl<T, E: Display, R: DomainReason> ErrorOweBase<T, R> for Result<T, E> {
    fn owe(self, reason: R) -> Result<T, StructError<R>> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => {
                let msg = e.to_string();
                Err(StructError::from(reason).with_detail(msg))
            }
        }
    }
}

/// 便捷 owe_xxx：需要 From<UvsReason>
pub trait ErrorOwe<T, R: DomainReason + From<UvsReason>> {
    fn owe_sys(self) -> Result<T, StructError<R>>;
    fn owe_conf(self) -> Result<T, StructError<R>>;
    fn owe_net(self) -> Result<T, StructError<R>>;
    // ...（保持现有方法列表）
}
```

**迁移影响**:
- 使用 `.owe_sys()` 等的代码：无变化（`use ErrorOwe` 不变）
- 使用 `.owe(reason)` 的代码：需额外 `use ErrorOweBase`

---

### 2.4 改进 D: op_context! 宏修复 mod_path

**涉及文件**: `context.rs`（新增宏定义）

```rust
/// 在调用处展开 module_path!()，使 mod_path 正确指向调用方模块
#[macro_export]
macro_rules! op_context {
    ($target:expr) => {
        $crate::OperationContext::want($target)
            .with_mod_path(module_path!())
    };
}
```

调用方从：
```rust
let mut op = OperationContext::want("load-engine-res").with_auto_log();
```
改为：
```rust
let mut op = op_context!("load-engine-res").with_auto_log();
```

**影响**: 非 breaking（纯新增）。0.5.7 可引入。

---

### 2.5 改进 E: tracing feature 实际生效

**涉及文件**: `Cargo.toml`, `context.rs`

Drop impl 增加 tracing 分支（`context.rs:52-68`）：

```rust
impl Drop for OperationContext {
    fn drop(&mut self) {
        if !self.exit_log { return; }

        #[cfg(feature = "tracing")]
        {
            let ctx = self.format_context();
            match self.result() {
                OperationResult::Suc => {
                    tracing::info!(target: self.mod_path.as_str(), op = %ctx, "suc");
                }
                OperationResult::Fail => {
                    tracing::error!(target: self.mod_path.as_str(), op = %ctx, "fail");
                }
                OperationResult::Cancel => {
                    tracing::warn!(target: self.mod_path.as_str(), op = %ctx, "cancel");
                }
            }
        }
        #[cfg(all(feature = "log", not(feature = "tracing")))]
        {
            // 现有 log 宏代码保持不变
        }
    }
}
```

logging 方法同理（`context.rs:217-250`），每个方法增加 `#[cfg(feature = "tracing")]` 分支。

**优先级策略**: tracing 优先于 log（`#[cfg(all(feature = "log", not(feature = "tracing")))]`）。

---

### 2.6 改进 F: owe_xxx 去除消息冗余

**涉及文件**: `universal.rs`, `owenance.rs`

**UvsReason 变体简化**（`universal.rs:31-86`）：

```rust
// Before (0.5.x)                    // After (0.6)
SystemError(String),           →     System,
NetworkError(String),          →     Network,
ValidationError(String),       →     Validation,
BusinessError(String),         →     Business,
RunRuleError(String),          →     RunRule,
NotFoundError(String),         →     NotFound,
PermissionError(String),       →     Permission,
DataError(String, Option<usize>), →  Data,
ResourceError(String),         →     Resource,
TimeoutError(String),          →     Timeout,
ExternalError(String),         →     External,
LogicError(String),            →     Logic,
ConfigError(ConfErrReason),    →     Config(ConfErrReason),  // 保留子分类
```

**map_err_with 变更**（`owenance.rs:73-84`）：

```rust
// Before: 闭包接收 msg
fn map_err_with<T, E, R, F>(result: Result<T, E>, f: F) -> Result<T, StructError<R>>
where E: Display, R: DomainReason, F: FnOnce(String) -> R,
{
    result.map_err(|e| {
        let msg = e.to_string();
        let reason = f(msg.clone());       // msg 进入 reason
        StructError::from(reason).with_detail(msg)  // msg 又进入 detail
    })
}

// After: 闭包不接收 msg
fn map_err_with<T, E, R, F>(result: Result<T, E>, f: F) -> Result<T, StructError<R>>
where E: Display, R: DomainReason, F: FnOnce() -> R,
{
    result.map_err(|e| {
        let detail = e.to_string();
        let reason = f();                              // reason 不含 msg
        StructError::from(reason).with_detail(detail)  // msg 只在 detail
    })
}
```

**Display 输出对比**:
```
// Before:
[201] system error << connection refused
  -> Details: connection refused

// After:
[201] system error
  -> Details: connection refused
```

**影响**: Breaking change。所有使用 `UvsReason::system_error("msg")` 的代码需要更新。适合 0.6.0。

---

## 三、可选改进

| 改进 | 内容 | 复杂度 | 建议 |
|------|------|--------|------|
| Error::source() | 增加 `source: Option<Box<dyn Error + Send + Sync>>` | 高 | **不做**。与 Clone/PartialEq 冲突，detail 字符串够用 |
| OpGuard 独立类型 | 从 OperationContext 抽出 RAII 日志守卫 | 中 | **可选**。clone + exit_log 隐患真实但非高频 |
| ErrorCategory enum | `fn category() -> ErrorCategory` 替代 `category_name() -> &str` | 低 | 可做 |

---

## 四、版本策略

```
0.5.7  — 新增 UvsFrom 统一 trait（改进 B），deprecate 12 个旧 trait
       — 新增 op_context! 宏（改进 D）
       — 新增 tracing feature 实际生效（改进 E）
       — ErrorOweBase 拆分（改进 C，非 breaking，纯新增 trait）
       — derive_more features 缩减为 ["from"]

0.6.0  — DomainReason 去掉 Serialize 硬约束，serde 改 feature gate（改进 A）
       — UvsReason 变体去掉 String，消息统一存 detail（改进 F）
       — 删除 derive-getters 依赖，手写 getter
       — 删除 deprecated 的 12 个 UvsXxxFrom trait
```

---

## 五、验证清单

| 验证项 | 命令 |
|-------|------|
| orion-error 无 feature | `cargo test -p orion-error --no-default-features` |
| orion-error 全 feature | `cargo test -p orion-error --all-features` |
| orion-error serde only | `cargo test -p orion-error --features serde` |
| orion-error tracing only | `cargo test -p orion-error --features tracing` |
| 下游兼容（0.5.x） | `cd wp-motor && cargo test --workspace` |
| 下游引入（0.6） | `cd wp-reactor && cargo build --workspace && cargo test --workspace` |
