pub mod log_types {
    use serde::{Deserialize, Serialize};
    use std::fmt;

    /// Enum representing all logging levels within the logs. Implements fmt::Display trait in order to be string-convertible.
    ///
    /// # Examples
    /// ```
    /// let level = if temperature_exceeded_30 || humidity_exceeded_70 { Level::CRITICAL } else if temperature_exceeded_25 || humidity_exceeded_60 { Level::WARN } else { Level::INFO };
    /// ´´´
    #[derive(Serialize, Deserialize)]
    pub enum Level {
        INFO,
        WARN,
        CRITICAL,
    }

    impl fmt::Display for Level {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                Level::INFO => write!(f, "INFO"),
                Level::WARN => write!(f, "WARN"),
                Level::CRITICAL => write!(f, "CRITICAL"),
            }
        }
    }

    /// Enum containing names of dummy devices that are used to associate logs with. Implements fmt::Display trait in order to be string-convertible.
    ///
    /// # Examples
    /// ```
    /// let device = if rng.random_bool(0.33) { Device::Arduino0 } else if rng.random_bool(0.5) { Device::Arduino1 } else { Device::Arduino2 };
    /// ´´´
    #[derive(Serialize, Deserialize)]
    pub enum Device {
        Arduino0,
        Arduino1,
        Arduino2,
    }

    impl fmt::Display for Device {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                Device::Arduino0 => write!(f, "arduino0"),
                Device::Arduino1 => write!(f, "arduino1"),
                Device::Arduino2 => write!(f, "arduino2"),
            }
        }
    }

    /// Enum containing measurments for the values collected within the logs. Implements fmt::Display trait in order to be string-convertible.
    ///
    /// # Examples
    /// ```
    /// let temp_str = Measurement::Temperature.to_string()
    /// ´´´
    #[derive(Serialize, Deserialize)]
    pub enum Measurement {
        Temperature,
        Humidity,
    }

    impl fmt::Display for Measurement {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                Measurement::Temperature => write!(f, "temperature"),
                Measurement::Humidity => write!(f, "humidity"),
            }
        }
    }

    /// Struct representing info msg within each log. This is serializable in order to be represented within a dataframe as string.
    ///
    /// # Examples
    /// ```
    /// ...
    /// let msg: Message = Message {
    ///            device: device,
    ///            msg: info_msg,
    ///            exceeded_values: [
    ///                temperature_exceeded_25,
    ///                humidity_exceeded_60
    ///            ]
    ///        };
    /// let msg_json: String = to_string(&msg).unwrap()
    /// ´´´
    #[derive(Serialize, Deserialize)]
    pub struct Message {
        pub device: Device,
        pub msg: String,
        pub exceeded_values: [bool; 2],
    }

    /// Struct representing the whole log as struct. This is serializable in order to be represented within a dataframe as string.
    ///
    /// # Examples
    /// ```
    /// ...
    /// let log: Log = Log {
    ///            timestamp: timestamp,
    ///            level: level,
    ///            temperatur: temperature,
    ///            humidity: humidity,
    ///            msg: msg
    ///        }
    /// let log_json: String = to_string(&log).unwrap()
    /// ´´´
    #[derive(Serialize, Deserialize)]
    pub struct Log {
        pub timestamp: String,
        pub level: Level,
        pub temperatur: f32,
        pub humidity: f32,
        pub msg: Message,
    }
}
