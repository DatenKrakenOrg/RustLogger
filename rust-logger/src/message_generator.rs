use crate::config::{FieldConfig, FieldValue, MessageTypeConfig};
use chrono::{Duration, NaiveDate, NaiveDateTime, SecondsFormat, TimeZone, Utc};
use rand::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

pub struct MessageGenerator {
    config: MessageTypeConfig,
    years: (NaiveDate, NaiveDate),
    rng: rand::rngs::ThreadRng,
}

impl MessageGenerator {
    pub fn new(config: MessageTypeConfig, years: (i32, i32)) -> Result<Self, String> {
        if years.1 - years.0 <= 0 {
            return Err("Year range invalid: end_year must be greater than start_year".to_string());
        }

        Ok(Self {
            config,
            years: (
                NaiveDate::from_yo_opt(years.0, 1).unwrap(),
                NaiveDate::from_yo_opt(years.1, 1).unwrap(),
            ),
            rng: rand::rng(),
        })
    }

    pub fn generate_message(&mut self) -> HashMap<String, FieldValue> {
        let mut message = HashMap::new();
        let fields = self.config.fields.clone();

        for (field_name, field_config) in fields {
            let value = self.generate_field_value(&field_name, &field_config, &message);
            message.insert(field_name, value);
        }

        self.apply_contextual_logic(&mut message);
        message
    }

    fn generate_field_value(
        &mut self,
        _field_name: &str,
        config: &FieldConfig,
        _existing_values: &HashMap<String, FieldValue>,
    ) -> FieldValue {
        match config.r#type.as_str() {
            "datetime" => self.generate_datetime(),
            "enum" => self.generate_enum(config),
            "string" => self.generate_string(config),
            "float" => self.generate_float(config),
            "integer" => self.generate_integer(config),
            "uuid" => FieldValue::Uuid(Uuid::new_v4().to_string()),
            _ => FieldValue::String("unknown".to_string()),
        }
    }

    fn generate_datetime(&mut self) -> FieldValue {
        let days_in_range = (self.years.1 - self.years.0).num_days();
        let random_days: i64 = self.rng.random_range(0..days_in_range);
        let naive: NaiveDateTime = (self.years.0 + Duration::days(random_days))
            .and_hms_opt(
                self.rng.random_range(0..24),
                self.rng.random_range(0..60),
                self.rng.random_range(0..60),
            )
            .unwrap()
            .into();
        let timestamp = Utc.from_utc_datetime(&naive).to_rfc3339_opts(SecondsFormat::Millis, true);
        FieldValue::DateTime(timestamp)
    }

    fn generate_enum(&mut self, config: &FieldConfig) -> FieldValue {
        if let Some(values) = &config.values {
            let value = values.choose(&mut self.rng).unwrap().clone();
            FieldValue::String(value)
        } else {
            FieldValue::String("default".to_string())
        }
    }

    fn generate_string(&mut self, config: &FieldConfig) -> FieldValue {
        if let Some(pattern) = &config.pattern {
            if let Some(values) = &config.values {
                let base_value = values.choose(&mut self.rng).unwrap();
                let result = pattern.replace("{id}", base_value);
                FieldValue::String(result)
            } else if pattern.contains("{prefix}") {
                if let Some(values) = &config.values {
                    let prefix = values.choose(&mut self.rng).unwrap();
                    let result = pattern.replace("{prefix}", prefix);
                    FieldValue::String(result)
                } else {
                    FieldValue::String("default_table".to_string())
                }
            } else if pattern.contains("{env}") && pattern.contains("{service}") {
                if let Some(values) = &config.values {
                    let full_value = values.choose(&mut self.rng).unwrap();
                    FieldValue::String(full_value.clone())
                } else {
                    FieldValue::String("prod.service".to_string())
                }
            } else if pattern.contains("{hex8}") {
                let hex_value: String = (0..8)
                    .map(|_| self.rng.random_range(0..16))
                    .map(|n| format!("{:x}", n))
                    .collect();
                FieldValue::String(hex_value)
            } else if let Some(range) = &config.range {
                let id = self.rng.random_range(range[0] as i64..=range[1] as i64);
                let result = pattern.replace("{id}", &id.to_string());
                FieldValue::String(result)
            } else {
                FieldValue::String("default".to_string())
            }
        } else if let Some(values) = &config.values {
            let value = values.choose(&mut self.rng).unwrap().clone();
            FieldValue::String(value)
        } else {
            FieldValue::String("default".to_string())
        }
    }

    fn generate_float(&mut self, config: &FieldConfig) -> FieldValue {
        if let Some(range) = &config.range {
            let value = self.rng.random_range(range[0]..range[1]);
            FieldValue::Float(value)
        } else {
            FieldValue::Float(0.0)
        }
    }

    fn generate_integer(&mut self, config: &FieldConfig) -> FieldValue {
        if let Some(range) = &config.range {
            let value = self.rng.random_range(range[0] as i64..=range[1] as i64);
            FieldValue::Integer(value)
        } else if let Some(values) = &config.values {
            let int_values: Result<Vec<i64>, _> = values.iter().map(|v| v.parse()).collect();
            if let Ok(int_values) = int_values {
                let value = *int_values.choose(&mut self.rng).unwrap();
                FieldValue::Integer(value)
            } else {
                FieldValue::Integer(0)
            }
        } else {
            FieldValue::Integer(0)
        }
    }

    fn apply_contextual_logic(&mut self, message: &mut HashMap<String, FieldValue>) {
        match self.config.name.as_str() {
            "iot_sensor" => self.apply_iot_logic(message),
            "system_metrics" => self.apply_system_logic(message),
            "timescaledb" => self.apply_timescaledb_logic(message),
            "kafka" => self.apply_kafka_logic(message),
            "application_logs" => self.apply_application_logic(message),
            _ => {}
        }
    }

    fn apply_iot_logic(&mut self, message: &mut HashMap<String, FieldValue>) {
        let temp = if let Some(FieldValue::Float(t)) = message.get("temperature") {
            *t
        } else {
            return;
        };
        
        let humidity = if let Some(FieldValue::Float(h)) = message.get("humidity") {
            *h
        } else {
            return;
        };

        let level = if temp > 30.0 || humidity > 0.8 {
            "CRITICAL"
        } else if temp > 25.0 || humidity > 0.6 {
            "WARN"
        } else {
            "INFO"
        };

        message.insert("level".to_string(), FieldValue::String(level.to_string()));
    }

    fn apply_system_logic(&mut self, message: &mut HashMap<String, FieldValue>) {
        let cpu = if let Some(FieldValue::Float(c)) = message.get("cpu_usage") {
            *c
        } else {
            return;
        };
        
        let memory = if let Some(FieldValue::Float(m)) = message.get("memory_usage") {
            *m
        } else {
            return;
        };

        let level = if cpu > 90.0 || memory > 95.0 {
            "ERROR"
        } else if cpu > 70.0 || memory > 80.0 {
            "WARN"
        } else {
            "INFO"
        };

        message.insert("level".to_string(), FieldValue::String(level.to_string()));

        if cpu > 80.0 {
            let correlated_memory = memory + self.rng.random_range(10.0..20.0);
            message.insert("memory_usage".to_string(), FieldValue::Float(correlated_memory.min(100.0)));
        }
    }

    fn apply_timescaledb_logic(&mut self, message: &mut HashMap<String, FieldValue>) {
        if let Some(FieldValue::String(operation)) = message.get("operation") {
            let mut duration = if let Some(FieldValue::Float(d)) = message.get("query_duration") {
                *d
            } else {
                return;
            };

            if operation == "VACUUM" || operation == "ANALYZE" {
                duration *= 5.0;
                message.insert("query_duration".to_string(), FieldValue::Float(duration));
                message.insert("level".to_string(), FieldValue::String("INFO".to_string()));
            } else if operation == "DELETE" || operation == "UPDATE" {
                if self.rng.random_bool(0.1) {
                    message.insert("level".to_string(), FieldValue::String("ERROR".to_string()));
                }
            }

            if duration > 1000.0 {
                message.insert("level".to_string(), FieldValue::String("WARN".to_string()));
            }
        }
    }

    fn apply_kafka_logic(&mut self, message: &mut HashMap<String, FieldValue>) {
        let lag = if let Some(FieldValue::Integer(l)) = message.get("lag") {
            *l
        } else {
            return;
        };

        if lag > 10000 {
            message.insert("level".to_string(), FieldValue::String("ERROR".to_string()));
        } else if lag > 5000 {
            message.insert("level".to_string(), FieldValue::String("WARN".to_string()));
        }

        if let Some(FieldValue::Float(throughput)) = message.get("throughput") {
            if *throughput < 500.0 {
                message.insert("level".to_string(), FieldValue::String("WARN".to_string()));
            }
        }
    }

    fn apply_application_logic(&mut self, message: &mut HashMap<String, FieldValue>) {
        let status = if let Some(FieldValue::Integer(status)) = message.get("status_code") {
            *status
        } else {
            return;
        };
        
        let response_time = if let Some(FieldValue::Float(rt)) = message.get("response_time") {
            *rt
        } else {
            return;
        };

        let level = if status >= 500 {
            "ERROR"
        } else if status >= 400 {
            "WARN"
        } else if response_time > 1000.0 {
            "WARN"
        } else {
            "INFO"
        };

        message.insert("level".to_string(), FieldValue::String(level.to_string()));

        if status >= 400 {
            let slower_response = response_time + self.rng.random_range(100.0..500.0);
            message.insert("response_time".to_string(), FieldValue::Float(slower_response));
        }
    }
}