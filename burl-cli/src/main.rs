extern crate clap;

use burl::parser::{from_get_url, parse_toml};
use burl::BenchClient;
// use burl_reporter::
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
    // TODO: further: Put, etc
}

/// CLI to run the burl benchmarker.
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

const DEFAULT_TOML: &str = "specs.toml";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let log_level = std::env::var(LOG_LEVEL).unwrap_or_else(|_| DEFAULT_LEVEL.to_string());
    env_logger::Builder::from_env(Env::default().default_filter_or(&log_level)).init();

    let args = CliArgs::parse();

    if let Some(specs) = match args.cmd {
        BenchRunnerArg::FromToml => {
            trace!("Parsing TOML");
            let file_name = args.file_name.unwrap_or_else(|| DEFAULT_TOML.to_string());

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
        trace!("Initializing runner with {:?}", &specs);
        let bencher = BenchClient::init(&specs)?;
        if let Some(run_summary) = bencher.run().await {
            if let Some(stats) = &run_summary.stats() {
                info!("{}", stats);
            }

            let report_summary = burl_reporter::ReportFactory::new(
                run_summary.start_time,
                run_summary.end_time,
                &specs,
                run_summary.stats_processor,
            );

            if let Err(err) = report_summary.create_report() {
                error!("Report creation failed: {}", err);
            }
        }
    }
    trace!("Finished");
    Ok(())
}
