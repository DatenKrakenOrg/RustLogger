pub mod log_types{
    use std::fmt;
    use serde::{Serialize, Deserialize};

    pub enum Level {
        INFO,
        WARN,
        CRITICAL
    }

    impl fmt::Display for Level {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                Level::INFO => write!(f, "INFO"),
                Level::WARN => write!(f, "WARN"),
                Level::CRITICAL => write!(f, "CRITICAL")
            }
        }
    }

     #[derive(Serialize, Deserialize)]
    pub enum Device {
        Arduino0,
        Arduino1,
        Arduino2
    }

    impl fmt::Display for Device {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                Device::Arduino0=> write!(f, "arduino0"),
                Device::Arduino1 => write!(f, "arduino1"),
                Device::Arduino2 => write!(f, "arduino2")
            }
        }
    }

     #[derive(Serialize, Deserialize)]
    pub enum Measurement {
        Temperature,
        Humidity,
    }

    impl fmt::Display for Measurement {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                Measurement::Temperature=> write!(f, "temperature"),
                Measurement::Humidity => write!(f, "humidity"),
            }
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct Message{
        pub device: Device,
        pub msg: String,
        pub exceeded_values: Vec<Measurement>
    }
}