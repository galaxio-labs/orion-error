# V3 最小落地计划

更新时间：2026-04-22

本文档用于冻结 `orion-error` 的 `V3` 最小可落地范围。

这里的目标不是一次性完成 RFC 里所有 `V3` 设想，而是先把最值得做、最不容易漂、最能形成工程收益的部分拆出来。

一句话：

> V3 第一批先做协议，不先做大重构。

## 1. V3 当前要解决什么

`V1` 主要解决旧入口收口与 compat 管理。

`V2` 主要解决：

- source 双通道模型
- `StructError` 与 `StdError` 的边界
- runtime / snapshot / report 分层

但到目前为止，`orion-error` 仍然主要是一个“更干净的错误模型”，还不是一个“稳定工程协议”。

当前仍缺：

1. 稳定的错误身份主键
2. 稳定的出口消费协议
3. 稳定的自动检查规则

这三件事才是 `V3` 第一批最值得做的内容。

## 2. 第一批明确不做什么

`V3` 第一批先不做：

- 不一次性重写 runtime carrier
- 不直接把 `DomainReason` 升级成重型 trait
- 不直接废掉当前 `ErrorMetadata`
- 不直接做完整 typed roundtrip
- 不把所有 renderer / policy / meta 一次性做完

原因很简单：

- 这些改动侵入面大
- 迁移成本高
- 但并不一定最先带来协议收益

## 3. 第一批范围

`V3` 第一批只拆成三段：

1. `V3-A`：稳定身份
2. `V3-B`：出口协议
3. `V3-C`：规范 enforcement

其中：

- `V3-A` 必须先做
- `V3-B` 可以在 `V3-A` 基础上推进
- `V3-C` 需要在前两者有了明确协议后再固化

## 4. V3-A：稳定身份

目标：

- 让错误身份不再主要依赖 `detail` 或展示文本
- 给测试、日志、HTTP/RPC、治理系统提供稳定主键

第一批约束：

- 引入稳定字符串 `code`
- 明确 `category` 的协议地位
- 保留现有 `i32 error_code()` 作为兼容编号，不把它继续当成长期协议主键

第一批产物：

- `docs/v3-stable-code-policy.md`
- `code` 命名规则
- code/category 的测试断言 helper 设计草案

第一批完成标准：

- 新测试可以优先断言 `code`
- `snapshot` / `report` 的后续演进以 `code` 为稳定身份字段
- 新文档不再把 `detail` 当成错误身份

## 5. V3-B：出口协议

目标：

- 让 CLI / HTTP / RPC / 日志 / 测试不再各自解释错误字段
- 建立统一的 report policy / renderer 边界

第一批约束：

- 不从 runtime 直接拼接所有出口文案
- `snapshot` 不等于最终展示文本
- `renderer` 不反向污染 runtime / snapshot 模型

第一批产物：

- `ErrorPolicy` 草案
- `ErrorRenderer` 草案
- `CLI` / `HTTP` / `log` / `test` 的默认消费规则
- 最小默认 text renderer 与默认 policy 实现
- 显式 policy wrapper，避免 policy 再回退到从 report 文本猜身份

第一批完成标准：

- 至少有一份稳定的出口消费文档
- 后续新增输出场景时，优先补 policy / renderer，而不是继续给 `StructError` 加导出职责

## 6. V3-C：Enforcement

目标：

- 让协议不只停留在文档和人工 code review

第一批约束：

- 新代码不得继续把 deprecated / compat 路径当主路径
- 测试应优先断言稳定字段，而不是脆弱文本

第一批产物：

- migration checker 草案
- banlist / grep 规则
- report/snapshot 断言 helper 草案
- repo 内 enforcement 脚本与 CI 检查入口
- 非阻塞的 docs/tests 迁移扫描模式

建议第一批检查项：

- 禁止新代码继续写 deprecated `want(...)`
- 禁止把 compat-only 导入当作主路径
- 禁止领域边界继续返回 `Result<T, String>`
- 测试优先断言 `code / category / meta`

## 7. 推荐执行顺序

按以下顺序推进：

1. 先落 `V3-A` 文档与最小接口草案
2. 再落 `V3-B` 的 policy / renderer 草案
3. 最后补 `V3-C` 的 enforcement 工具

不建议反过来做。

原因：

- 没有稳定身份，就没有稳定 policy
- 没有稳定 policy，就很难写 enforcement

## 8. 第一批代码切口建议

在代码层，`V3` 第一批建议优先做下面这些低风险切口：

1. 给 `Reason` 体系补稳定字符串 code 的只读观察入口
2. 给 `snapshot` / `report` 预留 `code` / `category` 字段演进位
3. 增加 `assert_err_code(...)` / `assert_err_category(...)` helper
4. 新增 policy trait 草案，不急着全面替换当前渲染实现

这里故意不把：

- typed meta 强建模
- runtime carrier 大重构
- reason trait 大升级

放进第一批。

## 9. 当前状态

截至 `2026-04-22`：

- `V3` 仍处于规划启动阶段
- 当前已开始冻结第一批最小落地范围
- `stable code/category` 的第一批只读 trait 与 `identity snapshot` 已开始进入代码层
- `ErrorPolicy` / `ErrorRenderer` / 默认 text renderer 已开始进入代码层
- `ErrorPolicyView` 已开始进入代码层，用于把稳定身份与 report 绑定成显式出口协议输入
- `scripts/check-v3-policy.sh` 已开始进入仓库，用于检查 deprecated 主路径、compat-only 导入和 `Result<T, String>` 误用
- enforcement 脚本已支持 `--report-only`，用于对 `docs/` / `tests/` 做迁移扫描而不阻塞 CI
- `V2` 仍是当前线上代码的主实现基线

因此，如果文档与实现冲突：

- 当前行为以 `src/`、测试和 `README` 为准
- `V3` 文档当前只代表下一阶段协议方向，不代表已经全部落地
