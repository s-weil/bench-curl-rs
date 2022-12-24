mod error;
mod parser;

extern crate clap;

use crate::parser::{from_get_url, parse_toml};
use burl::BenchClient;
use clap::{Parser, Subcommand};
use env_logger::Env;
use log::{error, info, trace};
use std::error::Error;

const LOG_LEVEL: &str = "LOG_LEVEL";
const DEFAULT_LEVEL: &str = "INFO";

#[derive(Subcommand, Debug)]
enum BenchRunnerArg {
    /// Read in a `specs.toml` file at the specified location `file_path`.
    FromToml,
    Get,
    // further: Post, AB testing, etc.
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

fn main() -> Result<(), Box<dyn Error>> {
    let log_level = std::env::var(LOG_LEVEL).unwrap_or_else(|_| DEFAULT_LEVEL.to_string());
    env_logger::Builder::from_env(Env::default().default_filter_or(&log_level)).init();

    let args = CliArgs::parse();

    if let Some(specs) = match args.cmd {
        BenchRunnerArg::FromToml => {
            trace!("Parsing TOML");
            let file_name = args.file_name.unwrap_or_else(|| "specs.toml".to_string());

            let specs = parse_toml(&file_name);
            if specs.is_none() {
                error!("Unable to parse the specifications");
            }
            specs
        }

        BenchRunnerArg::Get => {
            if let Some(url) = args.url {
                Some(from_get_url(url))
            } else {
                error!("URL parameter required.");
                None
            }
        }
    } {
        trace!("initializing runner with {:?}", &specs);
        let dir = specs.results_folder.clone();
        let bencher = BenchClient::init(specs)?;
        if let Some(stats) = bencher.start_run() {
            // trace!("Finished. Summary figures in {:?}Secs", unit);
            info!("{}", stats);
            burl::plot_stats(stats, dir);
        }
    }
    trace!("Finished");
    Ok(())
}
