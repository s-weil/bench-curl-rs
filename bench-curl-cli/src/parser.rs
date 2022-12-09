use core::BenchInput;
use std::fs;

// TODO: convert to result
pub fn parse_toml(file_name: &str) -> Option<BenchInput> {
    let file_content = fs::read_to_string(file_name).ok()?;
    let specs: BenchInput = toml::from_str(&file_content).ok()?;
    Some(specs)
}

pub fn from_get_url(url: String) -> BenchInput {
    BenchInput::new(url)
}
