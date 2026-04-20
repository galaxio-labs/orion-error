use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ErrorMetadata {
    fields: BTreeMap<String, MetadataValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum MetadataValue {
    String(String),
    Bool(bool),
    I64(i64),
    U64(u64),
}

impl ErrorMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn as_map(&self) -> &BTreeMap<String, MetadataValue> {
        &self.fields
    }

    pub fn insert<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<MetadataValue>,
    {
        let key = key.into();
        debug_assert!(!key.is_empty(), "metadata key must not be empty");
        if key.is_empty() {
            return;
        }

        self.fields.insert(key, value.into());
    }

    pub fn get(&self, key: &str) -> Option<&MetadataValue> {
        self.fields.get(key)
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        match self.get(key) {
            Some(MetadataValue::String(value)) => Some(value.as_str()),
            _ => None,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &MetadataValue)> {
        self.fields.iter()
    }

    pub(crate) fn merge_missing(&mut self, other: &ErrorMetadata) {
        for (key, value) in other.iter() {
            self.fields
                .entry(key.clone())
                .or_insert_with(|| value.clone());
        }
    }
}

impl From<String> for MetadataValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for MetadataValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<bool> for MetadataValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i64> for MetadataValue {
    fn from(value: i64) -> Self {
        Self::I64(value)
    }
}

impl From<i32> for MetadataValue {
    fn from(value: i32) -> Self {
        Self::I64(i64::from(value))
    }
}

impl From<u64> for MetadataValue {
    fn from(value: u64) -> Self {
        Self::U64(value)
    }
}

impl From<u32> for MetadataValue {
    fn from(value: u32) -> Self {
        Self::U64(u64::from(value))
    }
}

impl From<usize> for MetadataValue {
    fn from(value: usize) -> Self {
        Self::U64(value as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::{ErrorMetadata, MetadataValue};

    #[test]
    fn test_metadata_insert_overwrites_duplicate_keys() {
        let mut metadata = ErrorMetadata::new();
        metadata.insert("config.kind", "sink_route");
        metadata.insert("config.kind", "sink_defaults");

        assert_eq!(metadata.get_str("config.kind"), Some("sink_defaults"));
    }

    #[test]
    fn test_metadata_merge_missing_keeps_existing_values() {
        let mut merged = ErrorMetadata::new();
        merged.insert("config.kind", "sink_defaults");

        let mut outer = ErrorMetadata::new();
        outer.insert("config.kind", "sink_route");
        outer.insert("config.group", "infra");

        merged.merge_missing(&outer);

        assert_eq!(merged.get_str("config.kind"), Some("sink_defaults"));
        assert_eq!(merged.get_str("config.group"), Some("infra"));
    }

    #[test]
    fn test_metadata_get_str_only_for_string_values() {
        let mut metadata = ErrorMetadata::new();
        metadata.insert("parse.line", 7u32);

        assert_eq!(metadata.get("parse.line"), Some(&MetadataValue::U64(7)));
        assert_eq!(metadata.get_str("parse.line"), None);
    }
}
