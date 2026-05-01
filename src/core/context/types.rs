/// Outcome of an operation tracked by [`OperationContext`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OperationResult {
    /// The operation completed successfully.
    Suc,
    /// The operation failed (default).
    #[default]
    Fail,
    /// The operation was cancelled.
    Cancel,
}

// Use the compile-time module path as the default log target for readability.
const DEFAULT_MOD_PATH: &str = module_path!();

/// Expands `module_path!()` at the call site so log output automatically
/// reflects the correct module path.
#[macro_export]
macro_rules! op_context {
    ($target:expr) => {
        $crate::OperationContext::doing($target).with_mod_path(module_path!())
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OperationContext {
    context: CallContext,
    result: OperationResult,
    exit_log: bool,
    mod_path: String,
    #[cfg_attr(feature = "serde", serde(default))]
    action: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    locator: Option<String>,
    target: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    path: Vec<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "ErrorMetadata::is_empty")
    )]
    metadata: ErrorMetadata,
}
impl Default for OperationContext {
    fn default() -> Self {
        Self {
            context: CallContext::default(),
            action: None,
            locator: None,
            target: None,
            path: Vec::new(),
            result: OperationResult::Fail,
            exit_log: false,
            mod_path: DEFAULT_MOD_PATH.into(),
            metadata: ErrorMetadata::default(),
        }
    }
}
pub type WithContext = OperationContext;
impl From<CallContext> for OperationContext {
    fn from(value: CallContext) -> Self {
        OperationContext {
            context: value,
            result: OperationResult::Fail,
            action: None,
            locator: None,
            target: None,
            path: Vec::new(),
            exit_log: false,
            mod_path: DEFAULT_MOD_PATH.into(),
            metadata: ErrorMetadata::default(),
        }
    }
}

impl Drop for OperationContext {
    fn drop(&mut self) {
        if !self.exit_log {
            return;
        }

        #[cfg(feature = "tracing")]
        {
            let ctx = self.format_context();
            match self.result() {
                OperationResult::Suc => {
                    tracing::info!(
                        target: "domain",
                        mod_path = %self.mod_path,
                        "suc! {ctx}"
                    )
                }
                OperationResult::Fail => {
                    tracing::error!(
                        target: "domain",
                        mod_path = %self.mod_path,
                        "fail! {ctx}"
                    )
                }
                OperationResult::Cancel => {
                    tracing::warn!(
                        target: "domain",
                        mod_path = %self.mod_path,
                        "cancel! {ctx}"
                    )
                }
            }
        }

        #[cfg(all(feature = "log", not(feature = "tracing")))]
        {
            match self.result() {
                OperationResult::Suc => {
                    info!(target: self.mod_path.as_str(), "suc! {}", self.format_context());
                }
                OperationResult::Fail => {
                    error!(target: self.mod_path.as_str(), "fail! {}", self.format_context());
                }
                OperationResult::Cancel => {
                    warn!(target: self.mod_path.as_str(), "cancel! {}", self.format_context());
                }
            }
        }
    }
}

impl Display for OperationContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let target = self.compat_target();

        if let Some(action) = &self.action {
            writeln!(f, "doing: {action}")?;
        }
        if let Some(locator) = &self.locator {
            writeln!(f, "at: {locator}")?;
        }
        if let Some(target) = target.as_ref() {
            if self.action.as_deref() != Some(target.as_str()) {
                writeln!(f, "want: {target}")?;
            }
        }
        if let Some(path) = self.normalized_path_string() {
            if target.as_deref() != Some(path.as_str()) {
                writeln!(f, "path: {path}")?;
            }
        }
        for (i, (k, v)) in self.context().items.iter().enumerate() {
            writeln!(f, "{}. {k}: {v} ", i + 1)?;
        }
        Ok(())
    }
}
impl OperationContext {
    pub(crate) fn from_projection_parts(
        target: Option<String>,
        action: Option<String>,
        locator: Option<String>,
        path: Vec<String>,
        fields: Vec<(String, String)>,
        metadata: ErrorMetadata,
        result: OperationResult,
    ) -> Self {
        let mut ctx = Self {
            context: CallContext { items: fields },
            result,
            exit_log: false,
            mod_path: DEFAULT_MOD_PATH.into(),
            action,
            locator,
            target,
            path,
            metadata,
        };
        ctx.normalize_compat_target_storage();
        ctx
    }

    fn normalize_compat_target_storage(&mut self) {
        if self.target.as_deref() == self.action.as_deref() {
            self.target = None;
        }
    }

    fn push_path_segment(&mut self, segment: String) {
        if segment.is_empty() {
            return;
        }

        let locator_at_end = self
            .locator
            .as_ref()
            .is_some_and(|locator| self.path.last() == Some(locator));

        let insert_index = if locator_at_end {
            self.path.len().saturating_sub(1)
        } else {
            self.path.len()
        };
        let prev = insert_index
            .checked_sub(1)
            .and_then(|idx| self.path.get(idx));
        let current = self.path.get(insert_index);
        if prev != Some(&segment) && current != Some(&segment) {
            self.path.insert(insert_index, segment);
        }
    }

    pub(crate) fn compat_target(&self) -> Option<String> {
        self.target.clone().or_else(|| self.action.clone())
    }

    #[cfg(test)]
    pub(crate) fn from_target(target: String) -> Self {
        Self {
            action: None,
            locator: None,
            target: Some(target.clone()),
            path: Vec::new(),
            context: CallContext::default(),
            result: OperationResult::Fail,
            exit_log: false,
            mod_path: DEFAULT_MOD_PATH.into(),
            metadata: ErrorMetadata::default(),
        }
    }

    pub(crate) fn push_target_segment(&mut self, target: String) {
        if target.is_empty() {
            return;
        }

        if self.action.is_some() {
            self.push_path_segment(target);
            return;
        }

        if self.target.is_none() {
            self.target = Some(target);
            return;
        }

        let root = self.target.clone().expect("checked above");
        if root == target {
            return;
        }

        self.push_path_segment(target);
    }

    pub fn context(&self) -> &CallContext {
        &self.context
    }

    pub fn result(&self) -> &OperationResult {
        &self.result
    }

    pub fn exit_log(&self) -> &bool {
        &self.exit_log
    }

    pub fn mod_path(&self) -> &String {
        &self.mod_path
    }

    pub fn action(&self) -> &Option<String> {
        &self.action
    }

    pub fn locator(&self) -> &Option<String> {
        &self.locator
    }

    pub fn path(&self) -> &[String] {
        &self.path
    }

    pub fn metadata(&self) -> &ErrorMetadata {
        &self.metadata
    }

    pub fn new() -> Self {
        Self {
            target: None,
            action: None,
            locator: None,
            path: Vec::new(),
            context: CallContext::default(),
            result: OperationResult::Fail,
            exit_log: false,
            mod_path: DEFAULT_MOD_PATH.into(),
            metadata: ErrorMetadata::default(),
        }
    }
    pub fn doing<S: Into<String>>(action: S) -> Self {
        let action = action.into();
        Self {
            target: None,
            action: Some(action.clone()),
            locator: None,
            path: vec![action],
            context: CallContext::default(),
            result: OperationResult::Fail,
            exit_log: false,
            mod_path: DEFAULT_MOD_PATH.into(),
            metadata: ErrorMetadata::default(),
        }
    }
    pub fn at<S: Into<String>>(locator: S) -> Self {
        let locator = locator.into();
        Self {
            action: None,
            locator: Some(locator.clone()),
            target: None,
            path: vec![locator],
            context: CallContext::default(),
            result: OperationResult::Fail,
            exit_log: false,
            mod_path: DEFAULT_MOD_PATH.into(),
            metadata: ErrorMetadata::default(),
        }
    }
    pub fn with_auto_log(mut self) -> Self {
        self.exit_log = true;
        self
    }
    pub fn with_mod_path<S: Into<String>>(mut self, path: S) -> Self {
        self.mod_path = path.into();
        self
    }
    pub fn with_doing<S: Into<String>>(&mut self, action: S) {
        let action = action.into();
        if action.is_empty() {
            return;
        }
        if self.action.is_none() {
            self.action = Some(action.clone());
        }
        self.normalize_compat_target_storage();
        self.push_path_segment(action)
    }
    pub fn with_at<S: Into<String>>(&mut self, locator: S) {
        let locator = locator.into();
        if locator.is_empty() {
            return;
        }
        self.locator = Some(locator.clone());
        if self.path.last() != Some(&locator) {
            self.path.push(locator);
        }
    }

    pub(crate) fn path_string_with_segments(&self, path: &[String]) -> Option<String> {
        if path.is_empty() {
            None
        } else {
            Some(path.join(" / "))
        }
    }

    pub(crate) fn normalized_path_segments(&self) -> Vec<String> {
        let mut path = Vec::new();
        if let Some(action) = &self.action {
            path.push(action.clone());
        } else if let Some(target) = &self.target {
            path.push(target.clone());
        }

        for segment in &self.path {
            if path.first() == Some(segment) {
                continue;
            }
            if path.last() != Some(segment) {
                path.push(segment.clone());
            }
        }

        if let Some(locator) = &self.locator {
            if path.last() != Some(locator) {
                path.push(locator.clone());
            }
        }

        path
    }

    pub(crate) fn normalized_path_string(&self) -> Option<String> {
        self.path_string_with_segments(&self.normalized_path_segments())
    }

    pub(crate) fn into_at_context(mut self) -> Self {
        if self.locator.is_none() && self.compat_target().is_none() {
            if self.path.len() == 1 {
                self.locator = self.path.first().cloned();
            } else if self.context.items.len() == 1 {
                let (key, value) = &self.context.items[0];
                if key == "key" || key == "path" {
                    self.locator = Some(value.clone());
                }
            }
        }

        if let Some(locator) = &self.locator {
            if self.path.last() != Some(locator) {
                self.path.push(locator.clone());
            }
        }

        self
    }
    /// Alias: set the target resource/operation name. The first call
    /// establishes the root target; subsequent calls only append to the path.
    pub fn set_target<S: Into<String>>(&mut self, target: S) {
        self.push_target_segment(target.into())
    }

    pub fn path_string(&self) -> Option<String> {
        self.normalized_path_string()
    }

    /// Record a human-readable key-value pair into the context stack.
    ///
    /// The entry will appear in the error's `Display` output.
    /// For typed metadata hidden from console output, use [`record_meta()`] instead.
    ///
    /// Prefer [`with_field`](Self::with_field) for chained construction;
    /// `record_field` is for when you already have a mutable reference.
    pub fn record_field<K, V>(&mut self, key: K, val: V)
    where
        K: Into<String>,
        V: Display,
    {
        self.context.items.push((key.into(), val.to_string()));
    }

    pub fn record<K, V>(&mut self, key: K, val: V)
    where
        K: Into<String>,
        V: Display,
    {
        self.record_field(key, val);
    }

    /// Builder-pattern version of [`record_field`].
    pub fn with_field<K, V>(mut self, key: K, val: V) -> Self
    where
        K: Into<String>,
        V: Display,
    {
        self.record_field(key, val);
        self
    }

    /// Record typed metadata that is **not** included in Display output.
    ///
    /// Use this for structured fields intended for serialization, snapshots, or
    /// API responses. For user-visible context entries, use [`record_field()`].
    ///
    /// Prefer [`with_meta`](Self::with_meta) for chained construction;
    /// `record_meta` is for when you already have a mutable reference.
    pub fn record_meta<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<MetadataValue>,
    {
        self.metadata.insert(key, value);
    }

    /// Builder-pattern version of [`record_meta`].
    pub fn with_meta<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<MetadataValue>,
    {
        self.record_meta(key, value);
        self
    }

    pub fn mark_suc(&mut self) {
        self.result = OperationResult::Suc;
    }
    pub fn mark_cancel(&mut self) {
        self.result = OperationResult::Cancel;
    }

    /// Format context information for log output.
    #[cfg_attr(not(any(feature = "log", feature = "tracing")), allow(dead_code))]
    fn format_context(&self) -> String {
        let target = self.compat_target().unwrap_or_default();
        let path = self.normalized_path_string().unwrap_or_default();
        let action = self.action.clone().unwrap_or_default();
        let locator = self.locator.clone().unwrap_or_default();
        let mut parts = Vec::new();
        if !action.is_empty() {
            parts.push(format!("doing={action}"));
        } else if !target.is_empty() {
            parts.push(format!("want={target}"));
        }
        if !locator.is_empty() {
            parts.push(format!("at={locator}"));
        }
        if !path.is_empty() && path != target && path != locator {
            parts.push(format!("path={path}"));
        }
        let head = if parts.is_empty() {
            match (target.is_empty(), path.is_empty() || path == target) {
                (true, true) => String::new(),
                (false, true) => format!("want={target}"),
                (false, false) => format!("want={target} path={path}"),
                (true, false) => format!("path={path}"),
            }
        } else {
            parts.join(" ")
        };
        if self.context.items.is_empty() {
            return head;
        }
        if head.is_empty() {
            let body = self.context.to_string();
            body.strip_prefix('\n').unwrap_or(&body).to_string()
        } else {
            format!("{head}: {}", self.context)
        }
    }

    /// Create a scope guard, defaulting to failure state.
    /// Requires explicit `mark_success()`, `mark_cancel()`, or `mark_fail()`.
    pub fn scope(&mut self) -> OperationScope<'_> {
        OperationScope {
            ctx: self,
            mark_success: false,
        }
    }

    /// Create a scope guard that automatically marks success on drop.
    pub fn scoped_success(&mut self) -> OperationScope<'_> {
        OperationScope {
            ctx: self,
            mark_success: true,
        }
    }

    /// Record log information with the current context.
    ///
    /// Provides valuable context even when no error has occurred.
    /// NOTE: requires the `log` or `tracing` feature flag.
    #[cfg(feature = "tracing")]
    pub fn info<S: AsRef<str>>(&self, message: S) {
        tracing::info!(
            target: "domain",
            mod_path = %self.mod_path,
            "{}: {}",
            self.format_context(),
            message.as_ref()
        );
    }
    #[cfg(all(feature = "log", not(feature = "tracing")))]
    pub fn info<S: AsRef<str>>(&self, message: S) {
        info!(target: self.mod_path.as_str(), "{}: {}", self.format_context(), message.as_ref());
    }
    #[cfg(not(any(feature = "log", feature = "tracing")))]
    pub fn info<S: AsRef<str>>(&self, _message: S) {}

    #[cfg(feature = "tracing")]
    pub fn debug<S: AsRef<str>>(&self, message: S) {
        tracing::debug!(
            target: "domain",
            mod_path = %self.mod_path,
            "{}: {}",
            self.format_context(),
            message.as_ref()
        );
    }
    #[cfg(all(feature = "log", not(feature = "tracing")))]
    pub fn debug<S: AsRef<str>>(&self, message: S) {
        debug!( target: self.mod_path.as_str(), "{}: {}", self.format_context(), message.as_ref());
    }
    #[cfg(not(any(feature = "log", feature = "tracing")))]
    pub fn debug<S: AsRef<str>>(&self, _message: S) {}

    #[cfg(feature = "tracing")]
    pub fn warn<S: AsRef<str>>(&self, message: S) {
        tracing::warn!(
            target: "domain",
            mod_path = %self.mod_path,
            "{}: {}",
            self.format_context(),
            message.as_ref()
        );
    }
    #[cfg(all(feature = "log", not(feature = "tracing")))]
    pub fn warn<S: AsRef<str>>(&self, message: S) {
        warn!( target: self.mod_path.as_str(), "{}: {}", self.format_context(), message.as_ref());
    }
    #[cfg(not(any(feature = "log", feature = "tracing")))]
    pub fn warn<S: AsRef<str>>(&self, _message: S) {}

    #[cfg(feature = "tracing")]
    pub fn error<S: AsRef<str>>(&self, message: S) {
        tracing::error!(
            target: "domain",
            mod_path = %self.mod_path,
            "{}: {}",
            self.format_context(),
            message.as_ref()
        );
    }
    #[cfg(all(feature = "log", not(feature = "tracing")))]
    pub fn error<S: AsRef<str>>(&self, message: S) {
        error!(target: self.mod_path.as_str(), "{}: {}", self.format_context(), message.as_ref());
    }
    #[cfg(not(any(feature = "log", feature = "tracing")))]
    pub fn error<S: AsRef<str>>(&self, _message: S) {}

    #[cfg(feature = "tracing")]
    pub fn trace<S: AsRef<str>>(&self, message: S) {
        tracing::trace!(
            target: "domain",
            mod_path = %self.mod_path,
            "{}: {}",
            self.format_context(),
            message.as_ref()
        );
    }
    #[cfg(all(feature = "log", not(feature = "tracing")))]
    pub fn trace<S: AsRef<str>>(&self, message: S) {
        trace!( target: self.mod_path.as_str(), "{}: {}", self.format_context(), message.as_ref());
    }
    #[cfg(not(any(feature = "log", feature = "tracing")))]
    pub fn trace<S: AsRef<str>>(&self, _message: S) {}

    /// Alias for documentation consistency (calls the same-named method above).
    pub fn log_info<S: AsRef<str>>(&self, message: S) {
        self.info(message)
    }
    pub fn log_debug<S: AsRef<str>>(&self, message: S) {
        self.debug(message)
    }
    pub fn log_warn<S: AsRef<str>>(&self, message: S) {
        self.warn(message)
    }
    pub fn log_error<S: AsRef<str>>(&self, message: S) {
        self.error(message)
    }
    pub fn log_trace<S: AsRef<str>>(&self, message: S) {
        self.trace(message)
    }

}

/// Guard value for scoped [`OperationContext`] lifecycle management.
///
/// Created via [`OperationContext::scope`] or [`OperationContext::auto_scope`].
/// Automatically records the exit result when dropped.
pub struct OperationScope<'a> {
    ctx: &'a mut OperationContext,
    mark_success: bool,
}

impl<'a> OperationScope<'a> {
    /// Explicitly mark as success.
    pub fn mark_success(&mut self) {
        self.mark_success = true;
    }

    /// Keep the failure state (default behavior).
    pub fn mark_failure(&mut self) {
        self.mark_success = false;
    }

    /// Mark as cancelled and prevent success write.
    pub fn cancel(&mut self) {
        self.ctx.mark_cancel();
        self.mark_success = false;
    }
}

impl<'a> Deref for OperationScope<'a> {
    type Target = OperationContext;

    fn deref(&self) -> &Self::Target {
        self.ctx
    }
}

impl<'a> DerefMut for OperationScope<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ctx
    }
}

impl Drop for OperationScope<'_> {
    fn drop(&mut self) {
        if self.mark_success {
            self.ctx.mark_suc();
        }
    }
}
