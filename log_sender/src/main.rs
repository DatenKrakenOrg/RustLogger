use dotenv::dotenv;
use reqwest;
use reqwest::Error;
use serde::Serialize;
use serde_json;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::{env, f64};

struct Config {
    endless: bool,
    repetitions: i32,
    logfile_path: String,
    endpoint: String,
}

impl Config {
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
        })
    }
}

#[derive(Serialize)]
struct InnerMsg {
    device: String,
    msg: String,
    exceeded_values: Vec<bool>,
}

#[derive(Serialize)]
struct LogEntry {
    timestamp: String, // Use String if the timestamp is coming as a string from `data.next()`
    level: String,
    temperature: f64,
    humidity: f64,
    msg: InnerMsg,
}

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

async fn process_file(config: &Config) {
    let client = reqwest::Client::new();
    // File hosts.txt must exist in the current path
    let mut lines = read_lines(&config.logfile_path).unwrap();
    lines.next();
    // Consumes the iterator, returns an (Optional) String
    for line in lines.map_while(Result::ok) {
        send_value(&client, &config.endpoint, line)
            .await
            .expect("blöd gelaufen")
    }
}
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
async fn send_value(client: &reqwest::Client, endpoint: &str, line: String) -> Result<(), Error> {
    let mut data = line.split(",");

    println!("{}", line);

    let log_entry = create_log_entry(&mut data);

    let res = client.post(endpoint).json(&log_entry).send().await?;

    println!("{}", res.status());

    match res.error_for_status() {
        Ok(_) => (),
        Err(err) => {
            println!("{}", err.to_string());
        }
    }

    //match res {
    //    Ok(response) => println!("sending succeeded with code {}", response.status()),
    //    Err(error) => println!("{}", error.to_string()),
    //}
    //
    Ok(())
}

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
