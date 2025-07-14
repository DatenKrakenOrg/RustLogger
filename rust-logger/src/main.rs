mod log_collector;
mod log_generator;
mod logging_types;
use clap::Parser;
use log_collector::{memory_optimized_df_collector, runtime_optimized_df_collector};
use log_generator::log_gen::LogGen;
use polars::{frame::DataFrame, io::SerWriter, prelude::CsvWriter};
use std::{fs::File, path::PathBuf};

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
    #[arg(short, long, default_value_t = std::path::Path::new(&std::env::current_dir().unwrap())
    .join("log_gen_output.csv")
    .to_str()
    .unwrap()
    .to_string())]
    path: String,
}

fn main() {
    let args = Args::parse();
    let log_gen = LogGen::new(args.count, (args.start_year, args.end_year)).expect("Error on log generation");
    let mut collected_df: DataFrame;

    if args.memory_optimized {
        collected_df = memory_optimized_df_collector(log_gen);
    } else {
        collected_df = runtime_optimized_df_collector(log_gen);
    }

    // Save DataFrame to CSV if csv already exists, append index to filename
    let mut file_path = PathBuf::from(&args.path);
    if !("csv" == file_path.extension().unwrap()) {
        panic!("Path must end with .csv: {}", file_path.display());
    }

    let mut index = 1;
    while file_path.exists() {
        file_path.pop();
        index += 1;
        file_path.push(format!("log_gen_output_{index}.csv"));
    }

    let mut file = File::create(file_path).expect("Could not create blank csv file!");

    CsvWriter::new(&mut file)
        .include_header(true)
        .with_separator(b',')
        .finish(&mut collected_df)
        .expect("Could not create csv file from dataframe!");
}
