//! Performance measurement: heap allocations in `StructError` construction.
//!
//! Run with:
//!   cargo test --release --test perf_context_allocation -- --nocapture
//!
//! Repeat count is chosen so even a short loop runs long enough on modern
//! hardware to get a stable reading.

use std::time::Instant;

use std::io;

use orion_error::{
    reason::UvsReason, runtime::source::SourceFrame, runtime::ErrorMetadata, OperationContext,
    StructError,
};

const N: u64 = 500_000;
const M: u64 = 200_000; // for source-heavy cases
const S: u64 = 1_000_000; // for frame-only benchmarks

#[test]
fn perf_measure_allocations() {
    // warmup: let the allocator settle
    for _ in 0..100_000 {
        black_hole(StructError::from(UvsReason::validation_error()));
    }

    // 1. Bare creation — no detail, no context, no source
    bench("bare             ", || {
        StructError::from(UvsReason::validation_error())
    });

    // 2. With detail
    bench("with-detail      ", || {
        StructError::from(UvsReason::validation_error()).with_detail("port number out of range")
    });

    // 3. With detail + position
    bench("with-detail+pos  ", || {
        StructError::from(UvsReason::validation_error())
            .with_detail("port number out of range")
            .with_position("src/config.rs:42")
    });

    // 4. builder — same payload as above via builder API
    bench("builder          ", || {
        StructError::builder(UvsReason::validation_error())
            .detail("port number out of range")
            .position("src/config.rs:42")
            .finish()
    });

    // === Source-related cases ===

    // 5. With std source (io::Error, cheap Debug)
    bench_n("with-std-source  ", M, || {
        StructError::from(UvsReason::system_error()).with_source(io::Error::other("disk offline"))
    });

    // 6. With std source + long message (Debug cost visible)
    bench_n("with-std-verbose ", M, || {
        StructError::from(UvsReason::system_error()).with_source(io::Error::other("x".repeat(256)))
    });

    // 7. With struct source (expensive Debug — full context stack)
    bench_n("with-struct-src  ", M, || {
        let ctx = OperationContext::doing("parse config");
        let inner = StructError::from(UvsReason::validation_error())
            .with_detail("port number out of range")
            .with_position("src/config.rs:42")
            .with_context(ctx)
            .with_context(OperationContext::at("config.toml"));
        StructError::from(UvsReason::system_error()).with_source(inner)
    });

    // === SourceFrame clone benchmarks ===

    // 8a. SourceFrame clone — short strings only (typical case)
    let frame_short = SourceFrame {
        index: 0,
        message: "validation error".into(),
        display: None,
        debug: None,
        type_name: Some("std::io::Error".into()),
        error_code: Some(500),
        reason: Some("validation error".into()),
        path: Some("load config".into()),
        detail: Some("field `email` is required".into()),
        metadata: ErrorMetadata::new(),
        is_root_cause: true,
        context_fields: Vec::new(),
    };
    bench_n_src("frame-clone-short", S, frame_short);

    // 8b. SourceFrame clone — long strings (worst case)
    let frame_long = SourceFrame {
        index: 0,
        message: "x".repeat(128).into(),
        display: Some("x".repeat(256).into()),
        debug: None,
        type_name: Some("crate::very::long::module::path::StructError".into()),
        error_code: Some(500),
        reason: Some("x".repeat(128).into()),
        path: Some("start engine / load config / parse config / config.toml".into()),
        detail: Some("x".repeat(128).into()),
        metadata: ErrorMetadata::new(),
        is_root_cause: true,
        context_fields: Vec::new(),
    };
    bench_n_src("frame-clone-long ", S, frame_long);

    // 8. Deep struct source chain (3 layers, compounding Debug)
    bench_n("deep-struct-src  ", M, || {
        let leaf = StructError::from(UvsReason::validation_error())
            .with_detail("port number out of range");
        let mid = StructError::from(UvsReason::data_error())
            .with_detail("parse failed")
            .with_source(leaf);
        StructError::from(UvsReason::system_error()).with_source(mid)
    });
}

fn bench_n(name: &str, n: u64, f: impl Fn() -> StructError<UvsReason>) {
    let start = Instant::now();
    for _ in 0..n {
        black_hole(f());
    }
    let elapsed = start.elapsed();
    println!(
        "  {name}  {throughput:>8}  {ns_per:.1} ns/iter  {ms:>6} ms",
        throughput = throughput(n, elapsed),
        ns_per = nanos_per(n, elapsed),
        ms = elapsed.as_millis(),
    );
}

fn bench_n_src(name: &str, n: u64, frame: SourceFrame) {
    let start = Instant::now();
    for _ in 0..n {
        let mut cloned = frame.clone();
        cloned.index += 1;
        black_hole(cloned);
    }
    let elapsed = start.elapsed();
    println!(
        "  frame-{name}  {throughput:>8}  {ns_per:.1} ns/iter  {ms:>6} ms",
        throughput = throughput(n, elapsed),
        ns_per = nanos_per(n, elapsed),
        ms = elapsed.as_millis(),
    );
}

fn bench(name: &str, f: impl Fn() -> StructError<UvsReason>) {
    let start = Instant::now();
    for _ in 0..N {
        black_hole(f());
    }
    let elapsed = start.elapsed();
    println!(
        "  {name}  {throughput:>8}  {ns_per:.1} ns/iter  {ms:>6} ms",
        throughput = throughput(N, elapsed),
        ns_per = nanos_per(N, elapsed),
        ms = elapsed.as_millis(),
    );
}

#[inline(never)]
fn black_hole<T>(_x: T) {}

fn throughput(n: u64, d: std::time::Duration) -> String {
    let ns = d.as_nanos() as f64;
    let iters_per_ns = n as f64 / ns;
    let iters_per_s = iters_per_ns * 1_000_000_000.0;
    if iters_per_s > 1_000_000.0 {
        format!("{:.0} M/s", iters_per_s / 1_000_000.0)
    } else if iters_per_s > 1_000.0 {
        format!("{:.0} K/s", iters_per_s / 1_000.0)
    } else {
        format!("{:.0} /s", iters_per_s)
    }
}

fn nanos_per(n: u64, d: std::time::Duration) -> f64 {
    d.as_nanos() as f64 / n as f64
}
