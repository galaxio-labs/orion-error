//! Demonstrates the 0.9 data/guard split for operation logging.
//!
//! Run with:
//! `cargo run --example autolog_guard --features log`
//! (or `--features tracing`).
//!
//! See `docs/*/src/user/autolog-guard.md` for the full usage guide.

use orion_error::{op_context, StructError, UnifiedReason};

fn main() {
    // Default to `info` so the `suc!` (info) and `cancel!` (warn) entries show up.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("--- 1. attaching data does not duplicate the log ---");
    attach_data_without_extra_log();

    println!("\n--- 2. failure-first scope with `?` ---");
    let _ = process_order(false); // success -> suc!
    let _ = process_order(true); // early return -> fail!

    println!("\n--- 3. explicit cancel ---");
    cancelled();
}

/// Attach a data copy of the operation to an error; only the guard logs.
fn attach_data_without_extra_log() {
    let guard = op_context!("renew")
        .with_auto_log()
        .with_field("gateway_id", "gw-1");

    // `&guard` copies pure data; it does not arm a second logger.
    let err = StructError::from(UnifiedReason::system_error()).with_context(&guard);
    println!("err carries the context as data only:\n{err}");
    drop(err); // no log here

    // The guard is still the single owner that writes the outcome log on drop.
    drop(guard);
}

/// Recommended fallible pattern: `scope()` defaults to failure, so `?` is safe.
fn process_order(fail: bool) -> Result<(), ()> {
    let mut guard = op_context!("process_order")
        .with_auto_log()
        .with_field("order_id", "order-123");

    let mut scope = guard.scope(); // default: failure
    work(fail)?; // early return skips mark_success -> fail! on drop
    scope.mark_success(); // success -> suc! on drop
    Ok(())
}

fn work(fail: bool) -> Result<(), ()> {
    if fail {
        Err(())
    } else {
        Ok(())
    }
}

/// Mark a guard as cancelled; it writes `cancel!` on drop.
fn cancelled() {
    let mut guard = op_context!("sync").with_auto_log();
    guard.cancel();
}
