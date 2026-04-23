# V3 Stable Code Policy

更新时间：2026-04-22

本文档用于冻结 `orion-error` 在 `V3` 第一批里的稳定错误身份策略。

这份策略只回答一个核心问题：

> 错误的长期稳定身份，到底应该靠什么来断言、传递和治理？

当前结论是：

- 长期稳定身份优先使用字符串 `code`
- `category` 是粗粒度治理维度
- `detail` 不是错误身份
- 现有 `i32 error_code()` 继续保留，但不再作为长期协议主键

## 1. 当前问题

当前代码里已经有：

- `ErrorCode::error_code() -> i32`
- `reason: R`
- `detail`

这足够支撑运行时传播与基本分类，但还不够支撑长期协议。

主要问题：

1. `i32` 编号更像兼容时代编号，不够表达跨项目稳定身份
2. 测试仍容易回到断言 `reason.to_string()` 或 `detail` 文本
3. CLI / HTTP / 日志 / telemetry 很难共享一套稳定主键

## 2. V3 的稳定身份结论

`V3` 里错误身份分成三层：

1. `code`
   - 稳定主键
2. `category`
   - 粗粒度治理分桶
3. `detail`
   - 补充解释，不参与身份判定

因此：

- `code` 变化应视为兼容性事件
- `detail` 改文案不应视为错误身份变化
- `category` 不能替代 `code`

## 3. 推荐命名规则

推荐使用：

```text
<domain>.<kind>
```

例如：

- `conf.file_not_found`
- `conf.invalid_value`
- `biz.reload_in_progress`
- `logic.unsupported_file_type`
- `sys.io_error`
- `sys.network_timeout`

当前约束：

- 只使用 ASCII 小写
- 使用 `.` 分隔 domain 与 kind
- 使用 `_` 连接 kind 内的多单词片段
- 不把动态值拼进 code

错误示例：

- `config.toml missing`
- `reload failed for tenant-a`
- `SystemError`
- `300`

这些都不适合作为长期稳定 code。

## 4. 与现有 `i32 error_code()` 的关系

当前 `i32 error_code()` 不立即删除。

它在 `V3` 第一批里的定位是：

- 兼容编号
- 排序/展示辅助信息
- 历史迁移过渡值

它不再承担：

- 长期协议主键
- 跨出口统一身份
- 首选测试断言字段

因此，后续推荐优先级应调整为：

1. `code`
2. `category`
3. `error_code()`

而不是继续相反。

## 5. 与 `Reason` 的关系

`Reason` 仍然是错误语义的主载体。

`V3` 不建议在第一批里直接把现有 `DomainReason` 强行升级成重型 trait。

第一批更稳妥的做法是：

- 先增加稳定 code 的观察能力
- 再逐步把 `snapshot` / `report` / 测试 helper 收敛到 `code`
- 最后再评估是否把 code/category 正式提升为 `Reason` trait 硬约束

这样可以避免一上来把所有 reason 枚举和下游实现都打碎。

## 6. 测试策略

从 `V3` 开始，建议测试优先断言：

- `code`
- `category`
- 稳定 meta 字段

而不是优先断言：

- `detail` 全文本
- CLI 文本渲染结果
- `Display` 拼接后的长字符串

推荐 helper：

- `assert_err_code(&err, "biz.reload_in_progress")`
- `assert_err_category(&err, ErrorCategory::Biz)`

如果必须断言 `detail`，也应把它视为辅助断言，而不是身份断言。

## 7. 对 snapshot / report 的要求

`V3` 后续代码演进时：

- `snapshot` 应具备稳定 `code`
- `report` 应显式消费 `code`
- renderer / policy 应基于 `code` 做稳定映射

尤其是：

- HTTP 状态码映射应优先基于 `code / category`
- telemetry 分桶应优先基于 `code`
- 测试快照应优先断言 `code`

## 8. 对迁移的要求

从 V2 迁到 V3 时，建议分三步：

1. 先给核心 reason 补稳定 code 表
2. 再把测试 helper 和 snapshot/report 收敛到 code
3. 最后再考虑是否提升 trait 约束

不建议直接：

- 全面替换所有现有断言
- 一步到位删除 `i32 error_code()`
- 一步到位重写 reason trait

## 9. 第一批实现建议

第一批代码层建议只做：

1. 新增稳定 code 观察 trait 或 helper
2. 给现有内建 reason 提供稳定 code
3. 增加 code/category 断言 helper
4. 为 snapshot/report 预留 code/category 演进位

这一步先不承诺：

- 立即统一所有下游自定义 reason
- 立即统一所有历史 JSON 形状
- 立即把当前所有文档断言改完

## 10. 当前状态说明

截至 `2026-04-22`：

- 本文档只冻结 `V3` 第一批的稳定身份规则
- 当前线上实现仍以 `V2` 为主
- 现有 `ErrorCode::error_code() -> i32` 仍然有效

因此，当前正确口径是：

- `i32 error_code()` 仍可继续使用
- 但 `V3` 新设计和新测试，应开始向稳定字符串 `code` 收敛
