pub mod log_gen {
    use crate::logging_types::log_types::{Device, Level, Log, Message};
    use chrono::{Duration, NaiveDate, NaiveDateTime};
    use rand::prelude::*;

    /// Creates a log generator used as iterator to generate random chunks of datapoints.
    ///
    /// # Format of Logs
    /// ┌────────────┬──────────┬─────────────┬──────────┬─────────────────────────────────┐
    /// │ timestamp  ┆ level    ┆ temperature ┆ humidity ┆ msg                             │
    /// │ ---        ┆ ---      ┆ ---         ┆ ---      ┆ ---                             │
    /// │ date       ┆ str      ┆ f32         ┆ f32      ┆ str                             │
    /// ╞════════════╪══════════╪═════════════╪══════════╪═════════════════════════════════╡
    /// │ 2025-01-13 ┆ CRITICAL ┆ 33.149925   ┆ 0.920691 ┆ {"device":"Arduino0","msg":"CR… │
    /// │ 2025-06-18 ┆ INFO     ┆ 18.594086   ┆ 0.545695 ┆ {"device":"Arduino2","msg":"IN… │
    /// │ 2025-11-29 ┆ CRITICAL ┆ 31.061283   ┆ 0.150918 ┆ {"device":"Arduino0","msg":"CR… │
    /// ..
    ///
    /// # Examples
    /// ```
    /// let log_gen = LogGen::new(1000, (2025, 2026)).expect("Error on log");
    /// let mut collected_df: Dataframe = runtime_optimized_df_collector(log_gen);
    /// ´´´
    pub struct LogGen {
        count: usize,
        years: (NaiveDate, NaiveDate),
    }

    impl LogGen {
        pub fn new(count: usize, years: (i32, i32)) -> Result<LogGen, String> {
            if years.1 - years.0 > 0 && count > 0 {
                return Ok(Self {
                    count: count,
                    years: (
                        NaiveDate::from_yo_opt(years.0, 1).unwrap(),
                        NaiveDate::from_yo_opt(years.1, 1).unwrap(),
                    ),
                });
            } else {
                return Err(
                    "Year range invalid: should be years.0 > years.1 AND count > 0".to_string(),
                );
            }
        }

        // Date Generation found in: https://stackoverflow.com/questions/77434585/generate-random-date-in-rust-from-date-interval
        pub fn _generate_log(&self) -> Log {
            // First create random values for each datapoint
            let mut rng = rand::rng();
            let days_in_range = (self.years.1 - self.years.0).num_days();
            let random_days: i64 = rng.random_range(0..days_in_range);
            let timestamp: NaiveDateTime = (self.years.0 + Duration::days(random_days))
            .and_hms_opt(
                    rng.random_range(0..23),
                    rng.random_range(0..59),
                    rng.random_range(0..59),
                )
                .unwrap()
                .into();

            let temperature = rng.random_range(15.0..35.0);
            let humidity = rng.random_range(0.0..1.0);
            let temperature_exceeded_25 = temperature > 25.0;
            let humidity_exceeded_60 = humidity > 0.7;

            let temperature_exceeded_30 = temperature > 25.0;
            let humidity_exceeded_70 = humidity > 0.7;

            let level = if temperature_exceeded_30 || humidity_exceeded_70 {
                Level::CRITICAL
            } else if temperature_exceeded_25 || humidity_exceeded_60 {
                Level::WARN
            } else {
                Level::INFO
            };
            let device = if rng.random_bool(0.33) {
                Device::Arduino0
            } else if rng.random_bool(0.5) {
                Device::Arduino1
            } else {
                Device::Arduino2
            }; // each device having 33% chance of being selected => this might be adjustable later on

            let mut info_msg = format!("{}: ", level.to_string());

            // Add temperature and / or humidity information to info msg struct for logs => based whether it exceeds 2 thresholds
            if temperature_exceeded_30 {
                info_msg.push_str(&format!(
                    "Temperature exceeded 30°C: {:.2}°C. ",
                    temperature
                ));
            } else if temperature_exceeded_25 {
                info_msg.push_str(&format!(
                    "Temperature exceeded 25°C: {:.2}°C. ",
                    temperature
                ));
            } else {
                info_msg.push_str(&format!("Temperature: {:.2}°C. ", temperature));
            }

            if humidity_exceeded_70 {
                info_msg.push_str(&format!("Humidity exceeded 70%: {:.2}%. ", humidity));
            } else if humidity_exceeded_60 {
                info_msg.push_str(&format!("Humidity exceeded 60%: {:.2}%. ", humidity));
            } else {
                info_msg.push_str(&format!("Humidity: {:.2}%. ", humidity));
            }

            let msg = Message {
                device: device,
                msg: info_msg,
                exceeded_values: [temperature_exceeded_25, humidity_exceeded_60],
            };

            Log {
                timestamp: timestamp,
                level: level,
                temperatur: temperature,
                humidity: humidity,
                msg: msg,
            }
        }
    }

    impl Iterator for LogGen {
        type Item = Log;

        fn next(&mut self) -> Option<Self::Item> {
            if self.count == 0 {
                return None;
            }

            self.count -= 1;
            Some(self._generate_log())
        }
    }
}
