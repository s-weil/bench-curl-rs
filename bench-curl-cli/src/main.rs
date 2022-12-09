mod error;
mod parser;

extern crate clap;

use clap::{Parser, Subcommand};
use core::BenchClient;
use std::error::Error;

use crate::parser::parse_toml;

/*
    Examples to run
    * bench-curl-cli from-toml
    * bench-curl-cli -- -f "./tests/specs.toml" from-toml
    * bench-curl-cli -- --url 'localhost:5000' get
*/

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

// TODO: add better logging
fn main() -> Result<(), Box<dyn Error>> {
    let args = CliArgs::parse();

    if let Some(specs) = match args.cmd {
        BenchRunnerArg::FromToml => {
            let file_name = args
                .file_name
                .clone()
                .unwrap_or_else(|| "specs.toml".to_string());

            if let Some(specs) = parse_toml(&file_name) {
                dbg!(specs);
            } else {
                print!("Unable to parse the specifications");
            }
            None
        }

        BenchRunnerArg::Get => {
            if let Some(url) = args.url.clone() {
                let specs = core::BenchInput::from_get_url(url);
                dbg!(specs);
            } else {
                print!("URL parameter required.");
            }
            None
        }
    } {
        let bencher = BenchClient::init(specs)?;
        if let Some(stats) = bencher.start_run() {
            println!("SUMMARY: {:?}", stats);
        }
    }
    print!("{:?}", args);
    Ok(())
}
