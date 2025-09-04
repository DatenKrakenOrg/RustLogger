mod config;
mod message_generator;
mod utility;
use clap::Parser;
use config::{MessageTypesConfig, FieldValue};
use message_generator::MessageGenerator;

use std::{path::PathBuf, collections::HashMap};
use polars::prelude::*;
use utility::default_path;

/// CLI Arguments to Parse via clap refer to documentation of clap for more information.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Number of logs to create.
    #[arg(short, long, default_value_t = 100000)]
    count: usize,
    /// Start year => start_year <= x < end_year
    #[arg(short, long, default_value_t = 2025)]
    start_year: i32,
    /// End year => start_year <= x < end_year. Must be greater than start year in order to not panic the program.
    #[arg(short, long, default_value_t = 2026)]
    end_year: i32,
    /// Use memory optimization instead of runtime optimized version.
    #[arg(short, long, default_value_t = false)]
    memory_optimized: bool,
    /// Path to save csv to.
    #[arg(short, long, default_value_t = default_path())]
    path: String,
    /// Message types to generate (comma-separated). If not specified, generates all types.
    #[arg(short, long)]
    types: Option<String>,
    /// Path to message types configuration file.
    #[arg(long, default_value = "message_types.toml")]
    config_path: String,
}

/// Main entry point of the application.
/// Parses command line arguments, loads configuration, and generates log data
/// for the specified message types, saving each type to its own CSV file.
fn main() {
    env_logger::init();
    // Parse command line arguments
    let args = Args::parse();

    // Load message type configuration from TOML file
    let config = MessageTypesConfig::load_from_file(&PathBuf::from(&args.config_path))
        .expect("Failed to load message types configuration");

    // Determine which message types to generate:
    // Either from command line argument (comma-separated) or all available types
    let selected_types: Vec<String> = if let Some(types_str) = &args.types {
        types_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        config.list_types().into_iter().cloned().collect()
    };

    // Determine the output directory for CSV files
    let base_path = PathBuf::from(&args.path);
    let default_dir = PathBuf::from(".");
    let parent_dir = base_path.parent().unwrap_or(&default_dir);

    // Generate logs for each selected message type
    for message_type in &selected_types {
        if let Some(type_config) = config.get_type(message_type) {
            log::info!("Generating {} logs for message type: {}", args.count, message_type);
            
            // Create a message generator for this specific type
            let mut generator = MessageGenerator::new(
                type_config.clone(), 
                (args.start_year, args.end_year)
            ).expect("Failed to create message generator");

            // Generate the specified number of log messages
            let mut logs = Vec::new();
            for _ in 0..args.count {
                logs.push(generator.generate_message());
            }

            // Save generated logs to CSV file named after the message type
            let file_path = parent_dir.join(format!("{}.csv", message_type));
            save_logs_to_csv(&logs, &file_path, type_config.fields.keys().collect())
                .expect("Failed to save logs to CSV");
            
            log::info!("Saved {} logs to: {}", args.count, file_path.display());
        } else {
            log::error!("Unknown message type: {}", message_type);
        }
    }
}

/// Saves log data to a CSV file using Polars DataFrame.
/// 
/// # Arguments
/// * `logs` - Vector of log entries as HashMaps with field names and values
/// * `file_path` - Path where the CSV file should be saved
/// * `field_order` - Order in which fields should appear as columns in the CSV
/// 
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Success or error details
fn save_logs_to_csv(
    logs: &[HashMap<String, FieldValue>], 
    file_path: &PathBuf, 
    field_order: Vec<&String>
) -> Result<(), Box<dyn std::error::Error>> {
    // Early return if no logs to process
    if logs.is_empty() {
        return Ok(());
    }

    // Print header for console output
    let header = field_order.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(",");
    log::debug!("CSV Header: {}", header);

    // Create columns for DataFrame
    let mut columns: Vec<Column> = Vec::new();

    // Create a Polars Series for each field in the specified order
    for field_name in &field_order {
        // Extract values for this field from all log entries
        let values: Vec<String> = logs
            .iter()
            .map(|log| {
                log.get(*field_name)
                    .map(|v| v.to_csv_string())
                    .unwrap_or_else(|| "".to_string())
            })
            .collect();

        // Create a Series (column) and add to the columns vector
        let series = Series::new((*field_name).into(), values);
        columns.push(series.into());
    }

    // Create DataFrame from columns and write to CSV file
    let df = DataFrame::new(columns)?;
    
    let mut file = std::fs::File::create(file_path)?;
    CsvWriter::new(&mut file)
        .include_header(true)
        .finish(&mut df.clone())?;
    
    Ok(())
}
