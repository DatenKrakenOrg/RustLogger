mod config;
mod message_generator;
mod log_collector;
mod log_generator;
mod logging_types;
mod utility;
use clap::Parser;
use config::{MessageTypesConfig, FieldValue};
use message_generator::MessageGenerator;

use std::{fs::File, path::PathBuf, collections::HashMap};
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

fn main() {
    let args = Args::parse();
    
    let config = MessageTypesConfig::load_from_file(&PathBuf::from(&args.config_path))
        .expect("Failed to load message types configuration");

    let selected_types: Vec<String> = if let Some(types_str) = &args.types {
        types_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        config.list_types().into_iter().cloned().collect()
    };

    let base_path = PathBuf::from(&args.path);
    let default_dir = PathBuf::from(".");
    let parent_dir = base_path.parent().unwrap_or(&default_dir);

    for message_type in &selected_types {
        if let Some(type_config) = config.get_type(message_type) {
            println!("Generating {} logs for message type: {}", args.count, message_type);
            
            let mut generator = MessageGenerator::new(
                type_config.clone(), 
                (args.start_year, args.end_year)
            ).expect("Failed to create message generator");

            let mut logs = Vec::new();
            for _ in 0..args.count {
                logs.push(generator.generate_message());
            }

            let file_path = parent_dir.join(format!("{}.csv", message_type));
            save_logs_to_csv(&logs, &file_path, type_config.fields.keys().collect())
                .expect("Failed to save logs to CSV");
            
            println!("Saved {} logs to: {}", args.count, file_path.display());
        } else {
            eprintln!("Unknown message type: {}", message_type);
        }
    }
}

fn save_logs_to_csv(
    logs: &[HashMap<String, FieldValue>], 
    file_path: &PathBuf, 
    field_order: Vec<&String>
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(file_path)?;
    
    use std::io::Write;
    
    // Write header
    let header = field_order.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(",");
    writeln!(file, "{}", header)?;
    
    // Write data rows
    for log in logs {
        let row: Vec<String> = field_order
            .iter()
            .map(|field| {
                log.get(*field)
                    .map(|v| v.to_csv_string())
                    .unwrap_or_else(|| "".to_string())
            })
            .collect();
        writeln!(file, "{}", row.join(","))?;
    }
    
    Ok(())
}
