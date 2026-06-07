// use serde::Deserialize;
// use config::{Config, Environment};

// #[derive(Debug, Deserialize)]
// pub struct Settings {
//     pub allowed_ips: Vec<String>,
//     pub api_keys: Vec<String>,
//     pub bind_address: String,
// }

// impl Settings {
//     pub fn new() -> Result<Self, config::ConfigError> {
//         let mut settings = Config::default();
//         settings.merge(Environment::default())?;
//         settings.try_deserialize()
//     }
// }