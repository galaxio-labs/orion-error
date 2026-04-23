# 文档导航

本文档目录只描述 `orion-error` 当前最终状态；历史阶段、迁移计划和版本阶段叙事不再作为使用者文档保留。

建议阅读顺序：

1. [顶层 README](../README.md)
2. [使用教程](./tutorial.md)
3. [DomainReason 与 ErrorIdentityProvider 使用指南](./reason-identity-guide.md)
4. [协议契约](./protocol-contract.md)
5. [Stable Snapshot Schema](./stable-snapshot-schema.md)
6. [与 thiserror 的配合](./thiserror-comparison.md)
7. [日志说明](./LOGGING.md)

## 当前导入约定

- 新代码优先使用 `orion_error::prelude::*` 或 crate root 小集合导入。
- 需要明确职责边界时，使用 `runtime` / `conversion` / `reason` / `snapshot` / `report` / `bridge`。
- 旧 `owe(...)` / `err_wrap(...)` 等兼容 helper 必须显式使用 `orion_error::compat_prelude::*` 或 `orion_error::compat_traits::*`。
- 公开命名空间不使用版本阶段作为导入层级。

## 当前主路径

- 普通错误第一次进入结构化体系：`into_as(...)`
- 已结构化错误向上层建立新边界：`wrap_as(...)`
- 普通 source：`with_std_source(...)`
- 结构化 source：`with_struct_source(...)`
- 完整上下文帧：`with_context(...)`
- 上下文语义糖衣：`at(...)` / `doing(...)`
- 稳定身份和出口协议：`identity_snapshot()` / `policy_snapshot(...)` / `http_response(...)` / `cli_response(...)` / `log_response(...)` / `rpc_response(...)`

## 兼容边界

只为已经公开过的旧语义保留兼容入口；新增概念不再额外保留兼容别名或过渡投影。

如果文档与源码冲突，请以 `src/`、测试和顶层 README 为准。
