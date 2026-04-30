use smol_str::SmolStr;

pub trait RedactPolicy {
    fn redact_key(&self, _key: &str) -> bool {
        false
    }

    fn redact_value(&self, _key: Option<&str>, value: &str) -> Option<String> {
        Some(value.to_string())
    }
}

fn root_cause_source_frame(source_frames: &[SourceFrame]) -> Option<&SourceFrame> {
    source_frames
        .iter()
        .find(|frame| frame.is_root_cause)
        .or_else(|| source_frames.last())
}

fn format_metadata_summary(metadata: &ErrorMetadata) -> String {
    metadata
        .iter()
        .map(|(key, value)| format!("{key}={}", format_metadata_value(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_metadata_value(value: &MetadataValue) -> String {
    match value {
        MetadataValue::String(value) => format!("{value:?}"),
        MetadataValue::Bool(value) => value.to_string(),
        MetadataValue::I64(value) => value.to_string(),
        MetadataValue::U64(value) => value.to_string(),
    }
}

fn redact_optional_text(
    key: Option<&str>,
    value: Option<&str>,
    policy: &impl RedactPolicy,
) -> Option<String> {
    value.and_then(|value| policy.redact_value(key, value))
}

fn redact_context(ctx: OperationContext, policy: &impl RedactPolicy) -> OperationContext {
    let mut redacted_items = Vec::with_capacity(ctx.context().items.len());
    for (key, value) in &ctx.context().items {
        let kept = if policy.redact_key(key) {
            policy
                .redact_value(Some(key.as_str()), value)
                .or_else(|| Some("<redacted>".to_string()))
        } else {
            policy.redact_value(Some(key.as_str()), value)
        };

        if let Some(value) = kept {
            redacted_items.push((key.clone(), value));
        }
    }

    let redacted_target = ctx.compat_target();
    let redacted_want = redact_optional_text(Some("want"), redacted_target.as_deref(), policy);
    let redacted_action = redact_optional_text(Some("action"), ctx.action().as_deref(), policy);
    let redacted_locator = redact_optional_text(Some("locator"), ctx.locator().as_deref(), policy);
    let redacted_path = ctx
        .path()
        .iter()
        .filter_map(|segment| redact_optional_text(Some("path"), Some(segment.as_str()), policy))
        .collect::<Vec<_>>();
    OperationContext::from_projection_parts(
        redacted_want,
        redacted_action,
        redacted_locator,
        redacted_path,
        redacted_items,
        redact_metadata(ctx.metadata(), policy),
        ctx.result().clone(),
    )
}

fn redact_metadata(metadata: &ErrorMetadata, policy: &impl RedactPolicy) -> ErrorMetadata {
    let mut redacted = ErrorMetadata::new();
    for (key, value) in metadata.iter() {
        match value {
            MetadataValue::String(value) => {
                if policy.redact_key(key) {
                    if let Some(value) = policy
                        .redact_value(Some(key.as_str()), value)
                        .or_else(|| Some("<redacted>".to_string()))
                    {
                        redacted.insert(key.clone(), value);
                    }
                } else if let Some(value) = policy.redact_value(Some(key.as_str()), value) {
                    redacted.insert(key.clone(), value);
                }
            }
            MetadataValue::Bool(value) => {
                if !policy.redact_key(key) {
                    redacted.insert(key.clone(), *value);
                }
            }
            MetadataValue::I64(value) => {
                if !policy.redact_key(key) {
                    redacted.insert(key.clone(), *value);
                }
            }
            MetadataValue::U64(value) => {
                if !policy.redact_key(key) {
                    redacted.insert(key.clone(), *value);
                }
            }
        }
    }
    redacted
}

fn redact_frame(mut frame: SourceFrame, policy: &impl RedactPolicy) -> SourceFrame {
    frame.message = SmolStr::from(redact_required_text(Some("source.message"), &frame.message, policy));
    frame.display = redact_optional_text(Some("source.display"), frame.display.as_deref(), policy)
        .map(SmolStr::from);
    if let Some(ref debug_str) = frame.debug {
        frame.debug = Some(SmolStr::from(redact_required_text(Some("source.debug"), debug_str, policy)));
    }
    frame.detail = redact_optional_text(Some("detail"), frame.detail.as_deref(), policy)
        .map(SmolStr::from);
    frame.reason = redact_optional_text(Some("source.reason"), frame.reason.as_deref(), policy)
        .map(SmolStr::from);
    frame.path = redact_optional_text(Some("path"), frame.path.as_deref(), policy)
        .map(SmolStr::from);
    frame.metadata = redact_metadata(&frame.metadata, policy);
    frame
}

fn redact_required_text(key: Option<&str>, value: &str, policy: &impl RedactPolicy) -> String {
    policy
        .redact_value(key, value)
        .unwrap_or_else(|| "<redacted>".to_string())
}
