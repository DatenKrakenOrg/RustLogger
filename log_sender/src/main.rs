use dotenv::dotenv;
use reqwest;
use reqwest::Error;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use rand::prelude::*;
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
    logs_directory: String,
    endpoint: String,
    secret: String,
    config_path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MessageTypeConfig {
    pub name: String,
    pub index_name: String,
    pub description: String,
    pub regex_pattern: String,
    pub fields: HashMap<String, toml::Value>,
    pub logic: Option<HashMap<String, toml::Value>>,
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    message_types: Vec<MessageTypeConfig>,
}

#[derive(Serialize)]
struct LogPayload {
    message_type: String,
    csv_line: String,
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
            logs_directory: env::var("LOGS_DIRECTORY")
                .unwrap_or_else(|_| "./".to_string()),
            endpoint: env::var("ENDPOINT")
                .map_err(|_| "ENDPOINT environment variable is missing")?,
            secret: env::var("SECRET_API_KEY")
                .map_err(|_| "SECRET_API_KEY environment variable is missing")?,
            config_path: env::var("CONFIG_PATH")
                .unwrap_or_else(|_| "message_types.toml".to_string()),
        })
    }
}



/// Main application entry point.
///
/// Loads configuration and either runs endlessly or for a specified number of repetitions,
/// processing the log file each time.
#[tokio::main]
async fn main() {
    let config = Config::load().expect("Failed to load environment variables");
    
    // Load message types configuration
    let message_config = load_message_types(&config.config_path)
        .expect("Failed to load message types configuration");
    
    // Randomly select a message type
    let selected_type = select_random_message_type(&message_config)
        .expect("No message types available");
    
    println!("Selected message type: {} - {}", selected_type.name, selected_type.description);
    
    // Look for CSV file of that type
    let csv_file_path = PathBuf::from(&config.logs_directory).join(format!("{}.csv", selected_type.name));
    
    if !csv_file_path.exists() {
        eprintln!("CSV file not found: {}", csv_file_path.display());
        std::process::exit(1);
    }

    if config.endless {
        loop {
            process_file(&config, &selected_type, &csv_file_path).await;
        }
    } else {
        for _n in 0..config.repetitions {
            process_file(&config, &selected_type, &csv_file_path).await;
        }
    }
}

fn load_message_types(config_path: &str) -> Result<Vec<MessageTypeConfig>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(config_path)?;
    let config_file: ConfigFile = toml::from_str(&content)?;
    Ok(config_file.message_types)
}

fn select_random_message_type(message_types: &[MessageTypeConfig]) -> Option<MessageTypeConfig> {
    let mut rng = rand::rng();
    message_types.choose(&mut rng).cloned()
}

/// Processes the entire log file by reading each line and sending it to the endpoint.
///
/// Creates an HTTP client, reads the log file, skips the first line (header),
/// and sends each subsequent line to the configured endpoint.
///
/// # Arguments
/// * `config` - Configuration containing file path and endpoint URL
async fn process_file(config: &Config, message_type: &MessageTypeConfig, csv_file_path: &PathBuf) {
    let client = reqwest::Client::new();
    let mut lines = read_lines(csv_file_path).unwrap();
    lines.next(); // Skip header
    
    for line in lines.map_while(Result::ok) {
        send_log(&client, &config.endpoint,&config.secret, &message_type.name, line)
            .await
            .expect("Failed to establish a connection")
    }
}

/// Reads lines from a file and returns an iterator over them.
///
/// # Arguments
/// * `filename` - Path to the file to read
///
/// # Returns
/// * `io::Result<io::Lines<io::BufReader<File>>>` - Iterator over file lines or IO error
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

/// Sends a single log line to the HTTP endpoint.
///
/// Parses the CSV line into a LogEntry, serializes it to JSON, and sends it via POST.
/// Prints the original line and response status. Handles HTTP errors gracefully.
///
/// # Arguments
/// * `client` - HTTP client for making requests
/// * `endpoint` - URL to send the log entry to
/// * `line` - CSV line to parse and send
///
/// # Returns
/// * `Result<(), Error>` - Ok if successful, Error if HTTP request fails
async fn send_log(
    client: &reqwest::Client, 
    endpoint: &str,secret:&str, 
    message_type: &str, 
    csv_line: String
) -> Result<(), Error> {
    println!("Sending {} log: {}", message_type, csv_line);

    let payload = LogPayload {
        message_type: message_type.to_string(),
        csv_line: csv_line.clone(),
    };

    let res = client.post(endpoint).header("X-Api-Key", secret).json(&payload).send().await?;

    println!("Response: {}", res.status());

    match res.error_for_status() {
        Ok(_) => (),
        Err(err) => {
            println!("Error: {}", err.to_string());
        }
    }

    Ok(())
}


