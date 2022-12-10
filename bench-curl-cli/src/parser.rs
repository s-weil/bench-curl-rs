use core::BenchConfig;
use log::error;
use std::fs;

pub fn parse_toml(file_name: &str) -> Option<BenchConfig> {
    let file_content = fs::read_to_string(file_name).ok()?;
    let specs: BenchConfig = match toml::from_str(&file_content) {
        Ok(parsed) => parsed,
        Err(error) => {
            error!("unable to parse the TOML structure: {:?}", error);
            return None;
        }
    };

    Some(specs)
}

pub fn from_get_url(url: String) -> BenchConfig {
    BenchConfig::new(url)
}
