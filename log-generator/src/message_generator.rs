use crate::config::{FieldConfig, FieldValue, MessageTypeConfig};
use chrono::{Duration, NaiveDate, NaiveDateTime, SecondsFormat, TimeZone, Utc};
use rand::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

/// Generates realistic log messages based on message type configuration.
/// Handles different field types, applies contextual logic, and maintains randomness
/// within specified constraints.
pub struct MessageGenerator {
    /// Configuration defining the message type structure and constraints
    config: MessageTypeConfig,
    /// Date range for timestamp generation (start_date, end_date)
    years: (NaiveDate, NaiveDate),
    /// Random number generator for value generation
    rng: rand::rngs::ThreadRng,
}

impl MessageGenerator {
    /// Creates a new message generator for a specific message type.
    /// 
    /// # Arguments
    /// * `config` - Message type configuration with field definitions
    /// * `years` - Year range as (start_year, end_year) for timestamp generation
    /// 
    /// # Returns
    /// * `Result<Self, String>` - New generator instance or error message
    /// 
    /// # Errors
    /// * Returns error if end_year <= start_year
    pub fn new(config: MessageTypeConfig, years: (i32, i32)) -> Result<Self, String> {
        // Validate year range
        if years.1 - years.0 <= 0 {
            return Err("Year range invalid: end_year must be greater than start_year".to_string());
        }

        Ok(Self {
            config,
            // Convert years to NaiveDate objects for easier date calculations
            years: (
                NaiveDate::from_yo_opt(years.0, 1).unwrap(),
                NaiveDate::from_yo_opt(years.1, 1).unwrap(),
            ),
            rng: rand::rng(),
        })
    }

    /// Generates a single log message with all fields populated.
    /// 
    /// # Returns
    /// * `HashMap<String, FieldValue>` - Complete log message with field names and values
    /// 
    /// # Panics
    /// * Panics if the message type is configured with `generate = false`
    pub fn generate_message(&mut self) -> HashMap<String, FieldValue> {
        // Safety check to prevent generation of non-generatable message types
        if !self.config.generate.unwrap_or(true) {
            panic!("This message type is not configured for generation.");
        }
        
        let mut message = HashMap::new();
        let fields = self.config.fields.clone();

        // Generate values for each field according to its configuration
        for (field_name, field_config) in fields {
            let value = self.generate_field_value(&field_name, &field_config, &message);
            message.insert(field_name, value);
        }

        // Apply message-type-specific logic to correlate fields
        self.apply_contextual_logic(&mut message);
        message
    }

    /// Generates a value for a single field based on its type and configuration.
    /// 
    /// # Arguments
    /// * `_field_name` - Name of the field (currently unused, reserved for future features)
    /// * `config` - Field configuration specifying type, constraints, etc.
    /// * `_existing_values` - Already generated values (for potential field correlations)
    /// 
    /// # Returns
    /// * `FieldValue` - Generated value appropriate for the field type
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

    /// Generates a random datetime within the configured year range.
    /// 
    /// # Returns
    /// * `FieldValue::DateTime` - RFC3339 formatted datetime string with milliseconds
    fn generate_datetime(&mut self) -> FieldValue {
        // Calculate random date within the year range
        let days_in_range = (self.years.1 - self.years.0).num_days();
        let random_days: i64 = self.rng.random_range(0..days_in_range);
        
        // Generate random time components
        let naive: NaiveDateTime = (self.years.0 + Duration::days(random_days))
            .and_hms_opt(
                self.rng.random_range(0..24),
                self.rng.random_range(0..60),
                self.rng.random_range(0..60),
            )
            .unwrap()
            .into();
            
        // Convert to UTC and format as RFC3339 with milliseconds
        let timestamp = Utc.from_utc_datetime(&naive).to_rfc3339_opts(SecondsFormat::Millis, true);
        FieldValue::DateTime(timestamp)
    }

    /// Generates a value by randomly selecting from predefined enum values.
    /// 
    /// # Arguments
    /// * `config` - Field configuration containing the list of possible values
    /// 
    /// # Returns
    /// * `FieldValue::String` - Randomly selected value from the enum list
    fn generate_enum(&mut self, config: &FieldConfig) -> FieldValue {
        if let Some(values) = &config.values {
            let value = values.choose(&mut self.rng).unwrap().clone();
            FieldValue::String(value)
        } else {
            FieldValue::String("default".to_string())
        }
    }

    /// Generates string values using patterns and templates.
    /// Supports various placeholder patterns like {id}, {prefix}, {env}.{service}, etc.
    /// 
    /// # Arguments
    /// * `config` - Field configuration with pattern and values
    /// 
    /// # Returns
    /// * `FieldValue::String` - Generated string based on pattern or direct value selection
    fn generate_string(&mut self, config: &FieldConfig) -> FieldValue {
        if let Some(pattern) = &config.pattern {
            if pattern.contains("{env}") && pattern.contains("{service}") {
                if let Some(values) = &config.values {
                    let full_value = values.choose(&mut self.rng).unwrap();
                    FieldValue::String(full_value.clone())
                } else {
                    FieldValue::String("prod.service".to_string())
                }
            } else if pattern.contains("{prefix}") {
                if let Some(values) = &config.values {
                    let prefix = values.choose(&mut self.rng).unwrap();
                    let result = pattern.replace("{prefix}", prefix);
                    FieldValue::String(result)
                } else {
                    FieldValue::String("default_table".to_string())
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
            } else if let Some(values) = &config.values {
                let base_value = values.choose(&mut self.rng).unwrap();
                let result = pattern.replace("{id}", base_value);
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

    /// Generates a random floating point number within the specified range.
    /// 
    /// # Arguments
    /// * `config` - Field configuration with range [min, max]
    /// 
    /// # Returns
    /// * `FieldValue::Float` - Random float within range, or 0.0 if no range specified
    fn generate_float(&mut self, config: &FieldConfig) -> FieldValue {
        if let Some(range) = &config.range {
            let value = self.rng.random_range(range[0]..range[1]);
            FieldValue::Float(value)
        } else {
            FieldValue::Float(0.0)
        }
    }

    /// Generates a random integer within the specified range or from predefined values.
    /// 
    /// # Arguments
    /// * `config` - Field configuration with either range [min, max] or list of values
    /// 
    /// # Returns
    /// * `FieldValue::Integer` - Random integer within constraints, or 0 as fallback
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

    /// Applies message type specific logic to correlate fields and create realistic data.
    /// This includes setting severity levels based on metric values, correlating related fields,
    /// and applying domain-specific rules.
    /// 
    /// # Arguments
    /// * `message` - Mutable reference to the generated message to modify
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

    /// Applies IoT sensor specific logic to set severity levels based on environmental readings.
    /// 
    /// # Arguments
    /// * `message` - Message to modify with IoT-specific correlations
    fn apply_iot_logic(&mut self, message: &mut HashMap<String, FieldValue>) {
        // Extract temperature and humidity values
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

        // Set severity level based on environmental thresholds
        let level = if temp > 30.0 || humidity > 0.8 {
            "CRITICAL"
        } else if temp > 25.0 || humidity > 0.6 {
            "WARN"
        } else {
            "INFO"
        };

        message.insert("level".to_string(), FieldValue::String(level.to_string()));
    }

    /// Applies system metrics logic to correlate CPU and memory usage with severity levels.
    /// High resource usage triggers higher severity and correlates CPU/memory values.
    /// 
    /// # Arguments
    /// * `message` - Message to modify with system-specific correlations
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

    /// Applies database operation logic to correlate query types with performance and errors.
    /// Adjusts query duration for maintenance operations and applies error probability.
    /// 
    /// # Arguments
    /// * `message` - Message to modify with database-specific correlations
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

    /// Applies Kafka message streaming logic to correlate consumer lag with severity levels.
    /// High lag and low throughput trigger warnings and errors.
    /// 
    /// # Arguments
    /// * `message` - Message to modify with Kafka-specific correlations
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

    /// Applies application log logic to correlate HTTP status codes with response times.
    /// Error status codes trigger slower response times and appropriate severity levels.
    /// 
    /// # Arguments
    /// * `message` - Message to modify with application-specific correlations
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
