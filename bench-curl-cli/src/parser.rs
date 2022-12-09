use core::BenchInput;
use std::fs;
use toml;

// TODO: convert to result
pub fn parse_toml(file_name: &str) -> Option<BenchInput> {
    let file_content = fs::read_to_string(file_name).ok()?;
    let specs: BenchInput = toml::from_str(&file_content).ok()?;
    Some(specs)
}
