use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MessageTypeConfig {
    pub name: String,
    pub index_name: String,
    pub description: String,
    pub fields: HashMap<String, FieldConfig>,
    pub logic: Option<HashMap<String, toml::Value>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FieldConfig {
    pub r#type: String,
    pub values: Option<Vec<String>>,
    pub range: Option<Vec<f64>>,
    pub pattern: Option<String>,
    pub unit: Option<String>,
    pub optional: Option<bool>,
    pub hex_length: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    message_types: Vec<MessageTypeConfig>,
}

pub struct MessageTypesConfig {
    pub types: HashMap<String, MessageTypeConfig>,
}

impl MessageTypesConfig {
    pub fn load_from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: ConfigFile = toml::from_str(&content)?;
        
        let mut types = HashMap::new();
        for message_type in config.message_types {
            types.insert(message_type.name.clone(), message_type);
        }
        
        Ok(MessageTypesConfig { types })
    }

    pub fn get_type(&self, name: &str) -> Option<&MessageTypeConfig> {
        self.types.get(name)
    }

    pub fn list_types(&self) -> Vec<&String> {
        self.types.keys().collect()
    }
}

#[derive(Debug, Clone)]
pub enum FieldValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    DateTime(String),
    Uuid(String),
}

impl FieldValue {
    pub fn to_csv_string(&self) -> String {
        match self {
            FieldValue::String(s) => s.clone(),
            FieldValue::Integer(i) => i.to_string(),
            FieldValue::Float(f) => f.to_string(),
            FieldValue::Boolean(b) => b.to_string(),
            FieldValue::DateTime(dt) => dt.clone(),
            FieldValue::Uuid(u) => u.clone(),
        }
    }
}