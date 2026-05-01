# Public Surface Grading

更新时间：2026-04-30

本文档基于当前 `orion-error 0.8.x` 代码，给公开 surface 做分级整理。

目标不是继续删除 API，而是固定下面四类边界：

1. 主路径 API
2. 观察面 API
3. 测试 / 适配器入口
4. 兼容保留 API

如果后续要继续提升到 `9+`，这份分级表应作为 public API review 的参考基线。

## 1. 主路径 API

这些 API 构成当前推荐主路径，应长期稳定保留：

- `StructError<R>`
- `OperationContext::doing(...)`
- `OperationContext::at(...)`
- `with_context(...)`
- `with_source(...)`
- `StructErrorBuilder::source(...)`
- `report()`
- `render()`
- `snapshot().stable_export()`
- `identity_snapshot()`
- `exposure_snapshot(...)`
- `source_err(...)`
- `conv_err()`

- `cli::print_error(...)`

特征：

- README / tutorial / docs 主文档会优先描述它们
- 新业务代码默认优先使用它们
- 不应再为相同任务引入并列“主路径”

## 2. 观察面 API

这些 API 有明确价值，但更适合诊断、测试、观测、辅助断言：

- `source_frames()`
- `root_cause_frame()`
- `source_payload()`
- `source_payload_kind()`
- `action_main()`
- `locator_main()`
- `target_path()`
- `render_redacted(...)`
- `render_user_debug()`
- `render_user_debug_redacted(...)`

特征：

- 它们不是主传播 / 主构造入口
- 应在文档里明确属于 observation / diagnostics surface
- 不应在 quick start 中抢占主路径叙事位

## 3. 测试 / 适配器入口

这些 API 主要服务测试、schema 校验、中间层适配或协议拼装：

- `ErrorProtocolSnapshot::from_report_skeleton(...)`
- `dev::prelude::*`
- `dev::testing::*`
- `interop::*`
- `runtime::source::*`
- snapshot / stable snapshot 之间的兼容转换路径

特征：

- 允许公开存在
- 但应明确不是正常业务主路径
- 文档中应把它们描述成 secondary path
- 其中 `dev::prelude::*` 应保持在对象级检查面，不再扩成 frame 级宽导出

## 4. 兼容保留 API

这些字段或投影仍然有现实兼容价值，但名字本身带有历史包袱：

- context / snapshot frame 中的 `target`

当前统一口径：

- runtime 主语义应优先理解为 `action` / `locator` / path segments
- `target` 继续存在，主要作为 compat projection
- `path` 是稳定导出的路径投影

## 5. 当前结论

当前 `orion-error` 的主要结构问题已经不是“大量兼容 API 混在主路径里”，而是：

- 少量 compat projection 字段仍公开存在
- 少量 observation / secondary path 仍需要靠文档说明降级

这意味着下一阶段如果要继续打磨：

- 不应再优先做内部模型重写
- 应优先做 public surface review 与分级锁定

## 6. 后续建议

如果进入下一个版本线，可以按这个顺序评估：

1. 是否继续保留 frame 中的 `target`
2. 是否需要继续缩窄 `dev::prelude::*`
3. 是否要给 observation / adapter API 增加更明确的模块或命名提示

在没有明确版本策略前，当前更合理的做法是：

- 保持主路径稳定
- 保持观察面可用
- 用文档和测试锁住 secondary / compat 的定位
