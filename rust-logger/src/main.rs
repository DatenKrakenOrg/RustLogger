use chrono::prelude::*;
use polars::prelude::*;
mod logging_types;
use logging_types::log_types::{Level, Device, Message, Measurement};


fn main() {
// Erstelle leere Series für jede gewünschte Spalte
    // let s_timestamp = Series::new_empty("timestamp", &DataType::Datetime);
    // let s_level= Series::new_empty("level", Level);
    // let s_msg = Series::new_empty("temperature".into(), &DataType::Float32);
    // let s_meta = Series::new_empty("meta", Meta);
    // Kombiniere die leeren Series zu einem DataFrame


    
    let _df: DataFrame = df!(
        "timestamp" => [
            NaiveDate::from_ymd_opt(2025, 07, 07).unwrap().and_hms_opt(10, 55, 00).unwrap(),
        ],
        "level" => [Level::WARN.to_string()],
        "temperature" => [30.2],
        "humidity" => [0.9],
        "msg" => [serde_json::to_string(
            &Message{
                device: Device::Arduino0,
                msg: String::from("Temperature and Humidity exceeded!"),
                exceeded_values: vec![Measurement::Temperature, Measurement::Humidity],
            }
        ).unwrap()],
    )
    .unwrap();

    println!("{}", _df);
}
