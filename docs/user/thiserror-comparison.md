# 与 thiserror 的关系

`orion-error` 和 `thiserror` 不是互斥关系，但定位不同。

## 定位差异

**thiserror**：定义标准 Rust error 类型，服务于 `std::error::Error` 生态。

**orion-error**：定义运行时结构化错误载体，管理上下文、source frame、快照、协议投影。

## 能力对比

| 能力 | thiserror | orion-error |
|------|-----------|-------------|
| 定义标准错误类型 | 强 | 不是主要目标 |
| 领域 reason derive | 需要额外补稳定身份 | `OrionError` 是推荐入口 |
| 运行时结构化上下文 | 无 | 有 |
| source frame 追踪 | 无 | 有 |
| stable code / category | 无 | 有 |
| snapshot / report / projection | 无 | 有 |

## 什么场景保留 thiserror

- 对外公开标准 `std::error::Error` 类型
- 外部库 API 要求标准 error 类型
