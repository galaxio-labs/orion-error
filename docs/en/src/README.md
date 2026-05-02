# orion-error Documentation

> [简体中文](../zh/)

Suggested reading order: start with the user guide to learn concepts and usage, then check the developer guide for public API contracts and release details.

---

## User Guide

| Document | Description |
|----------|-------------|
| [Why orion-error](./user/why-orion-error.md) | Error governance motivation and examples |
| [Tutorial](./user/tutorial.md) | Getting started tutorial |
| [Protocol Contract](./user/protocol-contract.md) | Exposure projection contract |
| [Report / Exposure Boundary](./user/report-exposure-boundary.md) | Diagnostic vs exposure boundary |
| [Logging](./user/LOGGING.md) | Logging integration |
| [Comparison with thiserror](./user/thiserror-comparison.md) | Differences and coexistence |
| [Ecosystem Comparison](./user/ecosystem-comparison.md) | anyhow / thiserror / color-eyre / orion-error |
| [Design Constraints](./user/design-constraints.md) | Known design constraints |

## Developer Guide

| Document | Description |
|----------|-------------|
| [API Contract](./dev/api-contract.md) | Public API boundaries |
| [Compatibility Migration](./dev/compat-migration.md) | Migration from older APIs |
| [Public Surface Grading](./dev/public-surface-grading.md) | Layered export grading |
| [Release Checklist](./dev/release-checklist.md) | Pre-release checks |
| [Performance Benchmarks](./dev/perf/structerror-allocation.md) | Allocation benchmarks |

---

*When docs and implementation conflict, `src/`, `tests/`, `examples/` are authoritative.*
