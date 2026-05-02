# Documentation

> [中文版](./README.zh-CN.md)

Suggested reading order: start with `user/` or `user-en/` to learn concepts and usage, then check `dev/` or `dev-en/` for design details.

---

## [user/](./user/) — User Guide (中文)

| Document | Description |
|----------|-------------|
| [为什么需要 orion-error](./user/why-orion-error.md) | Error governance motivation and examples |
| [使用教程](./user/tutorial.md) | Getting started tutorial |
| [OrionError 与稳定身份](./user/reason-identity-guide.md) | Defining domain reason types |
| [协议契约](./user/protocol-contract.md) | Exposure projection contract |
| [Report / Exposure Boundary](./user/report-exposure-boundary.md) | Diagnostic vs exposure boundary |
| [日志说明](./user/LOGGING.md) | Logging integration |
| [与 thiserror 的关系](./user/thiserror-comparison.md) | Comparison with thiserror |
| [与生态方案对比](./user/ecosystem-comparison.md) | Comparison with anyhow/thiserror/color-eyre |
| [设计约束](./user/design-constraints.md) | Known design constraints |

## [user-en/](./user-en/) — User Guide (English)

| Document | Description |
|----------|-------------|
| [Why orion-error](./user-en/why-orion-error.md) | Error governance motivation and examples |
| [Tutorial](./user-en/tutorial.md) | Getting started tutorial |
| [Protocol Contract](./user-en/protocol-contract.md) | Exposure projection contract |
| [Report / Exposure Boundary](./user-en/report-exposure-boundary.md) | Diagnostic vs exposure boundary |
| [Comparison with thiserror](./user-en/thiserror-comparison.md) | Differences and coexistence |
| [Ecosystem Comparison](./user-en/ecosystem-comparison.md) | anyhow / thiserror / color-eyre / orion-error |
| [Design Constraints](./user-en/design-constraints.md) | Known design constraints |

## [dev/](./dev/) — Developer Guide (中文)

| Document | Description |
|----------|-------------|
| [API Contract](./dev/api-contract.md) | Public API boundaries |
| [Public Surface Grading](./dev/public-surface-grading.md) | Layered export grading |
| [Release Checklist](./dev/release-checklist.md) | Pre-release checks |
| [Performance Benchmarks](./dev/perf/structerror-allocation.md) | Allocation benchmarks |

## [dev-en/](./dev-en/) — Developer Guide (English)

| Document | Description |
|----------|-------------|
| [API Contract](./dev-en/api-contract.md) | Public API boundaries |
| [Public Surface Grading](./dev-en/public-surface-grading.md) | Layered export grading |
| [Release Checklist](./dev-en/release-checklist.md) | Pre-release checks |
| [Performance Benchmarks](./dev-en/perf/structerror-allocation.md) | Allocation benchmarks |

---

*When docs and implementation conflict, `src/`, `tests/`, `examples/` are authoritative.*
