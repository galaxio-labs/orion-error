# V1 Closure Summary

更新时间：2026-04-21

本文档用于记录 `orion-error 0.6.x / V1 API` 这一轮修复与文档收口的最终结论。

## 1. 结论

本轮 V1 修复可以判定为：

- 实现层通过
- 主文档层通过
- 历史设计文档已完成降级收口

当前仓库内，`V1` 主路径已经固定为：

- 普通错误第一次进入结构化体系：`into_as(...)`
- 已结构化错误向上层建立新语义边界：`wrap_as(...)`
- 已结构化错误仅做 reason 映射：`err_conv()`
- 普通 source：`with_std_source(...)`
- 结构化 source：`with_struct_source(...)`
- 上下文主命名：`doing(...)` / `at(...)`

compat 路径仍然保留，但不再属于 V1 主叙事：

- `owe_*()` / `owe_*_source()`
- `err_wrap(...)`
- `want(...)`
- `with_source(...)`

## 2. 已锁住的关键边界

- `into_as(...)` 不再提供 `E: StdError` blanket impl
- `IntoAs` 只对封闭的 `UnstructuredSource` 开放
- `raw_source(...)` 只接受显式 opt-in 的 `RawStdError`
- `StructError<_>` 不能进入 `raw_source(...)`
- `wrap_as(...)` 已作为独立公开 trait 路径落地
- `prelude::*` 与 compat 导入已拆分为不同导出面

这些边界当前由 API 形状、测试、doctest 与 `compile_fail` 共同约束。

## 3. 文档状态

当前一线入口文档：

1. `README.md`
2. `docs/v1-fix-and-review-plan.md`
3. `docs/tutorial.md`
4. `docs/v1-migration-checklist.md`
5. `docs/thiserror-comparison.md`

这些文档已经统一到 V1 主路径口径。

`docs/error-handling/01-08` 当前统一视为：

- 历史设计参考
- 治理思路记录
- 非当前 V1 API 使用手册

对应目录入口已明确声明这一点。

## 4. 剩余记账

以下问题没有在 V1 中假装彻底解决，继续记账到 V2：

- `StructError: StdError` 的根冲突
- 标准错误生态与结构化错误体系的终局桥接模型
- `StructError` 如退出 `StdError` 后的最终 API 形态
- 其他非主入口历史文档的长期治理

## 5. 使用建议

如果后续继续开发 `0.6.x`：

- 新代码只沿 V1 主路径增加用法
- 不再把 compat API 写回主文档叙事
- 不再重新引入 blanket `StdError` 风格入口
- 文档如与实现冲突，优先按 `README`、tutorial、migration checklist、源码和测试纠偏
