//! Performance measurement: heap allocations in `StructError` construction.
//!
//! Run with:
//!   cargo test --release --test perf_context_allocation -- --nocapture
//!
//! Repeat count is chosen so even a short loop runs long enough on modern
//! hardware to get a stable reading.

use std::time::Instant;

use orion_error::{reason::UvsReason, StructError};

const N: u64 = 500_000;

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
        StructError::from(UvsReason::validation_error())
            .with_detail("port number out of range")
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
