use dotenv::dotenv;
use reqwest;
use reqwest::Error;
use serde::Serialize;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
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
        dotenv().ok(); // Load .env file
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
#[derive(Serialize)]
struct InnerMsg {
    device: String,
    msg: String,
    exceeded_values: Vec<bool>,
}

/// Complete log entry structure for serialization to JSON.
///
/// Represents a single log line parsed from the CSV file
#[derive(Serialize)]
struct LogEntry {
    timestamp: String, // Use String if the timestamp is coming as a string from `data.next()`
    level: String,
    temperature: f64,
    humidity: f64,
    msg: InnerMsg,
}

/// Main application entry point.
///
/// Loads configuration and either runs endlessly or for a specified number of repetitions,
/// processing the log file each time.
#[tokio::main]
async fn main() {
    let config = Config::load().expect("Failed to load environment variables");

    if config.endless {
        loop {
            process_file(&config).await;
        }
    } else {
        for _n in 0..config.repetitions {
            process_file(&config).await;
        }
    }
}

/// Processes the entire log file by reading each line and sending it to the endpoint.
///
/// Creates an HTTP client, reads the log file, skips the first line (header),
/// and sends each subsequent line to the configured endpoint.
///
/// # Arguments
/// * `config` - Configuration containing file path and endpoint URL
async fn process_file(config: &Config) {
    let client = reqwest::Client::new();
    let mut lines = read_lines(&config.logfile_path).unwrap();
    lines.next();
    // Consumes the iterator, returns an (Optional) String
    for line in lines.map_while(Result::ok) {
        send_value(&client, &config.endpoint,&config.secret, line)
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
async fn send_value(client: &reqwest::Client, endpoint: &str,secret:&str, line: String) -> Result<(), Error> {
    let mut data = line.split(",");

    println!("{}", line);

    let log_entry = create_log_entry(&mut data);

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

/// Creates a LogEntry from parsed CSV data.
///
/// Expects CSV data in the format: timestamp,level,humidity,temperature,device,msg,exceeded_values...
/// Parses each field according to its expected type and constructs a complete LogEntry.
///
/// # Arguments
/// * `data` - Iterator over CSV fields from a single line
///
/// # Returns
/// * `LogEntry` - Structured log entry ready for serialization
fn create_log_entry<'a>(data: &mut impl Iterator<Item = &'a str>) -> LogEntry {
    let timestamp = data.next().unwrap().to_string();
    let level = data.next().unwrap().to_string();
    let humidity: f64 = data
        .next()
        .unwrap()
        .parse()
        .expect("Failed to parse humidity");
    let temperature: f64 = data
        .next()
        .unwrap()
        .parse()
        .expect("Failed to parse temperature");
    let message_parts: Vec<&str> = data.collect();
    let msg: InnerMsg = get_message(&message_parts);

    LogEntry {
        timestamp,
        level,
        temperature,
        humidity,
        msg,
    }
}

/// Creates an InnerMsg from remaining CSV fields.
///
/// Parses the remaining CSV fields into device name, message text, and boolean exceeded values.
/// Provides default values for missing fields to handle malformed CSV gracefully.
///
/// # Arguments
/// * `data_collection` - Slice of remaining CSV fields after timestamp, level, humidity, temperature
///
/// # Returns
/// * `InnerMsg` - Message structure with device info and exceeded threshold flags
fn get_message(data_collection: &[&str]) -> InnerMsg {
    let mut data = data_collection.iter();

    // Extract the device name
    let device = data.next().unwrap_or(&"Unknown").to_string();

    // Extract the message
    let msg = data.next().unwrap_or(&"No message").to_string();

    // Extract exceeded values as booleans
    let exceeded_values: Vec<bool> = data
        .map(|&part| part.parse::<bool>().unwrap_or(false)) // Parse remaining parts as booleans
        .collect();

    // Create and return the InnerMsg instance
    InnerMsg {
        device,
        msg,
        exceeded_values,
    }
}
