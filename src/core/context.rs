#[cfg(all(feature = "log", not(feature = "tracing")))]
use log::{debug, error, info, trace, warn};
use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use super::metadata::{ErrorMetadata, MetadataValue};
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OperationResult {
    Suc,
    #[default]
    Fail,
    Cancel,
}

// 使用编译期模块路径作为默认日志 target，以提升可读性
const DEFAULT_MOD_PATH: &str = module_path!();

/// 在调用处展开 `module_path!()`，便于自动日志输出正确的模块路径。
#[macro_export]
macro_rules! op_context {
    ($target:expr) => {
        $crate::OperationContext::doing($target).with_mod_path(module_path!())
    };
}

#[derive(Debug, Clone, PartialEq)]
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
        if let Some(action) = &self.action {
            writeln!(f, "doing: {action}")?;
        }
        if let Some(locator) = &self.locator {
            writeln!(f, "at: {locator}")?;
        }
        if let Some(target) = &self.target {
            if self.action.as_deref() != Some(target.as_str()) {
                writeln!(f, "want: {target}")?;
            }
        }
        if let Some(path) = self.normalized_path_string() {
            if self.target.as_deref() != Some(path.as_str()) {
                writeln!(f, "path: {path}")?;
            }
        }
        for (i, (k, v)) in self.context().items.iter().enumerate() {
            writeln!(f, "{}. {k}: {v} ", i + 1)?;
        }
        Ok(())
    }
}
pub trait ContextRecord<S1, S2> {
    fn record_field(&mut self, key: S1, val: S2);

    fn record(&mut self, key: S1, val: S2)
    where
        Self: Sized,
    {
        self.record_field(key, val);
    }
}

impl<S1> ContextRecord<S1, String> for OperationContext
where
    S1: Into<String>,
{
    fn record_field(&mut self, key: S1, val: String) {
        self.context.items.push((key.into(), val));
    }
}

impl<S1> ContextRecord<S1, &str> for OperationContext
where
    S1: Into<String>,
{
    fn record_field(&mut self, key: S1, val: &str) {
        self.context.items.push((key.into(), val.into()));
    }
}

// Wrapper type for path values to avoid conflicts

impl<S1> ContextRecord<S1, &PathBuf> for OperationContext
where
    S1: Into<String>,
{
    fn record_field(&mut self, key: S1, val: &PathBuf) {
        self.context
            .items
            .push((key.into(), format!("{}", val.display())));
    }
}
impl<S1> ContextRecord<S1, &Path> for OperationContext
where
    S1: Into<String>,
{
    fn record_field(&mut self, key: S1, val: &Path) {
        self.context
            .items
            .push((key.into(), format!("{}", val.display())));
    }
}

impl OperationContext {
    pub(crate) fn from_target(target: String) -> Self {
        Self {
            action: None,
            locator: None,
            target: Some(target.clone()),
            path: vec![target],
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

        let locator_at_end = self
            .locator
            .as_ref()
            .is_some_and(|locator| self.path.last() == Some(locator));

        if self.target.is_none() {
            self.target = Some(target.clone());
            if self.path.is_empty() {
                self.path.push(target);
                return;
            }
            if self.path.first() != Some(&target) {
                self.path.insert(0, target);
            }
        } else {
            let root = self.target.clone().expect("checked above");
            if self.path.is_empty() {
                self.path.push(root.clone());
            }
            if self.path.first() != Some(&root) {
                self.path.insert(0, root);
            }

            let insert_index = if locator_at_end {
                self.path.len().saturating_sub(1)
            } else {
                self.path.len()
            };
            let prev = insert_index
                .checked_sub(1)
                .and_then(|idx| self.path.get(idx));
            let current = self.path.get(insert_index);
            if prev != Some(&target) && current != Some(&target) {
                self.path.insert(insert_index, target);
            }
        }
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

    pub fn target(&self) -> &Option<String> {
        &self.target
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
    #[deprecated(
        since = "0.7.0",
        note = "use doing(...) for action contexts; use at(...) for locator/resource contexts"
    )]
    pub fn want<S: Into<String>>(target: S) -> Self {
        Self::from_target(target.into())
    }
    pub fn doing<S: Into<String>>(action: S) -> Self {
        let action = action.into();
        let mut ctx = Self::from_target(action.clone());
        ctx.action = Some(action);
        ctx
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
    #[deprecated(since = "0.5.4", note = "use with_auto_log")]
    pub fn with_exit_log(mut self) -> Self {
        self.exit_log = true;
        self
    }
    pub fn with_auto_log(mut self) -> Self {
        self.exit_log = true;
        self
    }
    pub fn with_mod_path<S: Into<String>>(mut self, path: S) -> Self {
        self.mod_path = path.into();
        self
    }
    #[deprecated(since = "0.5.4", note = "use record")]
    pub fn with<S1: Into<String>, S2: Into<String>>(&mut self, key: S1, val: S2) {
        self.context.items.push((key.into(), val.into()));
    }

    #[deprecated(since = "0.5.4", note = "use record")]
    pub fn with_path<S1: Into<String>, S2: Into<PathBuf>>(&mut self, key: S1, val: S2) {
        self.context
            .items
            .push((key.into(), format!("{}", val.into().display())));
    }

    #[deprecated(
        since = "0.7.0",
        note = "use with_doing(...) for action path segments; use with_at(...) for locator segments"
    )]
    pub fn with_want<S: Into<String>>(&mut self, target: S) {
        self.push_target_segment(target.into());
    }
    pub fn with_doing<S: Into<String>>(&mut self, action: S) {
        let action = action.into();
        if action.is_empty() {
            return;
        }
        if self.action.is_none() {
            self.action = Some(action.clone());
        }
        self.push_target_segment(action)
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

        match (&self.action, &self.target) {
            (Some(action), Some(target)) => {
                path.push(target.clone());
                for segment in self.path.iter().skip(1) {
                    if path.last() != Some(segment) {
                        path.push(segment.clone());
                    }
                }
                if path.last() != Some(action) && self.path.len() == 1 {
                    path.push(action.clone());
                }
            }
            (Some(action), None) => {
                path.push(action.clone());
                for segment in &self.path {
                    if path.last() != Some(segment) {
                        path.push(segment.clone());
                    }
                }
            }
            _ => {
                for segment in &self.path {
                    if path.last() != Some(segment) {
                        path.push(segment.clone());
                    }
                }
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
        if self.locator.is_none() && self.target.is_none() {
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
    /// 别名：设置目标资源/操作名；首次设置根 want，后续调用仅追加 path
    pub fn set_target<S: Into<String>>(&mut self, target: S) {
        self.push_target_segment(target.into())
    }

    pub fn path_string(&self) -> Option<String> {
        self.normalized_path_string()
    }

    pub fn record_field<S1, S2>(&mut self, key: S1, val: S2)
    where
        Self: ContextRecord<S1, S2>,
    {
        <Self as ContextRecord<S1, S2>>::record_field(self, key, val);
    }

    pub fn with_field<S1, S2>(mut self, key: S1, val: S2) -> Self
    where
        Self: ContextRecord<S1, S2>,
    {
        self.record_field(key, val);
        self
    }

    pub fn record_meta<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<MetadataValue>,
    {
        self.metadata.insert(key, value);
    }

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

    /// 格式化上下文信息，用于日志输出
    #[cfg_attr(not(any(feature = "log", feature = "tracing")), allow(dead_code))]
    fn format_context(&self) -> String {
        let want = self.target.clone().unwrap_or_default();
        let path = self.normalized_path_string().unwrap_or_default();
        let action = self.action.clone().unwrap_or_default();
        let locator = self.locator.clone().unwrap_or_default();
        let mut parts = Vec::new();
        if !action.is_empty() {
            parts.push(format!("doing={action}"));
        } else if !want.is_empty() {
            parts.push(format!("want={want}"));
        }
        if !locator.is_empty() {
            parts.push(format!("at={locator}"));
        }
        if !path.is_empty() && path != want && path != locator {
            parts.push(format!("path={path}"));
        }
        let head = if parts.is_empty() {
            match (want.is_empty(), path.is_empty() || path == want) {
                (true, true) => String::new(),
                (false, true) => format!("want={want}"),
                (false, false) => format!("want={want} path={path}"),
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

    /// 创建作用域 guard，默认为失败状态，需显式 `mark_success()`
    pub fn scope(&mut self) -> OperationScope<'_> {
        OperationScope {
            ctx: self,
            mark_success: false,
        }
    }

    /// 创建作用域 guard，在作用域结束时自动标记成功
    pub fn scoped_success(&mut self) -> OperationScope<'_> {
        OperationScope {
            ctx: self,
            mark_success: true,
        }
    }

    /// 记录日志信息，在无错误情况下也可以提供有价值的上下文信息
    /// 注意：需要启用 `log` 或 `tracing` 特性
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

    /// 与文档示例一致的别名方法（调用上面的同名方法）
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

    pub(crate) fn context_mut_for_report(&mut self) -> &mut CallContext {
        &mut self.context
    }

    pub(crate) fn replace_target_for_report(&mut self, target: Option<String>) {
        self.target = target;
    }

    pub(crate) fn replace_action_for_report(&mut self, action: Option<String>) {
        self.action = action;
    }

    pub(crate) fn replace_locator_for_report(&mut self, locator: Option<String>) {
        self.locator = locator;
    }

    pub(crate) fn replace_path_for_report(&mut self, path: Vec<String>) {
        self.path = path;
    }

    pub(crate) fn replace_metadata_for_report(&mut self, metadata: ErrorMetadata) {
        self.metadata = metadata;
    }
}

pub struct OperationScope<'a> {
    ctx: &'a mut OperationContext,
    mark_success: bool,
}

impl<'a> OperationScope<'a> {
    /// 显式标记成功
    pub fn mark_success(&mut self) {
        self.mark_success = true;
    }

    /// 保持失败状态（默认行为）
    pub fn mark_failure(&mut self) {
        self.mark_success = false;
    }

    /// 标记为取消并阻止成功写入
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

impl From<String> for OperationContext {
    fn from(value: String) -> Self {
        Self {
            target: None,
            action: None,
            locator: None,
            path: Vec::new(),
            context: CallContext::from(("key", value.to_string())),
            result: OperationResult::Fail,
            exit_log: false,
            mod_path: DEFAULT_MOD_PATH.into(),
            metadata: ErrorMetadata::default(),
        }
    }
}

impl From<&PathBuf> for OperationContext {
    fn from(value: &PathBuf) -> Self {
        Self {
            target: None,
            action: None,
            locator: Some(format!("{}", value.display())),
            path: Vec::new(),
            context: CallContext::from(("path", format!("{}", value.display()))),
            result: OperationResult::Fail,
            exit_log: false,
            mod_path: DEFAULT_MOD_PATH.into(),
            metadata: ErrorMetadata::default(),
        }
    }
}

impl From<&Path> for OperationContext {
    fn from(value: &Path) -> Self {
        Self {
            target: None,
            action: None,
            locator: Some(format!("{}", value.display())),
            path: Vec::new(),
            context: CallContext::from(("path", format!("{}", value.display()))),
            result: OperationResult::Fail,
            exit_log: false,
            mod_path: DEFAULT_MOD_PATH.into(),
            metadata: ErrorMetadata::default(),
        }
    }
}

impl From<&str> for OperationContext {
    fn from(value: &str) -> Self {
        Self {
            target: None,
            action: None,
            locator: None,
            path: Vec::new(),
            context: CallContext::from(("key", value.to_string())),
            result: OperationResult::Fail,
            exit_log: false,
            mod_path: DEFAULT_MOD_PATH.into(),
            metadata: ErrorMetadata::default(),
        }
    }
}

impl From<(&str, &str)> for OperationContext {
    fn from(value: (&str, &str)) -> Self {
        Self {
            target: None,
            action: None,
            locator: None,
            path: Vec::new(),
            context: CallContext::from((value.0, value.1)),
            result: OperationResult::Fail,
            exit_log: false,
            mod_path: DEFAULT_MOD_PATH.into(),
            metadata: ErrorMetadata::default(),
        }
    }
}

impl From<(&str, String)> for OperationContext {
    fn from(value: (&str, String)) -> Self {
        Self {
            target: None,
            action: None,
            locator: None,
            path: Vec::new(),
            context: CallContext::from((value.0, value.1)),
            result: OperationResult::Fail,
            exit_log: false,
            mod_path: DEFAULT_MOD_PATH.into(),
            metadata: ErrorMetadata::default(),
        }
    }
}
// Marker trait to exclude types that are already covered by other implementations
trait NotAsRefStr: AsRef<Path> {}

// Implement for concrete path types but not for &str
impl NotAsRefStr for PathBuf {}
impl NotAsRefStr for Path {}
impl<T: AsRef<Path> + ?Sized> NotAsRefStr for &T where T: NotAsRefStr {}

impl<V: AsRef<Path>> From<(&str, V)> for OperationContext
where
    V: NotAsRefStr,
{
    fn from(value: (&str, V)) -> Self {
        Self {
            target: None,
            action: None,
            locator: None,
            path: Vec::new(),
            context: CallContext {
                items: vec![(
                    value.0.to_string(),
                    format!("{}", value.1.as_ref().display()),
                )],
            },
            result: OperationResult::Fail,
            exit_log: false,
            mod_path: DEFAULT_MOD_PATH.into(),
            metadata: ErrorMetadata::default(),
        }
    }
}

impl From<(String, String)> for OperationContext {
    fn from(value: (String, String)) -> Self {
        Self {
            target: None,
            action: None,
            locator: None,
            path: Vec::new(),
            context: CallContext::from((value.0, value.1)),
            result: OperationResult::Fail,
            exit_log: false,
            mod_path: DEFAULT_MOD_PATH.into(),
            metadata: ErrorMetadata::default(),
        }
    }
}

impl From<&OperationContext> for OperationContext {
    fn from(value: &OperationContext) -> Self {
        value.clone()
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CallContext {
    pub items: Vec<(String, String)>,
}

impl<K: AsRef<str>, V: AsRef<str>> From<(K, V)> for CallContext {
    fn from(value: (K, V)) -> Self {
        Self {
            items: vec![(value.0.as_ref().to_string(), value.1.as_ref().to_string())],
        }
    }
}

pub trait ContextAdd<T> {
    fn add_context(&mut self, val: T);
}

impl<K: Into<String>> ContextAdd<(K, String)> for OperationContext {
    fn add_context(&mut self, val: (K, String)) {
        self.record_field(val.0.into(), val.1);
    }
}
impl<K: Into<String>> ContextAdd<(K, &String)> for OperationContext {
    fn add_context(&mut self, val: (K, &String)) {
        self.record_field(val.0.into(), val.1.clone());
    }
}
impl<K: Into<String>> ContextAdd<(K, &str)> for OperationContext {
    fn add_context(&mut self, val: (K, &str)) {
        self.record_field(val.0.into(), val.1.to_string());
    }
}

impl<K: Into<String>> ContextAdd<(K, &PathBuf)> for OperationContext {
    fn add_context(&mut self, val: (K, &PathBuf)) {
        self.record_field(val.0.into(), format!("{}", val.1.display()));
    }
}
impl<K: Into<String>> ContextAdd<(K, &Path)> for OperationContext {
    fn add_context(&mut self, val: (K, &Path)) {
        self.record_field(val.0.into(), format!("{}", val.1.display()));
    }
}

impl Display for CallContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.items.is_empty() {
            writeln!(f, "\ncall context:")?;
        }
        for (k, v) in &self.items {
            writeln!(f, "\t{k} : {v}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_op_context_macro_sets_callsite_mod_path() {
        let ctx = crate::op_context!("macro_target");
        assert_eq!(*ctx.target(), Some("macro_target".to_string()));
        assert_eq!(ctx.mod_path().as_str(), module_path!());
    }

    #[test]
    fn test_withcontext_new() {
        let ctx = OperationContext::new();
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 0);
    }

    #[test]
    #[allow(deprecated)]
    fn test_withcontext_want() {
        let ctx = OperationContext::want("test_target");
        assert_eq!(*ctx.target(), Some("test_target".to_string()));
        assert_eq!(ctx.path(), &["test_target".to_string()]);
        assert_eq!(ctx.context().items.len(), 0);
    }

    #[test]
    fn test_doing_records_v2_action_with_compat_target_projection() {
        let mut ctx = OperationContext::doing("load_config");
        ctx.with_at("config.toml");

        assert_eq!(ctx.action().as_deref(), Some("load_config"));
        assert_eq!(ctx.locator().as_deref(), Some("config.toml"));
        assert_eq!(ctx.target().as_deref(), Some("load_config"));
        assert_eq!(
            ctx.path(),
            &["load_config".to_string(), "config.toml".to_string()]
        );

        let rendered = ctx.to_string();
        assert!(rendered.contains("doing: load_config"));
        assert!(rendered.contains("at: config.toml"));
    }

    #[test]
    fn test_withcontext_with() {
        let mut ctx = OperationContext::new();
        ctx.record("key1", "value1");
        ctx.record("key2", "value2");

        assert_eq!(ctx.context().items.len(), 2);
        assert_eq!(
            ctx.context().items[0],
            ("key1".to_string(), "value1".to_string())
        );
        assert_eq!(
            ctx.context().items[1],
            ("key2".to_string(), "value2".to_string())
        );
    }

    #[test]
    fn test_withcontext_with_path() {
        let mut ctx = OperationContext::new();
        let path = PathBuf::from("/test/path");
        ctx.record("file_path", &path);

        assert_eq!(ctx.context().items.len(), 1);
        assert!(ctx.context().items[0].1.contains("/test/path"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_withcontext_with_want() {
        let mut ctx = OperationContext::new();
        ctx.with_want("new_target");

        assert_eq!(*ctx.target(), Some("new_target".to_string()));
        assert_eq!(ctx.path(), &["new_target".to_string()]);
    }

    #[test]
    #[allow(deprecated)]
    fn test_withcontext_with_want_appends_path() {
        let mut ctx = OperationContext::want("place_order");
        ctx.with_want("read_order_payload");
        ctx.with_want("parse_order");

        assert_eq!(*ctx.target(), Some("place_order".to_string()));
        assert_eq!(
            ctx.path(),
            &[
                "place_order".to_string(),
                "read_order_payload".to_string(),
                "parse_order".to_string()
            ]
        );
        assert_eq!(
            ctx.path_string().as_deref(),
            Some("place_order / read_order_payload / parse_order")
        );
    }

    #[test]
    fn test_errcontext_from_string() {
        let ctx = CallContext::from(("key".to_string(), "test_string".to_string()));
        assert_eq!(ctx.items.len(), 1);
        assert_eq!(ctx.items[0], ("key".to_string(), "test_string".to_string()));
    }

    #[test]
    fn test_errcontext_from_str() {
        let ctx = CallContext::from(("key", "test_str"));
        assert_eq!(ctx.items.len(), 1);
        assert_eq!(ctx.items[0], ("key".to_string(), "test_str".to_string()));
    }

    #[test]
    fn test_errcontext_from_string_pair() {
        let ctx = CallContext::from(("key1".to_string(), "value1".to_string()));
        assert_eq!(ctx.items.len(), 1);
        assert_eq!(ctx.items[0], ("key1".to_string(), "value1".to_string()));
    }

    #[test]
    fn test_errcontext_from_str_pair() {
        let ctx = CallContext::from(("key1", "value1"));
        assert_eq!(ctx.items.len(), 1);
        assert_eq!(ctx.items[0], ("key1".to_string(), "value1".to_string()));
    }

    #[test]
    fn test_errcontext_from_mixed_pair() {
        let ctx = CallContext::from(("key1", "value1".to_string()));
        assert_eq!(ctx.items.len(), 1);
        assert_eq!(ctx.items[0], ("key1".to_string(), "value1".to_string()));
    }

    #[test]
    fn test_errcontext_default() {
        let ctx = CallContext::default();
        assert_eq!(ctx.items.len(), 0);
    }

    #[test]
    fn test_errcontext_display_single() {
        let ctx = CallContext::from(("key", "test"));
        let display = format!("{ctx}");
        assert!(display.contains("call context:"));
        assert!(display.contains("key : test"));
    }

    #[test]
    fn test_errcontext_display_multiple() {
        let mut ctx = CallContext::default();
        ctx.items.push(("key1".to_string(), "value1".to_string()));
        ctx.items.push(("key2".to_string(), "value2".to_string()));
        let display = format!("{ctx}");
        assert!(display.contains("call context:"));
        assert!(display.contains("key1 : value1"));
        assert!(display.contains("key2 : value2"));
    }

    #[test]
    fn test_errcontext_display_empty() {
        let ctx = CallContext::default();
        let display = format!("{ctx}");
        assert_eq!(display, "");
    }

    #[test]
    fn test_withcontext_from_string() {
        let ctx = OperationContext::from("test_string".to_string());
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 1);
        assert_eq!(
            ctx.context().items[0],
            ("key".to_string(), "test_string".to_string())
        );
    }

    #[test]
    fn test_withcontext_from_str() {
        let ctx = OperationContext::from("test_str".to_string());
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 1);
        assert_eq!(
            ctx.context().items[0],
            ("key".to_string(), "test_str".to_string())
        );
    }

    #[test]
    fn test_withcontext_from_pathbuf() {
        let path = PathBuf::from("/test/path");
        let ctx = OperationContext::from(&path);
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 1);
        assert!(ctx.context().items[0].1.contains("/test/path"));
    }

    #[test]
    fn test_withcontext_from_path() {
        let path = "/test/path";
        let ctx = OperationContext::from(path);
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 1);
        assert!(ctx.context().items[0].1.contains("/test/path"));
    }

    #[test]
    fn test_withcontext_from_string_pair() {
        let ctx = OperationContext::from(("key1".to_string(), "value1".to_string()));
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 1);
        assert_eq!(
            ctx.context().items[0],
            ("key1".to_string(), "value1".to_string())
        );
    }

    #[test]
    fn test_withcontext_from_str_pair() {
        let ctx = OperationContext::from(("key1", "value1"));
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 1);
        assert_eq!(
            ctx.context().items[0],
            ("key1".to_string(), "value1".to_string())
        );
    }

    #[test]
    fn test_withcontext_from_mixed_pair() {
        let ctx = OperationContext::from(("key1", "value1".to_string()));
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 1);
        assert_eq!(
            ctx.context().items[0],
            ("key1".to_string(), "value1".to_string())
        );
    }

    #[test]
    fn test_withcontext_from_path_pair() {
        let path = PathBuf::from("/test/path");
        let ctx = OperationContext::from(("file", path.to_string_lossy().as_ref()));
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 1);
        assert!(ctx.context().items[0].0.contains("file"));
        assert!(ctx.context().items[0].1.contains("/test/path"));
    }

    #[test]
    fn test_withcontext_display_with_target() {
        let mut ctx = OperationContext::doing("test_target");
        ctx.record("key1", "value1");
        let display = format!("{ctx}");
        assert!(display.contains("doing: test_target"));
        assert!(display.contains("1. key1: value1"));
    }

    #[test]
    fn test_withcontext_display_without_target() {
        let mut ctx = OperationContext::new();
        ctx.record("key1", "value1");
        let display = format!("{ctx}");
        assert!(!display.contains("target:"));
        assert!(display.contains("1. key1: value1"));
    }

    #[test]
    fn test_withcontext_from_errcontext() {
        let err_ctx = CallContext::from(("key1", "value1"));
        let ctx = OperationContext::from(err_ctx);
        assert!(ctx.target.is_none());
        assert_eq!(ctx.context().items.len(), 1);
        assert_eq!(
            ctx.context().items[0],
            ("key1".to_string(), "value1".to_string())
        );
    }

    #[test]
    fn test_withcontext_from_withcontext() {
        let mut ctx1 = OperationContext::doing("target1");
        ctx1.record("key1", "value1");
        ctx1.with_doing("step1");
        let ctx2 = OperationContext::from(&ctx1);
        assert_eq!(*ctx2.target(), Some("target1".to_string()));
        assert_eq!(ctx2.path(), &["target1".to_string(), "step1".to_string()]);
        assert_eq!(ctx2.context().items.len(), 1);
        assert_eq!(
            ctx2.context().items[0],
            ("key1".to_string(), "value1".to_string())
        );
    }

    #[test]
    fn test_withcontext_from_str_path_pair() {
        let path = PathBuf::from("/test/path");
        let ctx = OperationContext::from(("file", &path));
        assert_eq!(ctx.context().items.len(), 1);
        assert_eq!(ctx.context().items[0].0, "file");
        assert!(ctx.context().items[0].1.contains("/test/path"));
    }

    #[test]
    fn test_withcontext_from_str_pathbuf_pair() {
        let path = PathBuf::from("/test/pathbuf");
        let ctx = OperationContext::from(("file", path));
        assert_eq!(ctx.context().items.len(), 1);
        assert_eq!(ctx.context().items[0].0, "file");
        assert!(ctx.context().items[0].1.contains("/test/pathbuf"));
    }

    // ContextAdd trait tests are commented out due to trait implementation issues
    // These tests will be revisited when the ContextAdd trait is properly implemented

    #[test]
    fn test_withcontext_edge_cases() {
        let ctx1 = OperationContext::from("".to_string());
        assert_eq!(ctx1.context().items.len(), 1);
        assert_eq!(ctx1.context().items[0], ("key".to_string(), "".to_string()));

        let ctx2 = OperationContext::from(("".to_string(), "".to_string()));
        assert_eq!(ctx2.context().items.len(), 1);
        assert_eq!(ctx2.context().items[0], ("".to_string(), "".to_string()));
    }

    #[test]
    fn test_errcontext_equality() {
        let ctx1 = CallContext::from(("key1", "value1"));
        let ctx2 = CallContext::from(("key1", "value1"));
        let ctx3 = CallContext::from(("key1", "value2"));

        assert_eq!(ctx1, ctx2);
        assert_ne!(ctx1, ctx3);
    }

    #[test]
    fn test_withcontext_equality() {
        let ctx1 = OperationContext::from(("key1", "value1"));
        let ctx2 = OperationContext::from(("key1", "value1"));
        let ctx3 = OperationContext::from(("key1", "value2"));

        assert_eq!(ctx1, ctx2);
        assert_ne!(ctx1, ctx3);
    }

    #[test]
    fn test_withcontext_clone() {
        let mut ctx = OperationContext::doing("target");
        ctx.record("key", "value");

        let cloned = ctx.clone();
        assert_eq!(ctx.target(), cloned.target());
        assert_eq!(ctx.context().items.len(), cloned.context().items.len());
        assert_eq!(ctx.context().items[0], cloned.context().items[0]);
    }

    #[test]
    fn test_withcontext_with_types() {
        let mut ctx = OperationContext::new();

        // 测试各种类型转换
        ctx.record("string_key", "string_value");
        ctx.record("string_key", 42.to_string()); // 数字转字符串
        ctx.record("bool_key", true.to_string()); // 布尔转字符串

        assert_eq!(ctx.context().items.len(), 3);

        // 验证最后一个添加的值
        assert_eq!(
            ctx.context().items[2],
            ("bool_key".to_string(), "true".to_string())
        );
    }

    #[test]
    fn test_mark_suc() {
        let mut ctx = OperationContext::new();
        assert!(ctx.result == OperationResult::Fail);

        ctx.mark_suc();
        assert!(ctx.result == OperationResult::Suc);
    }

    #[test]
    fn test_with_exit_log() {
        let ctx = OperationContext::new().with_auto_log();
        assert!(ctx.exit_log);

        let ctx2 = OperationContext::doing("test").with_auto_log();
        assert!(ctx2.exit_log);
        assert_eq!(*ctx2.target(), Some("test".to_string()));
    }

    #[test]
    fn test_scope_marks_success() {
        let mut ctx = OperationContext::doing("scope_success");
        {
            let _scope = ctx.scoped_success();
        }
        assert!(matches!(ctx.result(), OperationResult::Suc));
    }

    #[test]
    fn test_scope_preserves_failure() {
        let mut ctx = OperationContext::doing("scope_fail");
        {
            let mut scope = ctx.scoped_success();
            scope.mark_failure();
        }
        assert!(matches!(ctx.result(), OperationResult::Fail));
    }

    #[test]
    fn test_scope_cancel() {
        let mut ctx = OperationContext::doing("scope_cancel");
        {
            let mut scope = ctx.scoped_success();
            scope.cancel();
        }
        assert!(matches!(ctx.result(), OperationResult::Cancel));
    }

    #[test]
    fn test_format_context_with_target() {
        let mut ctx = OperationContext::doing("test_target");
        ctx.record("key1", "value1");

        let formatted = ctx.format_context();
        assert_eq!(
            formatted,
            "doing=test_target: \ncall context:\n\tkey1 : value1\n"
        );
    }

    #[test]
    fn test_format_context_without_target() {
        let mut ctx = OperationContext::new();
        ctx.record("key1", "value1");

        let formatted = ctx.format_context();
        assert_eq!(formatted, "call context:\n\tkey1 : value1\n");
    }

    #[test]
    fn test_format_context_empty() {
        let ctx = OperationContext::new();
        let formatted = ctx.format_context();
        assert_eq!(formatted, "");
    }

    #[test]
    fn test_format_context_with_target_only() {
        let ctx = OperationContext::doing("test_target");
        let formatted = ctx.format_context();
        assert_eq!(formatted, "doing=test_target");
    }

    #[test]
    fn test_format_context_with_path() {
        let mut ctx = OperationContext::doing("place_order");
        ctx.with_doing("read_order_payload");
        ctx.record("order_id", "42");

        let formatted = ctx.format_context();
        assert_eq!(
            formatted,
            "doing=place_order path=place_order / read_order_payload: \ncall context:\n\torder_id : 42\n"
        );
    }

    #[test]
    fn test_path_string_and_display_use_normalized_action_locator_order() {
        let mut ctx = OperationContext::at("engine.toml");
        ctx.with_doing("start engine");

        assert_eq!(
            ctx.path_string().as_deref(),
            Some("start engine / engine.toml")
        );
        assert_eq!(
            ctx.path(),
            &["start engine".to_string(), "engine.toml".to_string()]
        );

        let rendered = format!("{ctx}");
        assert!(rendered.contains("doing: start engine"));
        assert!(rendered.contains("at: engine.toml"));
        assert!(rendered.contains("path: start engine / engine.toml"));
        assert!(!rendered.contains("path: engine.toml / start engine"));
    }

    #[test]
    fn test_logging_methods() {
        let ctx = OperationContext::doing("test_target");

        // 这些方法主要测试它们不会panic，实际日志输出需要日志框架支持
        ctx.info("info message");
        ctx.debug("debug message");
        ctx.warn("warn message");
        ctx.error("error message");
        ctx.trace("trace message");
    }

    #[test]
    fn test_logging_methods_with_empty_context() {
        let ctx = OperationContext::new();

        // 测试空上下文时的日志方法
        ctx.info("info message");
        ctx.debug("debug message");
        ctx.warn("warn message");
        ctx.error("error message");
        ctx.trace("trace message");
    }

    #[test]
    fn test_context_add_trait() {
        let mut ctx = OperationContext::new();

        // 测试ContextAdd trait的实现
        ctx.add_context(("key1", "value1"));
        ctx.add_context(("key2", "value2"));

        assert_eq!(ctx.context().items.len(), 2);
        assert_eq!(
            ctx.context().items[0],
            ("key1".to_string(), "value1".to_string())
        );
        assert_eq!(
            ctx.context().items[1],
            ("key2".to_string(), "value2".to_string())
        );
    }

    #[test]
    fn test_drop_trait_with_success() {
        {
            let mut ctx = OperationContext::doing("test_drop").with_auto_log();
            ctx.record("operation", "test");
            ctx.mark_suc(); // 标记为成功
                            // ctx 在这里离开作用域，会触发Drop trait
        }
        // 注意：Drop trait的日志输出需要日志框架配置才能看到
        // 这里主要测试Drop trait不会panic
    }

    #[test]
    fn test_drop_trait_with_failure() {
        {
            let mut ctx = OperationContext::doing("test_drop_fail").with_auto_log();
            ctx.record("operation", "test_fail");
            // 不调用mark_suc，保持is_suc = false
            // ctx 在这里离开作用域，会触发Drop trait
        }
        // 注意：Drop trait的日志输出需要日志框架配置才能看到
        // 这里主要测试Drop trait不会panic
    }

    #[test]
    fn test_drop_trait_without_exit_log() {
        {
            let mut ctx = OperationContext::doing("test_no_log");
            ctx.record("operation", "no_log");
            ctx.mark_suc();
            // exit_log = false，不会触发日志输出
            // ctx 在这里离开作用域，Drop trait应该什么都不做
        }
        // 测试通过即可
    }

    #[test]
    fn test_complex_context_scenario() {
        // 模拟一个复杂的操作场景
        let mut ctx = OperationContext::doing("user_registration").with_auto_log();

        // 添加各种上下文信息
        ctx.record("user_id", "12345");
        ctx.record("email", "test@example.com");
        ctx.record("role", "user");

        // 记录各种级别的日志
        ctx.info("开始用户注册流程");
        ctx.debug("验证用户输入");
        ctx.warn("检测到潜在的安全风险");

        // 模拟操作成功
        ctx.mark_suc();
        ctx.info("用户注册成功");

        // 验证上下文状态
        assert!(ctx.result == OperationResult::Suc);
        assert!(ctx.exit_log);
        assert_eq!(*ctx.target(), Some("user_registration".to_string()));
        assert_eq!(ctx.context().items.len(), 3);

        // 验证format_context输出
        let formatted = ctx.format_context();
        assert!(formatted.contains("user_registration"));
        assert!(formatted.contains("user_id"));
        assert!(formatted.contains("email"));
        assert!(formatted.contains("role"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_context_serialization() {
        let mut ctx = OperationContext::doing("serialization_test");
        ctx.with_doing("inner_step");
        ctx.record("key1", "value1");
        ctx.record("key2", "value2");

        // 测试序列化
        let serialized = serde_json::to_string(&ctx).expect("序列化失败");
        assert!(serialized.contains("serialization_test"));
        assert!(serialized.contains("inner_step"));
        assert!(serialized.contains("key1"));
        assert!(serialized.contains("value1"));

        // 测试反序列化
        let deserialized: OperationContext =
            serde_json::from_str(&serialized).expect("反序列化失败");
        assert_eq!(ctx, deserialized);
    }

    #[test]
    fn test_context_with_special_characters() {
        let mut ctx = OperationContext::new();

        // 测试特殊字符
        ctx.record("key_with_spaces", "value with spaces");
        ctx.record("key_with_unicode", "值包含中文");
        ctx.record("key_with_symbols", "value@#$%^&*()");

        assert_eq!(ctx.context().items.len(), 3);
        assert_eq!(
            ctx.context().items[0],
            (
                "key_with_spaces".to_string(),
                "value with spaces".to_string()
            )
        );
        assert_eq!(
            ctx.context().items[1],
            ("key_with_unicode".to_string(), "值包含中文".to_string())
        );
        assert_eq!(
            ctx.context().items[2],
            ("key_with_symbols".to_string(), "value@#$%^&*()".to_string())
        );

        // 测试显示
        let display = format!("{ctx}");
        assert!(display.contains("key_with_spaces"));
        assert!(display.contains("值包含中文"));
        assert!(display.contains("value@#$%^&*()"));
    }

    #[test]
    fn test_context_builder_pattern() {
        // 测试构建器模式的使用
        let ctx = OperationContext::doing("builder_test").with_auto_log();

        assert_eq!(*ctx.target(), Some("builder_test".to_string()));
        assert_eq!(ctx.path(), &["builder_test".to_string()]);
        assert!(ctx.exit_log);
    }

    #[test]
    fn test_context_multiple_with_calls() {
        let mut ctx = OperationContext::new();

        // 多次调用with方法
        ctx.record("key1", "value1");
        ctx.record("key2", "value2");
        ctx.record("key3", "value3");
        ctx.record("key1", "new_value1"); // 覆盖key1

        // 注意：当前实现允许重复的key，这是预期的行为
        assert_eq!(ctx.context().items.len(), 4);
        assert_eq!(
            ctx.context().items[0],
            ("key1".to_string(), "value1".to_string())
        );
        assert_eq!(
            ctx.context().items[3],
            ("key1".to_string(), "new_value1".to_string())
        );
    }

    #[test]
    fn test_context_from_various_types() {
        // 测试从各种类型创建OperationContext
        let ctx1 = OperationContext::from("simple_string");
        assert_eq!(
            ctx1.context().items[0],
            ("key".to_string(), "simple_string".to_string())
        );

        let ctx2 = OperationContext::from(("custom_key", "custom_value"));
        assert_eq!(
            ctx2.context().items[0],
            ("custom_key".to_string(), "custom_value".to_string())
        );

        let path = PathBuf::from("/test/path/file.txt");
        let ctx3 = OperationContext::from(&path);
        assert!(ctx3.context().items[0].0.contains("path"));
        assert!(ctx3.context().items[0].1.contains("/test/path/file.txt"));
    }

    // ContextTake trait 测试用例
    #[test]
    fn test_context_take_with_string_types() {
        let mut ctx = OperationContext::new();

        // 测试字符串类型的ContextTake实现
        ctx.record("string_key", "string_value");
        ctx.record("string_key2", String::from("string_value2"));
        ctx.record(String::from("string_key3"), "string_value3");
        ctx.record(String::from("string_key4"), String::from("string_value4"));

        assert_eq!(ctx.context().items.len(), 4);
        assert_eq!(
            ctx.context().items[0],
            ("string_key".to_string(), "string_value".to_string())
        );
        assert_eq!(
            ctx.context().items[1],
            ("string_key2".to_string(), "string_value2".to_string())
        );
        assert_eq!(
            ctx.context().items[2],
            ("string_key3".to_string(), "string_value3".to_string())
        );
        assert_eq!(
            ctx.context().items[3],
            ("string_key4".to_string(), "string_value4".to_string())
        );
    }

    #[test]
    fn test_context_take_with_numeric_types() {
        let mut ctx = OperationContext::new();

        // 测试数字类型的ContextTake实现（需要转换为字符串）
        ctx.record("int_key", 42.to_string());
        ctx.record("float_key", 3.24.to_string());
        ctx.record("bool_key", true.to_string());

        assert_eq!(ctx.context().items.len(), 3);
        assert_eq!(
            ctx.context().items[0],
            ("int_key".to_string(), "42".to_string())
        );
        assert_eq!(
            ctx.context().items[1],
            ("float_key".to_string(), "3.24".to_string())
        );
        assert_eq!(
            ctx.context().items[2],
            ("bool_key".to_string(), "true".to_string())
        );
    }

    #[test]
    fn test_context_take_with_path_context() {
        let mut ctx = OperationContext::new();

        // 测试PathContext包装类型的ContextTake实现
        let path1 = PathBuf::from("/test/path1.txt");
        let path2 = Path::new("/test/path2.txt");

        ctx.record("file1", &path1);
        ctx.record("file2", path2);

        assert_eq!(ctx.context().items.len(), 2);
        assert_eq!(ctx.context().items[0].0, "file1");
        assert!(ctx.context().items[0].1.contains("/test/path1.txt"));
        assert_eq!(ctx.context().items[1].0, "file2");
        assert!(ctx.context().items[1].1.contains("/test/path2.txt"));
    }

    #[test]
    fn test_context_take_mixed_types() {
        let mut ctx = OperationContext::new();

        // 测试混合使用字符串和PathContext类型
        ctx.record("name", "test_user");
        ctx.record("age", 25.to_string());
        ctx.record("config_file", &PathBuf::from("/etc/config.toml"));
        ctx.record("status", "active");

        assert_eq!(ctx.context().items.len(), 4);
        assert_eq!(
            ctx.context().items[0],
            ("name".to_string(), "test_user".to_string())
        );
        assert_eq!(
            ctx.context().items[1],
            ("age".to_string(), "25".to_string())
        );
        assert_eq!(ctx.context().items[2].0, "config_file");
        assert!(ctx.context().items[2].1.contains("/etc/config.toml"));
        assert_eq!(
            ctx.context().items[3],
            ("status".to_string(), "active".to_string())
        );
    }

    #[test]
    fn test_context_take_edge_cases() {
        let mut ctx = OperationContext::new();

        // 测试边界情况
        ctx.record("", ""); // 空字符串
        ctx.record("empty_value", ""); // 空值
        ctx.record("", "empty_key"); // 空键
        ctx.record("special_chars", "@#$%^&*()"); // 特殊字符
        ctx.record("unicode", "测试中文字符"); // Unicode字符

        assert_eq!(ctx.context().items.len(), 5);
        assert_eq!(ctx.context().items[0], ("".to_string(), "".to_string()));
        assert_eq!(
            ctx.context().items[1],
            ("empty_value".to_string(), "".to_string())
        );
        assert_eq!(
            ctx.context().items[2],
            ("".to_string(), "empty_key".to_string())
        );
        assert_eq!(
            ctx.context().items[3],
            ("special_chars".to_string(), "@#$%^&*()".to_string())
        );
        assert_eq!(
            ctx.context().items[4],
            ("unicode".to_string(), "测试中文字符".to_string())
        );
    }

    #[test]
    fn test_context_take_multiple_calls() {
        let mut ctx = OperationContext::new();

        // 测试多次调用take方法
        ctx.record("key1", "value1");
        ctx.record("key2", "value2");
        ctx.record("key1", "new_value1"); // 覆盖key1
        ctx.record("key3", &PathBuf::from("/path/file.txt"));
        ctx.record("key2", &PathBuf::from("/path/file2.txt")); // 覆盖key2

        // 注意：当前实现允许重复的key，这是预期的行为
        assert_eq!(ctx.context().items.len(), 5);
        assert_eq!(
            ctx.context().items[0],
            ("key1".to_string(), "value1".to_string())
        );
        assert_eq!(
            ctx.context().items[1],
            ("key2".to_string(), "value2".to_string())
        );
        assert_eq!(
            ctx.context().items[2],
            ("key1".to_string(), "new_value1".to_string())
        );
        assert_eq!(ctx.context().items[3].0, "key3");
        assert!(ctx.context().items[3].1.contains("/path/file.txt"));
        assert_eq!(ctx.context().items[4].0, "key2");
        assert!(ctx.context().items[4].1.contains("/path/file2.txt"));
    }

    #[test]
    fn test_context_take_with_existing_context() {
        // 创建一个已有上下文的OperationContext
        let mut ctx = OperationContext::from(("existing_key", "existing_value"));

        // 使用ContextTake添加更多上下文
        ctx.record("new_key1", "new_value1");
        ctx.record("new_key2", &PathBuf::from("/new/path.txt"));

        assert_eq!(ctx.context().items.len(), 3);
        assert_eq!(
            ctx.context().items[0],
            ("existing_key".to_string(), "existing_value".to_string())
        );
        assert_eq!(
            ctx.context().items[1],
            ("new_key1".to_string(), "new_value1".to_string())
        );
        assert_eq!(ctx.context().items[2].0, "new_key2");
        assert!(ctx.context().items[2].1.contains("/new/path.txt"));
    }

    #[test]
    fn test_context_metadata_records_values() {
        let ctx = OperationContext::doing("load")
            .with_meta("config.kind", "wpsrc")
            .with_meta("parse.line", 1u32)
            .with_meta("parse.strict", true);

        assert_eq!(ctx.metadata().get_str("config.kind"), Some("wpsrc"));
        assert!(ctx.metadata().as_map().contains_key("parse.line"));
        assert!(ctx.metadata().as_map().contains_key("parse.strict"));
    }

    #[test]
    fn test_context_metadata_duplicate_key_overwrites() {
        let ctx = OperationContext::new()
            .with_meta("config.kind", "sink_route")
            .with_meta("config.kind", "sink_defaults");

        assert_eq!(ctx.metadata().get_str("config.kind"), Some("sink_defaults"));
    }

    #[test]
    fn test_context_metadata_ignores_empty_key() {
        let result = std::panic::catch_unwind(|| OperationContext::new().with_meta("", "ignored"));

        if cfg!(debug_assertions) {
            assert!(result.is_err());
        } else {
            let ctx = result.expect("release build should ignore empty metadata key");
            assert!(ctx.metadata().is_empty());
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_context_serialization_skips_empty_metadata_and_reads_missing_field() {
        let ctx = OperationContext::doing("serialization_test");

        let serialized = serde_json::to_value(&ctx).expect("序列化失败");
        assert!(!serialized
            .as_object()
            .expect("object")
            .contains_key("metadata"));

        let deserialized: OperationContext =
            serde_json::from_value(serialized).expect("反序列化失败");
        assert!(deserialized.metadata().is_empty());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_context_serialization_preserves_metadata() {
        let ctx = OperationContext::doing("serialization_test")
            .with_meta("config.kind", "sink_defaults")
            .with_meta("parse.line", 3u32);

        let serialized = serde_json::to_value(&ctx).expect("序列化失败");
        assert_eq!(
            serialized["metadata"]["config.kind"],
            serde_json::Value::String("sink_defaults".to_string())
        );
        assert_eq!(serialized["metadata"]["parse.line"], serde_json::json!(3));

        let deserialized: OperationContext =
            serde_json::from_value(serialized).expect("反序列化失败");
        assert_eq!(
            deserialized.metadata().get_str("config.kind"),
            Some("sink_defaults")
        );
    }
}
