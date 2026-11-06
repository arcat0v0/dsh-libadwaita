use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldKind {
    Bool,
    Number,
    Text,
    Enum(Vec<String>),
    Unavailable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldSpec {
    pub key: String,
    pub label: String,
    pub kind: FieldKind,
    pub value: Value,
}

pub fn fields_from_schema(schema: &Value, value: &Value) -> Vec<FieldSpec> {
    let Some(properties) = schema
        .get("properties")
        .and_then(|properties| properties.as_object())
    else {
        return Vec::new();
    };
    let mut fields = Vec::new();
    for (key, property) in properties {
        if key.to_ascii_lowercase().contains("secret")
            || key.to_ascii_lowercase().contains("password")
            || key.to_ascii_lowercase().contains("apikey")
        {
            continue;
        }
        let label = property
            .get("title")
            .and_then(|title| title.as_str())
            .unwrap_or(key)
            .to_string();
        let kind = if let Some(options) = property.get("enum").and_then(|enum_| enum_.as_array()) {
            FieldKind::Enum(
                options
                    .iter()
                    .filter_map(|option| option.as_str().map(str::to_string))
                    .collect(),
            )
        } else {
            match property.get("type").and_then(|type_| type_.as_str()) {
                Some("boolean") => FieldKind::Bool,
                Some("number") | Some("integer") => FieldKind::Number,
                Some("string") => FieldKind::Text,
                _ => FieldKind::Unavailable,
            }
        };
        fields.push(FieldSpec {
            key: key.clone(),
            label,
            kind,
            value: value.get(key).cloned().unwrap_or(Value::Null),
        });
    }
    fields
}

pub fn find_key(schema: &Value, candidates: &[&str]) -> Option<String> {
    let properties = schema.get("properties")?.as_object()?;
    for candidate in candidates {
        if properties.contains_key(*candidate) {
            return Some((*candidate).to_string());
        }
    }
    None
}

pub fn schema_has_namespace<'a>(
    descriptors: &'a [crate::protocol::SettingsDescriptor],
    namespace: &str,
) -> Option<&'a crate::protocol::SettingsDescriptor> {
    descriptors.iter().find(|descriptor| descriptor.ns == namespace)
}
