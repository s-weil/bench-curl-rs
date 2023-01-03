use crate::{
    reporting::plots::{plot_box_plot, plot_histogram, plot_time_series},
    reporting::stats::Stats,
    sampling::{SampleCollector, SampleResult},
    BenchConfig, ThreadIdx,
};
use chrono::{DateTime, Utc};
use log::{info, warn};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

const PLOT_DIR: &str = "plots";
const DATA_DIR: &str = "data";
const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Serialize)]
struct ReportMeta {
    start_time: String,
    end_time: String,
    config: BenchConfig,
}

impl<'a> From<&ReportSummary<'a>> for ReportMeta {
    fn from(rs: &ReportSummary<'a>) -> Self {
        Self {
            start_time: format!("{}", rs.start_time.format(FORMAT)),
            end_time: format!("{}", rs.end_time.format(FORMAT)),
            config: rs.config.clone(),
        }
    }
}

fn setup_report(path: &Path) -> Result<(PathBuf, PathBuf), std::io::Error> {
    if !path.exists() {
        fs::create_dir(path)?;
    }

    let report_file = path.join("report.html");
    if !report_file.exists() {
        let template = include_str!("./template.html");
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

fn serialize<D: Serialize>(data: &D) -> Result<String, String> {
    let json =
        serde_json::to_string_pretty(data).map_err(|err| format!("Cannot serialize: {}", err))?;
    Ok(json)
}

/// Serializes the data, creates or updates the file and its contents.
fn write_or_update<D: Serialize>(serializable_data: &D, file: PathBuf) -> Result<(), String> {
    let json = serialize(serializable_data)?;
    fs::write(file, json).map_err(|err| format!("Cannot save to file: {}", err))?;
    Ok(())
}

pub struct ReportSummary<'a> {
    config: &'a BenchConfig,
    sample_results_by_thread: HashMap<ThreadIdx, Vec<SampleResult>>,
    pub stats: Option<Stats>,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
}

impl<'a> ReportSummary<'a> {
    pub fn new(
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        config: &'a BenchConfig,
        samples_by_thread: Vec<SampleCollector>,
    ) -> Self {
        let stats = Stats::collect(&samples_by_thread, config.duration_scale());

        let mut sample_results_by_thread = HashMap::new();
        for samples in samples_by_thread {
            let sample_results = samples
                .results
                .into_iter()
                .flat_map(|sr| sr.as_result().cloned())
                .collect();
            sample_results_by_thread.insert(samples.thread_idx, sample_results);
        }

        Self {
            config,
            stats,
            sample_results_by_thread,
            start_time,
            end_time,
        }
    }

    fn dump_data(&self, dir: PathBuf) -> Result<(), String> {
        let stats_file = dir.join("stats.json");
        let samples_file = dir.join("samples.json");
        let meta_file = dir.join("meta.json");

        if stats_file.exists() | meta_file.exists() | samples_file.exists() {
            // TODO: create a backup of earlier run
            warn!("Overwriting base line results");
        }

        let dummy_meta = ReportMeta::from(self);

        // creates or updates the files and its contents
        write_or_update(&self.stats, stats_file)?;
        write_or_update(&dummy_meta, meta_file)?;
        write_or_update(&self.sample_results_by_thread, samples_file)?;

        Ok(())
    }

    fn plot_stats(&self, plot_dir: Option<PathBuf>) {
        if let Some(stats) = &self.stats {
            plot_histogram(stats, &plot_dir);
            plot_box_plot(stats, &plot_dir);
        }

        let time_series = self
            .sample_results_by_thread
            .iter()
            .map(|(thread_idx, sample_results)| {
                let ts = sample_results
                    .iter()
                    .map(|sr| sr.as_timeseries_point())
                    .collect();
                (*thread_idx, ts)
            })
            .collect();
        plot_time_series(&time_series, &plot_dir);
    }

    pub fn create_report(&self) -> Result<(), String> {
        // TODO: add plotoptions with outputpath, duration scale, title etc

        if let Some(report_path) = &self.config.report_folder {
            let path = Path::new(report_path);
            let (plot_dir, data_dir) = setup_report(path)
                .map_err(|err| format!("Unable to set up report structure: {}", err))?;

            self.dump_data(data_dir)?;
            self.plot_stats(Some(plot_dir));
        } else {
            self.plot_stats(None);
        }

        Ok(())
    }
}
