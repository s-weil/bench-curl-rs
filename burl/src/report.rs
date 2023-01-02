use crate::{plots, stats::Stats, BenchClient, BenchConfig};
use log::{info, warn};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

const PLOT_DIR: &'static str = "plots";
const DATA_DIR: &'static str = "data";

#[derive(Serialize)]
struct ReportMeta {
    // TODO: consider to change to chrono & NaiveDate
    start_time: SystemTime,
    end_time: SystemTime,
    // config: ...
}

// TODO: split stats into sample data and actual metrics/stats

fn setup_report(path: &Path) -> Result<(PathBuf, PathBuf), std::io::Error> {
    if !path.exists() {
        fs::create_dir(path)?;
    }

    let report_file = path.join("report.html");
    if !report_file.exists() {
        let template = include_str!("../template.html");
        fs::write(report_file, template)?;
    }

    let plot_dir = Path::new(&path).join(PLOT_DIR);
    if !plot_dir.exists() {
        fs::create_dir(&plot_dir)?;
    }

    let data_dir = Path::new(&path).join(DATA_DIR);
    if !data_dir.exists() {
        fs::create_dir(&data_dir)?;
    }

    info!("Creating report in {:?}", path.as_os_str());
    Ok((plot_dir, data_dir))
}

fn serialize<D: Serialize>(d: &D) -> Result<String, String> {
    let json = serde_json::to_string_pretty(d)
        .map_err(|err| format!("Cannot serialize: {}", err.to_string()))?;
    Ok(json)
}

/// Serializes the data, creates or updates the file and its contents.
fn write_or_update<D: Serialize>(d: &D, file: PathBuf) -> Result<(), String> {
    let json = serialize(d)?;
    fs::write(file, json).map_err(|err| format!("Cannot save to file: {}", err.to_string()))?;
    Ok(())
}

fn dump_result_data(stats: &Stats, dir: PathBuf) -> Result<(), String> {
    let stats_file = dir.join("stats.json");
    let samples_file = dir.join("samples.json");
    let meta_file = dir.join("meta.json");

    if stats_file.exists() | meta_file.exists() | samples_file.exists() {
        // TODO: create a backup of earlier run
        warn!("Overwriting existing results data");
    }

    let dummy_meta = ReportMeta {
        start_time: SystemTime::now(),
        end_time: SystemTime::now(),
    };

    // creates or updates the files and its contents
    write_or_update(&stats, stats_file)?;
    write_or_update(&dummy_meta, meta_file)?;

    Ok(())
}

// pub struct Report {
//     config: BenchConfig,
//     stats: Stats,
//     start_time: SystemTime,
//     end_time: SystemTime,
// }

pub fn create_report(stats: Stats, output_path: Option<String>) -> Result<(), String> {
    // TODO: add plotoptions with outputpath, duration scale, title etc

    if let Some(report_path) = output_path {
        let path = Path::new(&report_path);
        let (plot_dir, data_dir) = setup_report(&path)
            .map_err(|err| format!("Unable to set up report structure: {}", err))?;

        dump_result_data(&stats, data_dir)?;
        plots::plot_stats(stats, Some(plot_dir))
    } else {
        plots::plot_stats(stats, None)
    }

    Ok(())
}
