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
    if schema.get("refs").is_some() && schema.get("uid").is_some() {
        return fields_from_typert(schema, value);
    }
    let Some(properties) = schema
        .get("properties")
        .and_then(|properties| properties.as_object())
    else {
        return Vec::new();
    };
    let mut fields = Vec::new();
    for (key, property) in properties {
        if is_secret_key(key) {
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
    fields.retain(|field| !matches!(field.kind, FieldKind::Unavailable));
    fields
}

fn is_secret_key(key: &str) -> bool {
    let lowered = key.to_ascii_lowercase();
    lowered.contains("secret") || lowered.contains("password") || lowered.contains("apikey")
}

fn fields_from_typert(schema: &Value, value: &Value) -> Vec<FieldSpec> {
    let Some(refs) = schema.get("refs").and_then(Value::as_object) else {
        return Vec::new();
    };
    let Some(uid) = schema.get("uid").and_then(Value::as_u64) else {
        return Vec::new();
    };
    let root_ref = uid.to_string();
    let Some(dict) = refs
        .get(&root_ref)
        .and_then(|root| root.get("dict"))
        .and_then(Value::as_object)
    else {
        return Vec::new();
    };
    let mut fields = Vec::new();
    for (key, reference) in dict {
        if is_secret_key(key) {
            continue;
        }
        let Some(reference) = reference.as_u64().and_then(|id| refs.get(&id.to_string())) else {
            continue;
        };
        let Some(kind) = typert_kind(reference, refs) else {
            continue;
        };
        if matches!(kind, FieldKind::Unavailable) {
            continue;
        }
        fields.push(FieldSpec {
            key: key.clone(),
            label: humanize_key(key),
            kind,
            value: value.get(key).cloned().unwrap_or(Value::Null),
        });
    }
    fields
}

fn typert_kind(reference: &Value, refs: &serde_json::Map<String, Value>) -> Option<FieldKind> {
    match reference.get("type").and_then(Value::as_str)? {
        "boolean" => Some(FieldKind::Bool),
        "number" => Some(FieldKind::Number),
        "string" => Some(FieldKind::Text),
        "union" => {
            let options = typert_union_options(reference, refs)?;
            (!options.is_empty()).then_some(FieldKind::Enum(options))
        }
        _ => Some(FieldKind::Unavailable),
    }
}

fn typert_union_options(
    reference: &Value,
    refs: &serde_json::Map<String, Value>,
) -> Option<Vec<String>> {
    let list = reference.get("list").and_then(Value::as_array)?;
    let mut options = Vec::new();
    for item in list {
        let Some(item) = item.as_u64().and_then(|id| refs.get(&id.to_string())) else {
            return None;
        };
        if item.get("type").and_then(Value::as_str) != Some("const") {
            return None;
        }
        options.push(item.get("value")?.as_str()?.to_string());
    }
    Some(options)
}

fn humanize_key(key: &str) -> String {
    let mut label = String::with_capacity(key.len() + 4);
    for (index, character) in key.char_indices() {
        if index > 0 && character.is_ascii_uppercase() {
            label.push(' ');
            label.extend(character.to_lowercase());
        } else if index == 0 {
            label.extend(character.to_uppercase());
        } else {
            label.push(character);
        }
    }
    label
}

pub fn find_key(schema: &Value, candidates: &[&str]) -> Option<String> {
    if let Some(dict) = schema
        .get("refs")
        .and_then(|refs| {
            let uid = schema.get("uid")?.to_string();
            refs.get(&uid)
        })
        .and_then(|root| root.get("dict"))
        .and_then(Value::as_object)
    {
        for candidate in candidates {
            if dict.contains_key(*candidate) {
                return Some((*candidate).to_string());
            }
        }
        return None;
    }
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
