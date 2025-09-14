use chrono::{NaiveDateTime};
use polars::prelude::*;
use serde_json::to_string;

use crate::{log_generator::log_gen::LogGen, logging_types::log_types::Log};

/// Returns a Dataframe containing logs with timestamps row-wise by a runtime-optimized algorithm
///
/// # Examples
/// ```
/// let log_gen = LogGen::new(args.count, (args.start_year, args.end_year)).expect("Error on log");
/// let mut collected_df: Dataframe = runtime_optimized_df_collector(log_gen);
/// ´´´
pub fn runtime_optimized_df_collector(log_gen: LogGen) -> DataFrame {
    // Dataframes to concatenate via lazy optimiter after loop
    let mut chunked_lazy_df = Vec::new();

    // Collect all rows in seperate dataframes at once in order to make use of the lazyframe optimizer
    for chunk in log_gen.collect::<Vec<Log>>().chunks(1000) {
        // Extract each datapoint for column-wise alignment
        let timestamps: Vec<&str> = chunk.iter().map(|log| log.timestamp.as_str()).collect();
        let levels: Vec<String> = chunk.iter().map(|log| log.level.to_string()).collect();
        let temperatures: Vec<f32> = chunk.iter().map(|log| log.temperatur).collect();
        let humidities: Vec<f32> = chunk.iter().map(|log| log.humidity).collect();
        let msgs: Vec<String> = chunk
            .iter()
            .map(|log| to_string(&log.msg).unwrap())
            .collect();

        // Collect lazyframe of the current chunk
        let chunk_lazy_df = DataFrame::new(vec![
            Series::new("timestamp".into(), timestamps).into(),
            Series::new("level".into(), levels).into(),
            Series::new("temperature".into(), temperatures).into(),
            Series::new("humidity".into(), humidities).into(),
            Series::new("msg".into(), msgs).into(),
        ])
        .unwrap()
        .lazy();

        chunked_lazy_df.push(chunk_lazy_df)
    }

    // Concatenate Lazyframes and materialize them
    let df = concat(chunked_lazy_df, UnionArgs::default())
        .expect("Failed to concatenate LazyFrames")
        .collect()
        .expect("Failed to collect DataFrame from LazyFrames");
    return df;
}

/// Returns a Dataframe containing logs with timestamps row-wise by a memory-optimized algorithm.
///
/// # Examples
/// ```
/// let log_gen = LogGen::new(args.count, (args.start_year, args.end_year)).expect("Error on log");
/// let mut collected_df: Dataframe = runtime_optimized_df_collector(log_gen);
/// ´´´
pub fn memory_optimized_df_collector(log_gen: LogGen) -> DataFrame {
    // Initialize LazyFrame with needed schema
    let mut df = DataFrame::new(vec![
        Series::new("timestamp".into(), Vec::<NaiveDateTime>::new()).into(),
        Series::new("level".into(), Vec::<String>::new()).into(),
        Series::new("temperature".into(), Vec::<f32>::new()).into(),
        Series::new("humidity".into(), Vec::<f32>::new()).into(),
        Series::new("msg".into(), Vec::<String>::new()).into(),
    ])
    .unwrap()
    .lazy();

    // Convert each chunk to lazyframe and concatenate them in each iteration => This does not collect all lazyframes before concat
    for chunk in log_gen.collect::<Vec<Log>>().chunks(1000) {
        // Extract each datapoint for column-wise alignment
        let timestamps: Vec<&str> = chunk.iter().map(|log| log.timestamp.as_str()).collect();
        let levels: Vec<String> = chunk.iter().map(|log| log.level.to_string()).collect();
        let temperatures: Vec<f32> = chunk.iter().map(|log| log.temperatur).collect();
        let humidities: Vec<f32> = chunk.iter().map(|log| log.humidity).collect();
        let msgs: Vec<String> = chunk
            .iter()
            .map(|log| to_string(&log.msg).unwrap())
            .collect();

        // Create lazyframe for current chunk
        let chunk_df = DataFrame::new(vec![
            Series::new("timestamp".into(), timestamps).into(),
            Series::new("level".into(), levels).into(),
            Series::new("temperature".into(), temperatures).into(),
            Series::new("humidity".into(), humidities).into(),
            Series::new("msg".into(), msgs).into(),
        ])
        .unwrap()
        .lazy();

        // Concatenate Lazyframes
        df = concat([df, chunk_df], UnionArgs::default()).unwrap()
    }

    let materialized_df = df.collect().expect("Failed to collect LazyFrame");
    return materialized_df;
}
