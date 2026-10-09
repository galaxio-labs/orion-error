//! Regression + behavior tests for <https://github.com/galaxio-labs/orion-error/issues/64>.
//!
//! Option A: `OperationContext` is pure, `Clone` data with no `Drop` side effect;
//! the drop-time log lives on the separate, non-`Clone` [`AutoLogGuard`].
//!
//! These tests inspect actual log records, so they target the `log` backend (the
//! `tracing` path is selected instead when that feature is on).
#![cfg(all(feature = "log", not(feature = "tracing")))]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use orion_error::runtime::AutoLogGuard;
use orion_error::{OperationContext, StructError, UnifiedReason};

static FAIL: AtomicUsize = AtomicUsize::new(0);
static SUC: AtomicUsize = AtomicUsize::new(0);
static CANCEL: AtomicUsize = AtomicUsize::new(0);

/// The logger and counters are process-global, so serialize the tests.
static SERIAL: Mutex<()> = Mutex::new(());
/// Every captured log line, in order.
static LOGS: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct CountingLogger;

impl log::Log for CountingLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let msg = record.args().to_string();
        if msg.starts_with("fail!") {
            FAIL.fetch_add(1, Ordering::SeqCst);
        } else if msg.starts_with("suc!") {
            SUC.fetch_add(1, Ordering::SeqCst);
        } else if msg.starts_with("cancel!") {
            CANCEL.fetch_add(1, Ordering::SeqCst);
        }
        LOGS.lock().unwrap_or_else(|p| p.into_inner()).push(msg);
    }

    fn flush(&self) {}
}

static LOGGER: CountingLogger = CountingLogger;

fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn reset() {
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(log::LevelFilter::Trace);
    FAIL.store(0, Ordering::SeqCst);
    SUC.store(0, Ordering::SeqCst);
    CANCEL.store(0, Ordering::SeqCst);
    LOGS.lock().unwrap_or_else(|p| p.into_inner()).clear();
}

fn logs() -> Vec<String> {
    LOGS.lock().unwrap_or_else(|p| p.into_inner()).clone()
}

fn counts() -> (usize, usize, usize) {
    (
        FAIL.load(Ordering::SeqCst),
        SUC.load(Ordering::SeqCst),
        CANCEL.load(Ordering::SeqCst),
    )
}

// --- helpers that mirror the recommended call patterns -----------------------

/// Failure-first pattern: `scope()` defaults to failure, so `?` is safe.
fn guarded_fallible(fail: bool) -> Result<(), ()> {
    let mut guard = OperationContext::doing("job").with_auto_log();
    let mut scope = guard.scope();
    work(fail)?;
    scope.mark_success();
    Ok(())
}

fn work(fail: bool) -> Result<(), ()> {
    if fail {
        Err(())
    } else {
        Ok(())
    }
}

// --- core regression scenarios ----------------------------------------------

#[test]
fn guard_logs_exactly_once_on_drop() {
    let _serial = serial();
    reset();
    {
        let _guard = OperationContext::doing("renew").with_auto_log();
    }
    assert_eq!(counts(), (1, 0, 0));
}

#[test]
fn attaching_guard_does_not_duplicate_or_defer_the_log() {
    let _serial = serial();
    reset();
    let mut guard = OperationContext::doing("renew").with_auto_log();

    // Attaching via `&guard` copies pure data; the guard keeps ownership.
    let err = StructError::from(UnifiedReason::system_error()).with_context(&guard);
    assert_eq!(counts(), (0, 0, 0), "attach must not log");

    // Dropping the error copy must not log either (no Drop on the data type).
    drop(err);
    assert_eq!(counts(), (0, 0, 0), "error drop must not log");

    // Only the live guard writes, exactly once.
    guard.mark_success();
    drop(guard);
    assert_eq!(counts(), (0, 1, 0));
}

#[test]
fn cloned_data_is_side_effect_free() {
    let _serial = serial();
    reset();
    let ctx = OperationContext::doing("renew").with_field("gateway_id", "gw-1");
    let data = ctx.clone();
    let err = StructError::from(UnifiedReason::system_error()).with_context(data);

    drop(err);
    drop(ctx);
    assert_eq!(counts(), (0, 0, 0));
}

#[test]
fn into_context_disarms_the_guard() {
    let _serial = serial();
    reset();
    let guard: AutoLogGuard = OperationContext::doing("renew").with_auto_log();
    let ctx = guard.into_context();
    drop(ctx);
    assert_eq!(counts(), (0, 0, 0));
}

// --- message content --------------------------------------------------------

#[test]
fn failure_log_carries_action_and_fields() {
    let _serial = serial();
    reset();
    drop(
        OperationContext::doing("renew")
            .with_auto_log()
            .with_field("gateway_id", "gw-1"),
    );

    let lines = logs();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("fail!"), "got: {}", lines[0]);
    assert!(lines[0].contains("doing=renew"), "got: {}", lines[0]);
    assert!(lines[0].contains("gateway_id"), "got: {}", lines[0]);
    assert!(lines[0].contains("gw-1"), "got: {}", lines[0]);
}

#[test]
fn with_mod_path_override_is_respected() {
    let _serial = serial();
    reset();
    let guard = OperationContext::doing("renew")
        .with_auto_log()
        .with_mod_path("custom::module");
    assert_eq!(guard.mod_path(), "custom::module");
    drop(guard);
    assert_eq!(counts(), (1, 0, 0));
}

// --- result transitions -----------------------------------------------------

#[test]
fn cancel_emits_cancel_once() {
    let _serial = serial();
    reset();
    let mut guard = OperationContext::doing("renew").with_auto_log();
    guard.cancel();
    drop(guard);
    assert_eq!(counts(), (0, 0, 1));
}

#[test]
fn success_then_failure_wins() {
    let _serial = serial();
    reset();
    let mut guard = OperationContext::doing("renew").with_auto_log();
    guard.mark_success();
    guard.mark_failure();
    drop(guard);
    assert_eq!(counts(), (1, 0, 0));
}

#[test]
fn scope_defaults_to_failure_for_early_return() {
    let _serial = serial();
    reset();
    let _ = guarded_fallible(true); // `?` returns early -> no mark_success
    assert_eq!(counts(), (1, 0, 0), "early return must log fail!");
}

#[test]
fn scope_success_path_logs_suc() {
    let _serial = serial();
    reset();
    let _ = guarded_fallible(false);
    assert_eq!(counts(), (0, 1, 0));
}

#[test]
fn scoped_success_with_mark_failure_logs_fail() {
    let _serial = serial();
    reset();
    {
        let mut guard = OperationContext::doing("renew").with_auto_log();
        {
            let mut scope = guard.scoped_success();
            scope.mark_failure();
        }
    }
    assert_eq!(counts(), (1, 0, 0));
}

// --- repeated / shared attach ----------------------------------------------

#[test]
fn multiple_attaches_never_duplicate_the_log() {
    let _serial = serial();
    reset();
    let guard = OperationContext::doing("renew").with_auto_log();
    let _e1 = StructError::from(UnifiedReason::system_error()).with_context(&guard);
    let _e2 = StructError::from(UnifiedReason::system_error()).with_context(&guard);
    assert_eq!(counts(), (0, 0, 0));
    drop(guard);
    assert_eq!(counts(), (1, 0, 0), "still exactly one log for the guard");
}
