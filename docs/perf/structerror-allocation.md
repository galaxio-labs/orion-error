# StructError 堆分配性能基线

硬件：Apple M4 (Mac mini, 2024)
系统：macOS 15, aarch64
Rust：stable 2025-04-30
运行：`cargo test --release --test perf_context_allocation -- --nocapture`

---

## 测试场景

每个场景重复 500,000 次，测量总耗时后计算均值和吞吐量。

| 场景 | 构造内容 |
|------|---------|
| `bare` | `StructError::from(UvsReason::validation_error())` |
| `with-detail` | 同上 + `.with_detail("port number out of range")` |
| `with-detail+pos` | 同上 + `.with_position("src/config.rs:42")` |
| `builder` | builder API 等同 with-detail+pos |

## 结果

| 场景 | 吞吐量 | ns/iter | 总耗时 |
|------|--------|---------|--------|
| bare | 28 M/s | **35.9** | 17 ms |
| with-detail | 19 M/s | 53.3 | 26 ms |
| with-detail+pos | 15 M/s | 64.6 | 32 ms |
| builder | 15 M/s | 65.1 | 32 ms |

## 分析

- `bare`（35.9 ns）是 baseline，主要开销来自 `Box::new` + `Arc::new(Vec::new())` + `StructErrorImpl` 的栈构造
- `with-detail` 增加一次 `String` 堆分配，耗时 +48%
- `with-detail+pos` 增加两次 `String` 堆分配，耗时 +80%
- builder 与链式 API 等价，无额外开销

## 目标

优化后预期 `bare` 提升约 30-40%（去掉空 `Arc::new(vec![])` 堆分配）。

---

*测试文件：`tests/perf_context_allocation.rs`*
