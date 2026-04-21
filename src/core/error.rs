use std::{error::Error as StdError, fmt::Display, ops::Deref, sync::Arc};

use crate::ErrorWith;

use super::{
    context::{CallContext, OperationContext},
    domain::DomainReason,
    metadata::ErrorMetadata,
    ContextAdd, ErrorCode,
};
#[macro_export]
macro_rules! location {
    () => {
        format!("{}:{}:{}", file!(), line!(), column!())
    };
}

pub trait StructErrorTrait<T: DomainReason> {
    fn get_reason(&self) -> &T;
    fn get_detail(&self) -> Option<&String>;
    fn get_target(&self) -> Option<String>;
}

impl<T: DomainReason + ErrorCode> ErrorCode for StructError<T> {
    fn error_code(&self) -> i32 {
        self.reason.error_code()
    }
}

type BoxedSource = Arc<dyn StdError + Send + Sync + 'static>;

fn is_struct_error_type_name(type_name: &str) -> bool {
    type_name.contains("StructError<")
}

fn assert_non_struct_source(type_name: &str, message: &str) {
    assert!(!is_struct_error_type_name(type_name), "{message}");
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SourceFrame {
    pub index: usize,
    /// Stable human-facing summary. For `StructError` sources this is the reason text,
    /// not the full multi-line display output.
    pub message: String,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub display: Option<String>,
    /// Raw `Debug` output for local diagnostics. Do not send this to production logs
    /// without redaction; it may include internal fields or sensitive values.
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    pub debug: String,
    /// Best-effort type name. Rust does not expose concrete type names for every
    /// `dyn Error` frame, so this is not a complete classification key.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub type_name: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub error_code: Option<i32>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub reason: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub want: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub path: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub detail: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "ErrorMetadata::is_empty")
    )]
    pub metadata: ErrorMetadata,
    pub is_root_cause: bool,
}

fn merged_context_metadata(contexts: &[OperationContext]) -> ErrorMetadata {
    let mut merged = ErrorMetadata::new();
    for ctx in contexts {
        merged.merge_missing(ctx.metadata());
    }
    merged
}

fn collect_source_frames(
    err: &(dyn StdError + 'static),
    root_type_name: Option<&'static str>,
) -> Vec<SourceFrame> {
    let mut frames = Vec::new();
    let mut cur = Some(err);
    let mut index = 0;

    while let Some(source) = cur {
        frames.push(SourceFrame {
            index,
            message: source.to_string(),
            display: None,
            debug: format!("{source:?}"),
            type_name: if index == 0 {
                root_type_name.map(str::to_string)
            } else {
                None
            },
            error_code: None,
            reason: None,
            want: None,
            path: None,
            detail: None,
            metadata: ErrorMetadata::default(),
            is_root_cause: false,
        });
        cur = source.source();
        index += 1;
    }

    if let Some(last) = frames.last_mut() {
        last.is_root_cause = true;
    }

    frames
}

fn collect_source_frames_from<E>(source: &E) -> Vec<SourceFrame>
where
    E: StdError + Send + Sync + 'static,
{
    collect_source_frames(source, Some(std::any::type_name::<E>()))
}

fn collect_struct_error_source_frames<R>(source: &StructError<R>) -> Vec<SourceFrame>
where
    R: DomainReason + ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
{
    let mut frames = Vec::with_capacity(source.source_frames().len() + 1);
    frames.push(SourceFrame {
        index: 0,
        message: source.reason().to_string(),
        display: Some(source.to_string()),
        debug: format!("{source:?}"),
        type_name: Some(std::any::type_name::<StructError<R>>().to_string()),
        error_code: Some(source.error_code()),
        reason: Some(source.reason().to_string()),
        want: source.target_main(),
        path: source.target_path(),
        detail: source.detail().clone(),
        metadata: source.context_metadata(),
        is_root_cause: source.source_frames().is_empty(),
    });

    frames.extend(source.source_frames().iter().cloned().map(|mut frame| {
        frame.index += 1;
        frame
    }));

    frames
}

/// Structured error type containing detailed error information
/// including error source, contextual data, and debugging information.
#[derive(Debug, Clone)]
pub struct StructError<T: DomainReason> {
    imp: Box<StructErrorImpl<T>>,
}

#[cfg(feature = "serde")]
impl<T: DomainReason> serde::Serialize for StructError<T>
where
    T: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let source_frames = self.source_frames();
        let source_chain = self.source_chain();
        let source_message = source_chain.first().cloned();
        let want = self.target_main();
        let path = self.target_path();

        let mut state = serializer.serialize_struct("StructError", 9)?;
        state.serialize_field("reason", &self.imp.reason)?;
        state.serialize_field("detail", &self.imp.detail)?;
        state.serialize_field("position", &self.imp.position)?;
        state.serialize_field("context", self.imp.context.as_ref())?;
        state.serialize_field("want", &want)?;
        state.serialize_field("path", &path)?;
        state.serialize_field("source_frames", &source_frames)?;
        state.serialize_field("source_message", &source_message)?;
        state.serialize_field("source_chain", &source_chain)?;
        state.end()
    }
}

impl<T: DomainReason> StructError<T> {
    pub fn imp(&self) -> &StructErrorImpl<T> {
        &self.imp
    }
}

impl<T: DomainReason> PartialEq for StructError<T> {
    fn eq(&self, other: &Self) -> bool {
        self.imp == other.imp
    }
}

impl<T: DomainReason> Deref for StructError<T> {
    type Target = StructErrorImpl<T>;

    fn deref(&self) -> &Self::Target {
        &self.imp
    }
}
impl<T: DomainReason> StructError<T> {
    pub fn new(
        reason: T,
        detail: Option<String>,
        position: Option<String>,
        context: Vec<OperationContext>,
    ) -> Self {
        Self::new_with_source(reason, detail, position, context, None, Vec::new())
    }

    fn new_with_source(
        reason: T,
        detail: Option<String>,
        position: Option<String>,
        context: Vec<OperationContext>,
        source: Option<BoxedSource>,
        source_frames: Vec<SourceFrame>,
    ) -> Self {
        StructError {
            imp: Box::new(StructErrorImpl {
                reason,
                detail,
                position,
                context: Arc::new(context),
                source,
                source_frames: Arc::new(source_frames),
            }),
        }
    }
}

impl<T> From<T> for StructError<T>
where
    T: DomainReason,
{
    fn from(value: T) -> Self {
        StructError::new(value, None, None, Vec::new())
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct StructErrorImpl<T: DomainReason> {
    reason: T,
    detail: Option<String>,
    position: Option<String>,
    context: Arc<Vec<OperationContext>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    source: Option<BoxedSource>,
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    source_frames: Arc<Vec<SourceFrame>>,
}

impl<T: DomainReason> PartialEq for StructErrorImpl<T> {
    fn eq(&self, other: &Self) -> bool {
        self.reason == other.reason
            && self.detail == other.detail
            && self.position == other.position
            && self.context == other.context
    }
}

impl<T: DomainReason> StructErrorImpl<T> {
    pub fn reason(&self) -> &T {
        &self.reason
    }

    pub fn detail(&self) -> &Option<String> {
        &self.detail
    }

    pub fn position(&self) -> &Option<String> {
        &self.position
    }

    pub fn context(&self) -> &Arc<Vec<OperationContext>> {
        &self.context
    }

    pub fn source_ref(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_deref().map(|e| e as _)
    }

    pub fn source_frames(&self) -> &[SourceFrame] {
        self.source_frames.as_ref()
    }
}

pub fn convert_error<R1, R2>(other: StructError<R1>) -> StructError<R2>
where
    R1: DomainReason,
    R2: DomainReason + From<R1>,
{
    StructError::new(
        other.imp.reason.into(),
        other.imp.detail,
        other.imp.position,
        Arc::try_unwrap(other.imp.context).unwrap_or_else(|arc| (*arc).clone()),
    )
    .with_boxed_source_parts(other.imp.source, other.imp.source_frames)
}

impl<T: DomainReason> StructError<T> {
    fn with_boxed_source_parts(
        mut self,
        source: Option<BoxedSource>,
        source_frames: Arc<Vec<SourceFrame>>,
    ) -> Self {
        self.imp.source = source;
        self.imp.source_frames = source_frames;
        self
    }

    #[must_use]
    /// Attach a non-structured source error.
    ///
    /// For `StructError<_>` sources, use `with_struct_source(...)` so metadata and
    /// structured source frames are preserved.
    pub fn with_std_source<E>(self, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        self.with_source(source)
    }

    #[must_use]
    pub fn with_source<E>(mut self, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        assert_non_struct_source(
            std::any::type_name::<E>(),
            "use with_struct_source(...) when attaching StructError sources",
        );
        self.imp.source_frames = Arc::new(collect_source_frames_from(&source));
        self.imp.source = Some(Arc::new(source));
        self
    }

    #[must_use]
    pub(crate) fn with_struct_error_source<R>(mut self, source: StructError<R>) -> Self
    where
        R: DomainReason + ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
    {
        self.imp.source_frames = Arc::new(collect_struct_error_source_frames(&source));
        self.imp.source = Some(Arc::new(source));
        self
    }

    #[must_use]
    pub fn with_struct_source<R>(self, source: StructError<R>) -> Self
    where
        R: DomainReason + ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
    {
        self.with_struct_error_source(source)
    }

    pub fn source_ref(&self) -> Option<&(dyn StdError + 'static)> {
        self.imp.source_ref()
    }

    pub fn root_cause(&self) -> Option<&(dyn StdError + 'static)> {
        let mut cur = self.source_ref()?;
        while let Some(next) = cur.source() {
            cur = next;
        }
        Some(cur)
    }

    pub fn source_frames(&self) -> &[SourceFrame] {
        self.imp.source_frames()
    }

    pub fn root_cause_frame(&self) -> Option<&SourceFrame> {
        self.source_frames().last()
    }

    pub fn context_metadata(&self) -> ErrorMetadata {
        merged_context_metadata(self.contexts())
    }

    pub fn source_chain(&self) -> Vec<String> {
        self.source_frames()
            .iter()
            .map(|frame| frame.message.clone())
            .collect()
    }

    pub fn display_chain(&self) -> String
    where
        T: ErrorCode + std::fmt::Debug + Display + 'static,
    {
        let mut out = format!("{self}");
        let chain = self.source_chain();
        if !chain.is_empty() {
            out.push_str("\nCaused by:");
            for (idx, msg) in chain.iter().enumerate() {
                let mut lines = msg.lines();
                if let Some(first) = lines.next() {
                    out.push_str(&format!("\n  {idx}: {first}"));
                    for line in lines {
                        out.push_str(&format!("\n     {line}"));
                    }
                }
            }
        }
        out
    }

    pub fn render(&self, mode: super::report::RenderMode) -> String {
        self.report().render(mode)
    }

    pub fn render_redacted(
        &self,
        mode: super::report::RenderMode,
        policy: &impl super::report::RedactPolicy,
    ) -> String {
        self.report().render_redacted(mode, policy)
    }
}

impl<T> StdError for StructError<T>
where
    T: DomainReason + ErrorCode + std::fmt::Debug + Display + 'static,
{
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source_ref()
    }
}

impl<T: DomainReason> StructError<T> {
    pub fn builder(reason: T) -> StructErrorBuilder<T> {
        StructErrorBuilder {
            reason,
            detail: None,
            position: None,
            contexts: Vec::new(),
            source: None,
            source_frames: Vec::new(),
        }
    }

    /// 使用示例
    ///self.with_position(location!());
    #[must_use]
    pub fn with_position(mut self, position: impl Into<String>) -> Self {
        self.imp.position = Some(position.into());
        self
    }
    #[must_use]
    pub fn with_context(mut self, context: CallContext) -> Self {
        Arc::make_mut(&mut self.imp.context).push(OperationContext::from(context));
        self
    }

    pub fn contexts(&self) -> &[OperationContext] {
        self.imp.context.as_ref()
    }

    // 提供修改方法
    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.imp.detail = Some(detail.into());
        self
    }
    pub fn err<V>(self) -> Result<V, Self> {
        Err(self)
    }
    pub fn target_main(&self) -> Option<String> {
        self.context
            .iter()
            .rev()
            .find_map(|ctx| ctx.target().clone())
    }

    /// Compatibility alias for `target_main()`.
    ///
    /// Prefer `target_main()` in new code when pairing it with `target_path()`.
    pub fn target(&self) -> Option<String> {
        self.target_main()
    }

    pub fn path_segments(&self) -> Vec<String> {
        let mut path = Vec::new();
        for ctx in self.context.iter().rev() {
            let segments = if !ctx.path().is_empty() {
                ctx.path().to_vec()
            } else if let Some(target) = ctx.target().clone() {
                vec![target]
            } else {
                Vec::new()
            };

            for segment in segments {
                if path.last() != Some(&segment) {
                    path.push(segment);
                }
            }
        }
        path
    }

    pub fn target_path(&self) -> Option<String> {
        let segments = self.path_segments();
        if segments.is_empty() {
            None
        } else {
            Some(segments.join(" / "))
        }
    }
}

impl<T: DomainReason> StructErrorTrait<T> for StructError<T> {
    fn get_reason(&self) -> &T {
        &self.reason
    }

    fn get_detail(&self) -> Option<&String> {
        self.detail.as_ref()
    }

    fn get_target(&self) -> Option<String> {
        self.target()
    }
}

/*
impl<S1: Into<String>, S2: Into<String>, T: DomainReason> ContextAdd<(S1, S2)> for StructError<T> {
    fn add_context(&mut self, val: (S1, S2)) {
        self.imp.context.items.push((val.0.into(), val.1.into()));
    }
}
*/

impl<T: DomainReason> ContextAdd<&OperationContext> for StructError<T> {
    fn add_context(&mut self, ctx: &OperationContext) {
        Arc::make_mut(&mut self.imp.context).push(ctx.clone());
    }
}
impl<T: DomainReason> ContextAdd<OperationContext> for StructError<T> {
    fn add_context(&mut self, ctx: OperationContext) {
        Arc::make_mut(&mut self.imp.context).push(ctx);
    }
}

impl<T: std::fmt::Display + DomainReason + ErrorCode> Display for StructError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 核心错误信息
        write!(f, "[{}] {reason}", self.error_code(), reason = self.reason)?;

        // 位置信息优先显示
        if let Some(pos) = &self.position {
            write!(f, "\n  -> At: {pos}")?;
        }

        // 目标资源信息
        let want = self.target_main();
        if let Some(want) = &want {
            write!(f, "\n  -> Want: {want}")?;
        }

        if let Some(path) = self.target_path() {
            if want.as_deref() != Some(path.as_str()) {
                write!(f, "\n  -> Path: {path}")?;
            }
        }

        // 技术细节
        if let Some(detail) = &self.detail {
            write!(f, "\n  -> Details: {detail}")?;
        }

        if let Some(source) = self.source_ref() {
            write!(f, "\n  -> Source: {source}")?;
        }

        // 上下文信息
        if !self.context.is_empty() {
            writeln!(f, "\n  -> Context stack:")?;

            for (i, c) in self.context.iter().enumerate() {
                writeln!(f, "context {i}: ")?;
                writeln!(f, "{c}")?;
            }
        }

        Ok(())
    }
}

pub struct StructErrorBuilder<T: DomainReason> {
    reason: T,
    detail: Option<String>,
    position: Option<String>,
    contexts: Vec<OperationContext>,
    source: Option<BoxedSource>,
    source_frames: Vec<SourceFrame>,
}

impl<T: DomainReason> StructErrorBuilder<T> {
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn position(mut self, position: impl Into<String>) -> Self {
        self.position = Some(position.into());
        self
    }

    pub fn context(mut self, ctx: OperationContext) -> Self {
        self.contexts.push(ctx);
        self
    }

    pub fn context_ref(mut self, ctx: &OperationContext) -> Self {
        self.contexts.push(ctx.clone());
        self
    }

    /// Attach a non-structured source error.
    ///
    /// For `StructError<_>` sources, use `source_struct(...)` so metadata and
    /// structured source frames are preserved.
    pub fn source_std<E>(self, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        self.source(source)
    }

    pub fn source<E>(mut self, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        assert_non_struct_source(
            std::any::type_name::<E>(),
            "use source_struct(...) when attaching StructError sources",
        );
        self.source_frames = collect_source_frames_from(&source);
        self.source = Some(Arc::new(source));
        self
    }

    pub fn source_struct<R>(mut self, source: StructError<R>) -> Self
    where
        R: DomainReason + ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
    {
        self.source_frames = collect_struct_error_source_frames(&source);
        self.source = Some(Arc::new(source));
        self
    }

    pub fn finish(self) -> StructError<T> {
        StructError::new_with_source(
            self.reason,
            self.detail,
            self.position,
            self.contexts,
            self.source,
            self.source_frames,
        )
    }
}

impl<T: DomainReason> ErrorWith for StructError<T> {
    fn want<S: Into<String>>(mut self, desc: S) -> Self {
        let desc = desc.into();
        let ctx_stack = Arc::make_mut(&mut self.imp.context);
        if ctx_stack.is_empty() {
            ctx_stack.push(OperationContext::want(desc));
        } else if let Some(x) = ctx_stack.last_mut() {
            x.with_want(desc);
        }
        self
    }
    fn position<S: Into<String>>(mut self, pos: S) -> Self {
        self.imp.position = Some(pos.into());
        self
    }

    fn with<C: Into<OperationContext>>(mut self, ctx: C) -> Self {
        let ctx = ctx.into();
        self.add_context(ctx);
        self
    }
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use std::{error::Error as StdError, fmt};

    use crate::{ContextRecord, UvsReason};

    use super::*;
    use derive_more::From;
    use thiserror::Error;

    // Define a simple DomainReason for testing
    #[derive(Debug, Clone, PartialEq, Error, From)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize))]
    enum TestDomainReason {
        #[error("test error")]
        TestError,
        #[error("{0}")]
        Uvs(UvsReason),
    }

    impl ErrorCode for TestDomainReason {
        fn error_code(&self) -> i32 {
            match self {
                TestDomainReason::TestError => 1001,
                TestDomainReason::Uvs(uvs_reason) => uvs_reason.error_code(),
            }
        }
    }

    #[derive(Debug)]
    struct InnerError;

    impl fmt::Display for InnerError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "inner source")
        }
    }

    impl StdError for InnerError {}

    #[derive(Debug)]
    struct OuterError {
        source: InnerError,
    }

    impl fmt::Display for OuterError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "outer source")
        }
    }

    impl StdError for OuterError {
        fn source(&self) -> Option<&(dyn StdError + 'static)> {
            Some(&self.source)
        }
    }

    #[test]
    fn test_struct_error_serialization() {
        // Create a context
        let mut context = CallContext::default();
        context
            .items
            .push(("key1".to_string(), "value1".to_string()));
        context
            .items
            .push(("key2".to_string(), "value2".to_string()));

        // Create a StructError
        let error = StructError::new(
            TestDomainReason::TestError,
            Some("Detailed error description".to_string()),
            Some("file.rs:10:5".to_string()),
            vec![OperationContext::from(context)],
        );

        // Serialize to JSON
        let json_value = serde_json::to_value(&error).unwrap();
        println!("{json_value:#}");
        assert!(json_value.get("reason").is_some());
        assert!(json_value.get("detail").is_some());
        assert!(json_value.get("position").is_some());
        assert!(json_value.get("context").is_some());
        assert_eq!(json_value.get("want"), Some(&serde_json::Value::Null));
        assert_eq!(json_value.get("path"), Some(&serde_json::Value::Null));
        assert_eq!(
            json_value.get("source_frames"),
            Some(&serde_json::Value::Array(Vec::new()))
        );
        assert_eq!(
            json_value.get("source_message"),
            Some(&serde_json::Value::Null)
        );
        assert_eq!(
            json_value.get("source_chain"),
            Some(&serde_json::Value::Array(Vec::new()))
        );
    }

    #[test]
    fn test_struct_error_source_tracking() {
        let error = StructError::builder(TestDomainReason::TestError)
            .detail("high-level detail")
            .source(OuterError { source: InnerError })
            .finish();

        assert_eq!(error.source_ref().unwrap().to_string(), "outer source");
        assert_eq!(error.root_cause().unwrap().to_string(), "inner source");
        assert_eq!(error.root_cause_frame().unwrap().message, "inner source");

        let display_output = format!("{error}");
        assert!(display_output.contains("-> Source: outer source"));
    }

    #[test]
    fn test_struct_error_uses_outer_want_and_full_path() {
        let mut outer = OperationContext::want("place_order");
        outer.record("order_id", "42");

        let error = StructError::from(TestDomainReason::TestError)
            .want("read_order_payload")
            .want("parse_order")
            .with(outer);

        assert_eq!(error.target_main().as_deref(), Some("place_order"));
        assert_eq!(error.target().as_deref(), Some("place_order"));
        assert_eq!(
            error.target_path().as_deref(),
            Some("place_order / read_order_payload / parse_order")
        );
        assert_eq!(
            error.path_segments(),
            vec![
                "place_order".to_string(),
                "read_order_payload".to_string(),
                "parse_order".to_string()
            ]
        );

        let display_output = format!("{error}");
        assert!(display_output.contains("-> Want: place_order"));
        assert!(display_output.contains("-> Path: place_order / read_order_payload / parse_order"));
    }

    #[test]
    fn test_struct_error_display_chain() {
        let error = StructError::builder(TestDomainReason::TestError)
            .detail("high-level detail")
            .source(OuterError { source: InnerError })
            .finish();

        assert_eq!(
            error.source_chain(),
            vec!["outer source".to_string(), "inner source".to_string()]
        );
        assert_eq!(error.source_frames().len(), 2);
        assert_eq!(error.source_frames()[0].index, 0);
        assert_eq!(error.source_frames()[0].message, "outer source");
        assert_eq!(error.source_frames()[1].index, 1);
        assert_eq!(error.source_frames()[1].message, "inner source");
        assert!(error.source_frames()[1].is_root_cause);
        assert_eq!(
            error.source_frames()[0].type_name.as_deref(),
            Some(concat!(module_path!(), "::OuterError"))
        );
        assert_eq!(error.source_frames()[1].type_name, None);

        let display_chain = error.display_chain();
        assert!(display_chain.contains("Caused by:"));
        assert!(display_chain.contains("0: outer source"));
        assert!(display_chain.contains("1: inner source"));
    }

    #[test]
    fn test_struct_error_serialization_includes_source_summary() {
        let error = StructError::builder(TestDomainReason::TestError)
            .detail("high-level detail")
            .source(OuterError { source: InnerError })
            .finish();

        let json_value = serde_json::to_value(&error).unwrap();
        assert_eq!(
            json_value.get("source_message"),
            Some(&serde_json::Value::String("outer source".to_string()))
        );
        assert_eq!(
            json_value.get("source_chain"),
            Some(&serde_json::json!(["outer source", "inner source"]))
        );
        assert_eq!(
            json_value.get("source_frames"),
            Some(&serde_json::json!([
                {
                    "index": 0,
                    "message": "outer source",
                    "type_name": concat!(module_path!(), "::OuterError"),
                    "is_root_cause": false
                },
                {
                    "index": 1,
                    "message": "inner source",
                    "is_root_cause": true
                }
            ]))
        );
        assert!(!json_value["source_frames"][0]
            .as_object()
            .unwrap()
            .contains_key("debug"));
    }

    #[test]
    fn test_struct_error_serialization_includes_want_and_path() {
        let mut outer = OperationContext::want("place_order");
        outer.record("order_id", "42");

        let error = StructError::from(TestDomainReason::TestError)
            .want("read_order_payload")
            .with(outer);

        let json_value = serde_json::to_value(&error).unwrap();
        assert_eq!(
            json_value.get("want"),
            Some(&serde_json::Value::String("place_order".to_string()))
        );
        assert_eq!(
            json_value.get("path"),
            Some(&serde_json::Value::String(
                "place_order / read_order_payload".to_string()
            ))
        );
    }

    #[test]
    fn test_struct_error_context_metadata_prefers_inner_context() {
        let inner = OperationContext::want("load sink defaults")
            .with_meta("config.kind", "sink_defaults")
            .with_meta("file.path", "/tmp/defaults.toml");
        let outer = OperationContext::want("load infra sink routes")
            .with_meta("config.kind", "sink_route")
            .with_meta("config.group", "infra");

        let error = StructError::from(TestDomainReason::TestError)
            .with(inner)
            .with(outer);

        let metadata = error.context_metadata();
        assert_eq!(metadata.get_str("config.kind"), Some("sink_defaults"));
        assert_eq!(metadata.get_str("file.path"), Some("/tmp/defaults.toml"));
        assert_eq!(metadata.get_str("config.group"), Some("infra"));
    }

    #[test]
    fn test_with_struct_source_preserves_source_context_metadata() {
        let source = StructError::from(TestDomainReason::TestError).with(
            OperationContext::want("load sink defaults").with_meta("config.kind", "sink_defaults"),
        );

        let error = StructError::from(TestDomainReason::Uvs(UvsReason::system_error()))
            .with_struct_source(source);

        assert_eq!(
            error.source_frames()[0].metadata.get_str("config.kind"),
            Some("sink_defaults")
        );
    }

    #[test]
    fn test_builder_source_struct_preserves_source_context_metadata() {
        let source = StructError::from(TestDomainReason::TestError).with(
            OperationContext::want("load sink defaults").with_meta("config.kind", "sink_defaults"),
        );

        let error = StructError::builder(TestDomainReason::Uvs(UvsReason::system_error()))
            .source_struct(source)
            .finish();

        assert_eq!(
            error.source_frames()[0].metadata.get_str("config.kind"),
            Some("sink_defaults")
        );
    }

    #[test]
    fn test_with_struct_source_keeps_nested_source_frame_metadata() {
        let leaf = StructError::from(TestDomainReason::TestError).with(
            OperationContext::want("parse route")
                .with_meta("config.kind", "sink_route")
                .with_meta("config.group", "infra"),
        );
        let middle = StructError::from(TestDomainReason::Uvs(UvsReason::validation_error()))
            .with_struct_source(leaf);

        let error = StructError::from(TestDomainReason::Uvs(UvsReason::system_error()))
            .with_struct_source(middle);

        assert_eq!(
            error.source_frames()[1].metadata.get_str("config.kind"),
            Some("sink_route")
        );
        assert_eq!(
            error.source_frames()[1].metadata.get_str("config.group"),
            Some("infra")
        );
    }

    #[test]
    fn test_root_and_source_metadata_can_be_read_separately() {
        let source = StructError::from(TestDomainReason::TestError).with(
            OperationContext::want("load sink defaults").with_meta("config.kind", "sink_defaults"),
        );

        let error = StructError::from(TestDomainReason::Uvs(UvsReason::system_error()))
            .with(OperationContext::want("start engine").with_meta("component.name", "engine"))
            .with_struct_source(source);

        assert_eq!(
            error.context_metadata().get_str("component.name"),
            Some("engine")
        );
        assert_eq!(
            error.source_frames()[0].metadata.get_str("config.kind"),
            Some("sink_defaults")
        );
    }

    #[test]
    fn test_display_does_not_include_metadata() {
        let error = StructError::from(TestDomainReason::TestError).with(
            OperationContext::want("load sink defaults")
                .with_meta("config.kind", "sink_defaults")
                .with_meta("config.group", "infra"),
        );

        let display_output = format!("{error}");
        assert!(!display_output.contains("config.kind"));
        assert!(!display_output.contains("sink_defaults"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_source_frame_serialization_skips_empty_metadata() {
        let frame = SourceFrame {
            index: 0,
            message: "message".to_string(),
            display: None,
            debug: "debug".to_string(),
            type_name: None,
            error_code: None,
            reason: None,
            want: None,
            path: None,
            detail: None,
            metadata: ErrorMetadata::default(),
            is_root_cause: true,
        };

        let json_value = serde_json::to_value(&frame).unwrap();
        assert!(!json_value
            .as_object()
            .expect("object")
            .contains_key("metadata"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_source_frame_serialization_includes_metadata() {
        let error = StructError::from(TestDomainReason::TestError).with(
            OperationContext::want("load sink defaults")
                .with_meta("config.kind", "sink_defaults")
                .with_meta("parse.line", 1u32),
        );
        let wrapped = StructError::from(TestDomainReason::Uvs(UvsReason::system_error()))
            .with_struct_source(error);

        let json_value = serde_json::to_value(&wrapped).unwrap();
        assert_eq!(
            json_value["source_frames"][0]["metadata"]["config.kind"],
            serde_json::Value::String("sink_defaults".to_string())
        );
        assert_eq!(
            json_value["source_frames"][0]["metadata"]["parse.line"],
            serde_json::json!(1)
        );
    }

    #[test]
    fn test_with_source_debug_asserts_for_struct_error() {
        let source = StructError::from(TestDomainReason::TestError);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            StructError::from(TestDomainReason::Uvs(UvsReason::system_error())).with_source(source)
        }));

        assert!(result.is_err());
    }

    #[test]
    fn test_builder_source_debug_asserts_for_struct_error() {
        let source = StructError::from(TestDomainReason::TestError);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            StructError::builder(TestDomainReason::Uvs(UvsReason::system_error()))
                .source(source)
                .finish()
        }));

        assert!(result.is_err());
    }
}
