# Source Debug 格式化性能影响

测试 eager `format!("{source:?}")` 在 `collect_source_frames` 中的实际开销及优化效果。

运行：`cargo test --release --test perf_context_allocation -- --nocapture`

## 结果

### Before：eager `debug: format!("{source:?}")`

| 场景 | 吞吐量 | ns/iter | 说明 |
|------|--------|---------|------|
| bare | 56 M/s | 18.0 | baseline |
| with-std-source | 2.5 M/s | 400.9 | + `io::Error` |
| with-std-verbose | 1.7 M/s | 581.0 | + 256-byte `io::Error` |
| with-struct-src | 458 K/s | 2184.8 | + StructError (2 contexts) |
| deep-struct-src | 420 K/s | 2381.9 | + 3 层 StructError 链 |

### After：lazy `debug: None`（优化后）

| 场景 | 吞吐量 | ns/iter | 提升 |
|------|--------|---------|------|
| bare | 58 M/s | 17.3 | +4% (noise) |
| with-std-source | **3.9 M/s** | **259.3** | **+55%** |
| with-std-verbose | **4.0 M/s** | **252.1** | **+130%** |
| with-struct-src | **849 K/s** | **1177.8** | **+86%** |
| deep-struct-src | **1.2 M/s** | **821.3** | **+190%** |

## 分析

- `with-std-source` 从 400.9 → 259.3 ns，`Debug` 格式化占 ~140ns
- `with-std-verbose` 从 581.0 → 252.1 ns，长消息的 Debug 开销被完全消除
- `with-struct-src` 从 2184.8 → 1177.8 ns（-46%），Debug 遍历 context 栈的开销消失
- `deep-struct-src` 从 2381.9 → 821.3 ns（-65%），最深层的帧直接拷贝已有帧，无额外格式化

## 优化方法

将 `SourceFrame.debug` 从 `String` 改为 `Option<String>`：

```rust
// Before
pub debug: String,
// 在 collect_source_frames 中：debug: format!("{source:?}"),

// After
pub debug: Option<String>,
// 在 collect_source_frames 中：debug: None,
```

Redaction 仍然支持 `debug` 字段——测试中显式设置了 `Some(...)` 的值会被正常处理。`None` 的帧在 redaction 中跳过。
