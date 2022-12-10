mod error;
mod parser;

extern crate clap;

use clap::{Parser, Subcommand};
use core::BenchClient;
use env_logger::Env;
use log::{error, info, trace};
use std::error::Error;

use crate::parser::{from_get_url, parse_toml};

/*
    Examples to run
    * bench-curl-cli from-toml
    * bench-curl-cli -- -f "./tests/specs.toml" from-toml
    * bench-curl-cli -- --url 'localhost:5000' get
*/

const LOG_LEVEL: &str = "LOG_LEVEL";
const DEFAULT_LEVEL: &str = "INFO";

#[derive(Subcommand, Debug)]
enum BenchRunnerArg {
    /// Read in a `specs.toml` file at the specified location `file_path`.
    FromToml,
    Get,
    // further
}

/// CLI to run the benchmarker.
#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {
    #[clap(subcommand)]
    cmd: BenchRunnerArg,
    /// The path to the specs file.<br>
    /// Example: 'specs_dir/specs.toml'<br>
    /// Default value: 'specs.toml' in current dir
    #[clap(short, long)]
    file_name: Option<String>,
    #[clap(short, long)]
    url: Option<String>,
}

// TODO: add better logging?
fn main() -> Result<(), Box<dyn Error>> {
    let log_level = std::env::var(LOG_LEVEL).unwrap_or(DEFAULT_LEVEL.to_string());
    env_logger::Builder::from_env(Env::default().default_filter_or(&log_level)).init();

    let args = CliArgs::parse();

    if let Some(specs) = match args.cmd {
        BenchRunnerArg::FromToml => {
            info!("Parsing TOML");
            let file_name = args
                .file_name
                .clone()
                .unwrap_or_else(|| "specs.toml".to_string());

            info!("{:?}", &file_name);

            let specs = parse_toml(&file_name);
            if let None = specs {
                error!("Unable to parse the specifications");
            }
            specs
        }

        BenchRunnerArg::Get => {
            info!("GET");
            if let Some(url) = args.url.clone() {
                Some(from_get_url(url))
            } else {
                error!("URL parameter required.");
                None
            }
        }
    } {
        info!("initializing runner with {:?}", &specs);
        let unit = specs.duration_unit();

        let dir = specs.results_folder.clone();
        let bencher = BenchClient::init(specs)?;
        if let Some(stats) = bencher.start_run() {
            info!("SUMMARY: [in {:?}Secs] {:?}", unit, stats);
            core::plot(stats, dir);
        }
    }
    info!("Finished");
    Ok(())
}
