use crate::html_report::SummaryComponent;
use crate::plots::{
    BootstrapHistogramComponent, BoxPlotComponent, HistogramComponent, QQPlotComponent,
    TimeSeriesComponent,
};
use crate::ComponentWriter;
use burl::sampling::SampleResult;
use burl::stats::{StatsProcessor, StatsSummary};
use burl::{BenchClientConfig, BurlError, BurlResult, ThreadIdx};
use chrono::{DateTime, Utc};
use log::{info, warn};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

const COMPONENTS_DIR: &str = "components";
const DATA_DIR: &str = "data";
const FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const HIST_PATH: &str = "hist";

#[derive(Serialize)]
struct ReportMeta {
    start_time: String,
    end_time: String,
    config: BenchClientConfig,
}

impl<'a> From<&ReportFactory<'a>> for ReportMeta {
    fn from(rs: &ReportFactory<'a>) -> Self {
        Self {
            start_time: format!("{}", rs.start_time.format(FORMAT)),
            end_time: format!("{}", rs.end_time.format(FORMAT)),
            config: rs.config.clone(),
        }
    }
}

fn create_dir(dir: &Path) -> BurlResult<()> {
    if dir.exists() && dir.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(dir)?;
    Ok(())
}

fn hist_results(from_dir: &PathBuf) -> BurlResult<()> {
    if !from_dir.exists() {
        return Ok(());
    }

    let copy_dir = from_dir
        .join(HIST_PATH)
        .join(Utc::now().format("%Y-%m-%d__%H_%M_%S").to_string());

    create_dir(&copy_dir)?;

    for entry in fs::read_dir(from_dir)? {
        let entry = entry?;
        let src_path = entry.path();
        if !src_path.is_dir() {
            let target_file = copy_dir.join(entry.file_name());
            fs::rename(src_path.as_os_str(), target_file)?;
        }
    }

    Ok(())
}

fn read_data<D: serde::de::DeserializeOwned>(file: &PathBuf) -> BurlResult<D> {
    let file_data = fs::read_to_string(file)?;
    let data: D = serde_json::from_str(&file_data)?;
    Ok(data)
}

fn setup_report_structure(path: &Path) -> Result<(PathBuf, PathBuf), BurlError> {
    if !path.exists() {
        fs::create_dir(path)?;
    }

    let report_file = path.join("report.html");
    if !report_file.exists() {
        let template = include_str!("./templates/report_template.html");
        fs::write(report_file, template)?;
    }

    let components_dir = Path::new(&path).join(COMPONENTS_DIR);
    if !components_dir.exists() {
        fs::create_dir(&components_dir)?;
    }

    let data_dir = Path::new(&path).join(DATA_DIR);
    if !data_dir.exists() {
        fs::create_dir(&data_dir)?;
    }

    info!("Creating report in {:?}", path.as_os_str());
    Ok((components_dir, data_dir))
}

fn serialize<D: Serialize>(data: &D) -> BurlResult<String> {
    let json = serde_json::to_string_pretty(data)?;
    Ok(json)
}

/// Serializes the data, creates or updates the file and its contents.
fn write_or_update<D: Serialize>(serializable_data: &D, file: PathBuf) -> BurlResult<()> {
    let json = serialize(serializable_data)?;
    fs::write(file, json)?;
    Ok(())
}

pub struct ReportFactory<'a> {
    config: &'a BenchClientConfig,
    stats_processor: StatsProcessor,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
}

impl<'a> ReportFactory<'a> {
    pub fn new(
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        config: &'a BenchClientConfig,
        stats_processor: StatsProcessor,
    ) -> Self {
        Self {
            config,
            stats_processor,
            start_time,
            end_time,
        }
    }

    fn dump_data(
        &self,
        dir: PathBuf,
        stats: &Option<StatsSummary>,
        sample_results_by_thread: &HashMap<ThreadIdx, Vec<SampleResult>>,
    ) -> Result<(), BurlError> {
        let stats_file = dir.join("stats.json");
        let samples_file = dir.join("samples.json");
        let meta_file = dir.join("meta.json");

        if stats_file.exists() | meta_file.exists() | samples_file.exists() {
            if let Err(err) = hist_results(&dir) {
                warn!("Overwriting existing baseline results: {}", err);
            }
        }

        let report_meta = ReportMeta::from(self);

        // creates or updates the files and its contents
        write_or_update(stats, stats_file)?;
        write_or_update(&report_meta, meta_file)?;
        write_or_update(&sample_results_by_thread, samples_file)?;

        Ok(())
    }

    fn baseline_results(&self, data_dir: &Path) -> Option<StatsSummary> {
        let baseline_dir = match &self.config.baseline_path {
            Some(p) => PathBuf::new().join(p),
            None => data_dir.to_path_buf(),
        };

        if !baseline_dir.exists() {
            warn!(
                "Specified baseline directory does not exist: {:?}",
                baseline_dir.as_os_str()
            );
            return None;
        }

        let results_file = &baseline_dir.join("stats.json");

        if !results_file.exists() {
            warn!(
                "Expected file does not exist: {:?}",
                results_file.as_os_str()
            );
            return None;
        }

        let baseline_results: Option<StatsSummary> = read_data(results_file).ok();
        baseline_results
    }

    fn create_components(
        &self,
        components_dir: Option<PathBuf>,
        current_stats: &Option<StatsSummary>,
        baseline_stats: Option<StatsSummary>,
        sample_results_by_thread: &HashMap<ThreadIdx, Vec<SampleResult>>,
    ) -> BurlResult<()> {
        let stats = match current_stats {
            Some(stats) => stats,
            None => {
                return Ok(());
            }
        };

        let mut summary = SummaryComponent::new();
        let mut box_plot = BoxPlotComponent::new();
        let mut time_series_plot = TimeSeriesComponent::new();
        let mut histogram = HistogramComponent::new();
        let mut qq_plot = QQPlotComponent::new();
        let mut bs_histogram = BootstrapHistogramComponent::new();

        let time_series = sample_results_by_thread
            .iter()
            .map(|(thread_idx, sample_results)| {
                let ts = sample_results
                    .iter()
                    .map(|sr| sr.as_timeseries_point())
                    .collect();
                (*thread_idx, ts)
            })
            .collect();

        time_series_plot.add(&time_series);

        summary.add_current(&stats);
        box_plot.add_total(&stats.durations);
        histogram.set_bins(stats.min, stats.max);
        histogram.add_total(&stats.durations);
        qq_plot.add_current(&stats.normal_qq_curve());

        if stats.stats_by_thread.len() > 1 {
            box_plot.add_threads(&stats.stats_by_thread);
            histogram.add_threads(&stats.stats_by_thread);
        }

        if let (bootstrap_means, Some((lower_bound, upper_bound))) = stats.bootstrap_summary(
            self.config.n_bootstrap_draw_size(),
            self.config.n_bootstrap_samples(),
            self.config.alpha(),
        ) {
            bs_histogram.add_total(&bootstrap_means);
            bs_histogram.add_confidence_interval(lower_bound, upper_bound);
        }

        if let Some(bl_stats) = baseline_stats {
            qq_plot.add_baseline(&bl_stats.normal_qq_curve());
            summary.add_baseline(bl_stats.clone());
        }

        summary.compile(self.config.alpha(), self.config.n_bootstrap_samples());
        qq_plot.add_reference_line();

        match &components_dir {
            Some(dir) => {
                summary.write(&dir.join("summary.html"))?;
                box_plot.write(&dir.join("durations_distribution.html"))?;
                time_series_plot.write(&dir.join("durations_timeseries.html"))?;
                histogram.write(&dir.join("durations_histogram.html"))?;
                qq_plot.write(&dir.join("qq_plot.html"))?;
                bs_histogram.write(&dir.join("bootstrap_histogram.html"))?;
            }
            None => {
                box_plot.show();
                time_series_plot.show();
                histogram.show();
            }
        }

        Ok(())
    }

    pub fn create_report(&self) -> Result<(), BurlError> {
        let current_results: Option<StatsSummary> = self.stats_processor.stats_summary();
        let sample_results_by_thread = self.stats_processor.sample_results_by_thread();

        if let Some(report_path) = &self.config.report_directory {
            let path = Path::new(report_path);
            let (components_dir, data_dir) = setup_report_structure(path)?;

            let baseline_results: Option<StatsSummary> = self.baseline_results(&data_dir);
            self.dump_data(data_dir, &current_results, &sample_results_by_thread)?;
            self.create_components(
                Some(components_dir),
                &current_results,
                baseline_results,
                &sample_results_by_thread,
            )?;
        } else {
            self.create_components(None, &current_results, None, &sample_results_by_thread)?;
        }

        Ok(())
    }
}

// TODO: rename to Html? and to TableComponent below?
// pub trait ReportComponent {
//     type Content;
//     fn generate(&mut self, content: Self::Content) -> Self;
//     fn write(&self, file: PathBuf) -> BurlResult<()>;
// }

// pub trait ComponentWriter {
//     fn write(&self, file: PathBuf) -> BurlResult<()>;
// }

// type HtmlTemplate = String;
// impl<T> ComponentWriter for T
// where
//     T: Deref<Target = HtmlTemplate>,
// {
//     fn write(&self, file: PathBuf) -> BurlResult<()> {
//         fs::write(file, &self)?;
//         Ok(())
//     }
// }

// impl<T> ComponentWriter for T
// where
//     T::Resource = plotly::Plot,
// {
//     type Resource = plotly::Plot;
//     fn as_resource(&self) -> &plotly::Plot;
//     fn write(&self, file: PathBuf) -> BurlResult<()> {
//         self.as_resource().to_html(file);
//         Ok(())
//     }
// }

// pub trait ComponentCreator {
//     fn init() -> Self;
//     fn add<Content>(&mut self, content: &Content) -> Self;
// }

// pub trait ReportComponent: ComponentGenerator + ComponentWriter {}

// type HtmlTemplate = String;
// trait HtmlComponent: ReportComponent {
//     fn template(&self) -> HtmlTemplate;
//     fn write(&self, file: PathBuf) -> BurlResult<()> {
//         fs::write(file, &self)?;
//         Ok(())
//     }
// }

// pub struct SummaryComponent {
//     template_ref: &'static str,
// }

// pub trait PlotlyComponent: ReportComponent {
//     // fn write(&self, resource: &plotly::Plot, file: PathBuf) -> BurlResult<()> {
//     //     resource.to_html(file);
//     //     Ok(())
//     // }
//     fn show(&self, content: &plotly::Plot) -> () {
//         content.show();
//     }

//     // fn set_layout(&mut self, layout: plotly::Layout) -> Self::Resource;
// }

// impl ReportComponent for plotly::Plot {
//     fn generate(&mut self, content: Self::Content) -> Self {}
// }
