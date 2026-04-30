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

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CallContext {
    pub(crate) items: Vec<(String, String)>,
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
