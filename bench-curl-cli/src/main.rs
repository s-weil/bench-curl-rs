extern crate clap;

use clap::{Parser, Subcommand};

#[derive(Subcommand, Debug)]
enum BenchRunnerArg {
    /// Read in a `specs.toml` file at the specified location `file_path`.
    FromSpecs,
    // TODO: further, such Get url, etc
}

/// CLI to run the benchmarker.
#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {
    /// The scraper to use, with the search string
    #[clap(subcommand)]
    bench_arg: BenchRunnerArg,
    /// The path to the specs file.<br>
    /// Example: 'specs_dir/specs.toml'<br>
    /// Default value: 'specs.toml' in current dir
    #[clap(short, long)]
    file_path: Option<String>,
}

// TODO: add better logging
fn main() {
    let args = CliArgs::parse();
    print!("{:?}", args);
}
