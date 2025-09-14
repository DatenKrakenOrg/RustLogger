use dotenv::dotenv;
use polars::prelude::*;
use polars::frame::row::Row;
use reqwest;
use reqwest::Error;
use serde::{Deserialize, Serialize};
use std::{env, f64};

/// Configuration for the log sender application.
///
/// Loads settings from environment variables:
/// - ENDLESS: Whether to run endlessly (bool)
/// - REPETITIONS: Number of times to process the log file (i32)
/// - LOGFILE_PATH: Path to the log file to read from (String)
/// - ENDPOINT: HTTP endpoint to send logs to (String)
struct Config {
    endless: bool,
    repetitions: i32,
    logfile_path: String,
    endpoint: String,
    secret: String,
}

impl Config {
    /// Loads configuration from environment variables using dotenv.
    ///
    /// Returns:
    /// - Ok(Config) if all required variables are present and valid
    /// - Err(String) with error message if any variable is missing or invalid
    fn load() -> Result<Self, String> {
        if env::var("DEPLOYMENT").unwrap_or_default() != "PROD" {
            dotenv().ok();
        }
        Ok(Self {
            endless: env::var("ENDLESS")
                .map_err(|_| "ENDLESS environment variable is missing")?
                .parse()
                .map_err(|_| "ENDLESS must be a boolean")?,
            repetitions: env::var("REPETITIONS")
                .map_err(|_| "REPETITIONS environment variable is missing")?
                .parse()
                .map_err(|_| "REPETITIONS must be an integer")?,
            logfile_path: env::var("LOGFILE_PATH")
                .map_err(|_| "LOGFILE_PATH environment variable is missing")?,
            endpoint: env::var("ENDPOINT")
                .map_err(|_| "ENDPOINT environment variable is missing")?,
            secret: env::var("SECRET_API_KEY")
                .map_err(|_| "SECRET_API_KEY environment variable is missing")?,
        })
    }
}

/// Inner message structure containing device information and exceeded threshold values.
#[derive(Serialize, Clone)]
struct InnerMsg {
    device: String,
    msg: String,
    exceeded_values: Vec<bool>,
}

/// Temporary structure to parse the JSON from CSV that matches the log generator's Message structure
#[derive(Deserialize)]
struct CsvMessage {
    device: String,  // Device enum gets serialized as string
    msg: String,
    exceeded_values: [bool; 2],  // Array from log generator
}

/// Complete log entry structure for serialization to JSON.
///
/// Represents a single log line parsed from the CSV file
#[derive(Serialize, Clone)]
struct LogEntry {
    timestamp: String, // Use String if the timestamp is coming as a string from `data.next()`
    level: String,
    temperature: f64,
    humidity: f64,
    msg: InnerMsg,
}

/// Main application entry point.
///
/// Loads configuration, reads and parses the CSV file once, then either runs endlessly 
/// or for a specified number of repetitions, sending the same log entries each time.
/// This approach optimizes performance by avoiding repeated CSV parsing.
#[tokio::main]
async fn main() {
    let config = Config::load().expect("Failed to load environment variables");

    let log_entries = process_file(&config);

    if config.endless {
        loop {
            process_log_entries(&config, &log_entries).await;
        }
    } else {
        for _n in 0..config.repetitions {
            process_log_entries(&config, &log_entries).await;
        }
    }
}

/// Reads and parses the entire log file into LogEntry structs.
///
/// Uses Polars to properly parse CSV data including escaped quotes in JSON fields.
/// Returns a vector of LogEntry structs that can be reused for multiple sends,
/// avoiding the need to re-parse the CSV file on each iteration.
///
/// # Arguments
/// * `config` - Configuration containing file path
///
/// # Returns
/// * `Vec<LogEntry>` - Vector of parsed log entries ready for sending
fn process_file(config: &Config) -> Vec<LogEntry> {
    
    // Read CSV using Polars with proper escaping handling
    let df = CsvReader::from_path(&config.logfile_path)
        .expect("Failed to open CSV file")
        .has_header(true)
        .finish()
        .expect("Failed to read CSV file");
    
    // Process all rows into LogEntry structs first
    let mut log_entries = Vec::new();
    for i in 0..df.height() {
        let row = df.get_row(i).expect("Failed to get row");
        let log_entry = create_log_entry(row);
        log_entries.push(log_entry);
    }

    return log_entries;
    
}

/// Sends all log entries to the configured HTTP endpoint.
///
/// Creates an HTTP client and sends each log entry sequentially to the endpoint.
/// This function can be called multiple times with the same log entries for
/// repeated sending scenarios (endless mode or multiple repetitions).
///
/// # Arguments
/// * `config` - Configuration containing endpoint URL and API secret
/// * `log_entries` - Vector of pre-created LogEntry structs to send
async fn process_log_entries(config: &Config, log_entries: &Vec<LogEntry>) {
    let client = reqwest::Client::new();

    // Then send each log entry
    for log_entry in log_entries {
        send_value(&client, &config.endpoint, &config.secret, log_entry.clone())
            .await
            .expect("Failed to establish a connection")
    }
}

/// Sends a single log entry to the HTTP endpoint.
///
/// Serializes the LogEntry to JSON and sends it via POST.
/// Prints the response status. Handles HTTP errors gracefully.
///
/// # Arguments
/// * `client` - HTTP client for making requests
/// * `endpoint` - URL to send the log entry to
/// * `secret` - API secret key for authentication
/// * `log_entry` - Pre-created LogEntry ready for sending
///
/// # Returns
/// * `Result<(), Error>` - Ok if successful, Error if HTTP request fails
async fn send_value(client: &reqwest::Client, endpoint: &str, secret: &str, log_entry: LogEntry) -> Result<(), Error> {
    let res = client.post(endpoint).header("X-Api-Key", secret).json(&log_entry).send().await?;

    println!("{}", res.status());

    match res.error_for_status() {
        Ok(_) => (),
        Err(err) => {
            println!("{}", err.to_string());
        }
    }

    Ok(())
}

/// Creates a LogEntry from Polars Row data.
///
/// Expects CSV data in the format: timestamp,level,temperature,humidity,msg
/// where msg is a JSON string created by the log generator
///
/// # Arguments
/// * `row` - Polars Row containing CSV fields
///
/// # Returns
/// * `LogEntry` - Structured log entry ready for serialization
fn create_log_entry(row: Row<'_>) -> LogEntry {
    let timestamp = row.0[0].get_str().expect("Failed to get timestamp").to_string();
    let level = row.0[1].get_str().expect("Failed to get level").to_string();
    let temperature = row.0[2].try_extract::<f64>().expect("Failed to parse temperature");
    let humidity = row.0[3].try_extract::<f64>().expect("Failed to parse humidity");
    
    // The last field is the JSON-serialized Message struct
    let msg_json = row.0[4].get_str().expect("Failed to get msg");
    let msg: InnerMsg = parse_message_json(msg_json);

    LogEntry {
        timestamp,
        level,
        temperature,
        humidity,
        msg,
    }
}

/// Parses a JSON string from CSV into InnerMsg.
///
/// The log generator serializes Message structs to JSON strings in the CSV.
/// This function deserializes that JSON and converts it to the InnerMsg format expected by the API.
/// Handles CSV-escaped JSON by unescaping double quotes before parsing.
///
/// # Arguments
/// * `msg_json` - JSON string from CSV containing the serialized Message (may be CSV-escaped)
///
/// # Returns
/// * `InnerMsg` - Message structure with device info and exceeded threshold flags
fn parse_message_json(msg_json: &str) -> InnerMsg {
    // Handle CSV-escaped JSON by unescaping double quotes
    let unescaped_json = msg_json.replace("\"\"", "\"");
    
    match serde_json::from_str::<CsvMessage>(&unescaped_json) {
        Ok(csv_msg) => InnerMsg {
            device: csv_msg.device,
            msg: csv_msg.msg,
            exceeded_values: csv_msg.exceeded_values.to_vec(), // Convert [bool; 2] to Vec<bool>
        },
        Err(e) => {
            eprintln!("Failed to parse message JSON '{}': {}", unescaped_json, e);
            // Fallback to default values
            InnerMsg {
                device: "Unknown".to_string(),
                msg: "Failed to parse message".to_string(),
                exceeded_values: vec![false, false],
            }
        }
    }
}
