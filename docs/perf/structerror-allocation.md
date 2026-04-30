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

### Before：`context: Arc<Vec<OperationContext>>`

| 场景 | 吞吐量 | ns/iter | 总耗时 |
|------|--------|---------|--------|
| bare | 28 M/s | 35.9 | 17 ms |
| with-detail | 19 M/s | 53.3 | 26 ms |
| with-detail+pos | 15 M/s | 64.6 | 32 ms |
| builder | 15 M/s | 65.1 | 32 ms |

### After：`context: Option<Arc<Vec<OperationContext>>>`

| 场景 | 吞吐量 | ns/iter | 总耗时 | 提升 |
|------|--------|---------|--------|------|
| bare | **55 M/s** | **18.2** | 9 ms | **+97%** |
| with-detail | 27 M/s | 36.6 | 18 ms | +46% |
| with-detail+pos | 20 M/s | 48.9 | 24 ms | +32% |
| builder | 20 M/s | 48.8 | 24 ms | +33% |

## 优化方法

`StructErrorImpl` 中的 `context: Arc<Vec<OperationContext>>` → `context: Option<Arc<Vec<OperationContext>>>`。

空 context 时不再堆分配，仅在 `with_context()` 或 `ContextAdd::add_context()` 首次调用时懒初始化。

## 分析

- bare（18.2 ns）现为主要来自 `Box::new` + 栈构造
- with-detail 比 bare 多一次 `String` 堆分配（约 18 ns）
- with-detail+pos 比 bare 多两次 `String` 堆分配（约 30 ns）
- 预期符合：去掉一次空 Arc 堆分配 reduce ~18 ns

---

*测试文件：`tests/perf_context_allocation.rs`*
*优化改动：`src/core/error/carrier.rs` + `src/core/report/diagnostic.rs`*
