//! Data-vs-guard semantics for Option A (issue #64), independent of the logging
//! backend, so these run under every feature combination (including
//! `--all-features`).
//!
//! They assert the *type-level* contract: `OperationContext` is pure `Clone`
//! data, and `AutoLogGuard` is an owning wrapper that exposes the same data
//! without duplicating it.

use orion_error::runtime::AutoLogGuard;
use orion_error::OperationContext;

fn sample() -> OperationContext {
    let mut ctx = OperationContext::doing("renew").with_field("gateway_id", "gw-1");
    ctx.with_at("us-east-1");
    ctx
}

#[test]
fn context_clone_is_equal_pure_data() {
    let ctx = sample();
    let copy = ctx.clone();
    assert_eq!(ctx, copy, "cloning OperationContext is a pure data copy");
    assert_eq!(copy.action().as_deref(), Some("renew"));
    assert_eq!(copy.locator().as_deref(), Some("us-east-1"));
    // Attaching the clone adds no side effect: it is just data.
    let _err = orion_error::StructError::from(orion_error::UnifiedReason::system_error())
        .with_context(copy);
}

#[test]
fn guard_deref_exposes_the_same_data() {
    let ctx = sample();
    let guard = ctx.clone().with_auto_log();
    assert_eq!(guard.action(), ctx.action());
    assert_eq!(guard.path(), ctx.path());
    assert_eq!(guard.locator(), ctx.locator());
}

#[test]
fn from_guard_ref_copies_data_without_consuming_guard() {
    let guard = sample().with_auto_log();
    // `&guard` converts to a data copy (same fields), guard stays alive/owner.
    let data: OperationContext = (&guard).into();
    assert_eq!(&data, &*guard);
    // The guard is still usable after the copy.
    assert_eq!(guard.action().as_deref(), Some("renew"));
}

#[test]
fn into_context_preserves_data() {
    let guard: AutoLogGuard = sample().with_auto_log();
    let data = guard.into_context();
    assert_eq!(data.action().as_deref(), Some("renew"));
    assert_eq!(data.locator().as_deref(), Some("us-east-1"));
    assert!(format!("{data}").contains("gateway_id"));
}

#[test]
fn guard_builder_methods_return_the_guard() {
    let guard: AutoLogGuard = OperationContext::doing("renew")
        .with_auto_log()
        .with_field("gateway_id", "gw-1")
        .with_mod_path("custom::module");
    assert_eq!(guard.mod_path().as_str(), "custom::module");
    assert!(format!("{guard}").contains("gateway_id"));
}

#[test]
fn guard_display_delegates_to_context() {
    let guard = sample().with_auto_log();
    let rendered = format!("{guard}");
    assert!(rendered.contains("doing: renew"), "got: {rendered}");
    assert!(rendered.contains("at: us-east-1"), "got: {rendered}");
}

#[test]
fn guard_marks_result_through_deref() {
    let mut guard = sample().with_auto_log();
    assert!(format!("{:?}", guard.result()).contains("Fail"));
    guard.mark_success();
    assert!(format!("{:?}", guard.result()).contains("Suc"));
    guard.cancel();
    assert!(format!("{:?}", guard.result()).contains("Cancel"));
}
