use crate::{plots, stats::Stats};
use log::{info, warn};
use std::{
    fs,
    path::{Path, PathBuf},
};

const REPORT_TEMPLATE_PATH: &'static str = "../template.html";
const PLOT_DIR: &'static str = "plots";
const DATA_DIR: &'static str = "data";

const REPORT_TEMPLATE: &str = r#"
<html>
</head>
<body>
<div>
  <iframe src="./plots/durations_distribution.html" seamless width="2000" height="600" frameBorder="0">
    Warning: durations_distribution.html could not be included.
  </iframe>
</div>
<div>
  <iframe src="./plots/durations_histogram.html" seamless width="1200" height="600" title = "histogram" frameBorder="0">
    Warning: durations_histogram.html could not be included.
  </iframe>
</div>
<div>
  <iframe src="./plots/durations_timeseries.html" seamless width="1200" height="600" frameBorder="0">
    Warning: durations_timeseries.html could not be included.
  </iframe>
</div>
</body>
</html>
"#;

fn setup_report(path: &Path) -> Result<(PathBuf, PathBuf), std::io::Error> {
    if !path.exists() {
        fs::create_dir(path)?;
    }
    let report_file = path.join("report.html");
    if !report_file.exists() {
        // let template = include_str!(REPORT_TEMPLATE_PATH);
        fs::write(report_file, REPORT_TEMPLATE)?;
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

fn dump_result_data(stats: &Stats, dir: PathBuf) -> Result<(), String> {
    let json_data = stats.serialize()?;
    let file_name = dir.join("results.json");

    if file_name.exists() {
        warn!("Overwriting existing results data");
    }

    // creates or updates the file and its contents
    fs::write(file_name, json_data)
        .map_err(|err| format!("Cannot save results: {}", err.to_string()))?;
    Ok(())
}

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
