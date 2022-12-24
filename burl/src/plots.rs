use crate::stats::Stats;
use log::info;
use plotly::box_plot::BoxPoints;
use plotly::common::{Line, LineShape, Marker, Mode, Title};
use plotly::histogram::HistNorm;
use plotly::layout::{Axis, BoxMode, Layout};
use plotly::{BoxPlot, Histogram, NamedColor, Plot, Rgb, Scatter};
use std::fs;
use std::path::{Path, PathBuf};

/// https://github.com/igiagkiozis/plotly/blob/master/examples/statistical_charts/src/main.rs///
/// https://igiagkiozis.github.io/plotly/content/recipes/statistical_charts/box_plots.html

const REPORT_TEMPLATE: &str = r#"
<html>
</head>
<body>
<div>
  <iframe src="./plots/durations_distribution.html" seamless width="800" height="600" frameBorder="0">
    Warning: durations_distribution.html could not be included.
  </iframe>
</div>
<div>
  <iframe src="./plots/durations_histogram.html" seamless width="800" height="600" title = "histogram" frameBorder="0">
    Warning: durations_histogram.html could not be included.
  </iframe>
</div>
<div>
  <iframe src="./plots/durations_timeseries.html" seamless width="800" height="600" frameBorder="0">
    Warning: durations_timeseries.html could not be included.
  </iframe>
</div>
</body>
</html>
"#;

fn setup_report(output_path: Option<String>) -> Option<PathBuf> {
    let output = output_path?;
    let path = Path::new(&output);

    if !path.exists() {
        fs::create_dir(path).unwrap();
    }
    let report_file = path.join("report.html");
    if !report_file.exists() {
        fs::write(report_file, REPORT_TEMPLATE).unwrap();
    }

    let plot_dir = Path::new(&path).join("plots");
    if !plot_dir.exists() {
        fs::create_dir(&plot_dir).unwrap();
    }

    info!("Creating report in {}", output);
    Some(plot_dir)
}

pub fn plot_stats(stats: Stats, output_path: Option<String>) {
    // TODO: add plotoptions with outputpath, duration scale, title etc

    let plot_dir = setup_report(output_path);
    plot_time_series(&stats, &plot_dir);
    plot_histogram(&stats, &plot_dir);
    plot_box_plot(stats, &plot_dir);
}

fn plot_box_plot(stats: Stats, output_path: &Option<PathBuf>) {
    // let trace = Histogram::new(stats.distribution).name("h");
    let mut plot = Plot::new();
    let box_plot_layout = Layout::new()
        .title(Title::new("Box Plot"))
        .y_axis(
            Axis::new()
                .title(Title::new("durations [unit]"))
                .zero_line(true),
        )
        .box_mode(BoxMode::Group);
    plot.set_layout(box_plot_layout);

    let trace_durations_box_plot = BoxPlot::new(stats.distribution)
        .name("")
        .jitter(0.7)
        .point_pos(-1.8)
        .marker(Marker::new().color(Rgb::new(7, 40, 89)))
        .box_points(BoxPoints::All);
    plot.add_trace(trace_durations_box_plot);

    // TODO: possible to plot histogram and box in one?
    // let trace_histogram = Histogram::new(stats.distribution)
    //     .name("h")
    //     .marker(Marker::new().color(NamedColor::Pink));

    // plot.add_trace(trace_histogram);

    if let Some(path) = output_path {
        let file_name = path.join("durations_distribution.html");
        plot.to_html(file_name);
    } else {
        plot.show();
    }
}

fn plot_histogram(stats: &Stats, output_path: &Option<PathBuf>) {
    let mut plot = Plot::new();

    let layout = Layout::new()
        .title(Title::new("Durations frequency distribution"))
        .x_axis(Axis::new().title(Title::new("durations")).zero_line(true))
        .y_axis(Axis::new().title(Title::new("frequency")).zero_line(true));
    plot.set_layout(layout);

    let trace_histogram = Histogram::new(stats.distribution.clone())
        .hist_norm(HistNorm::Probability)
        .name("h")
        .opacity(0.6)
        .marker(Marker::new().color(NamedColor::Blue))
        .n_bins_x(stats.n_ok / 5_usize);

    plot.add_trace(trace_histogram);

    if let Some(path) = output_path {
        let file_name = path.join("durations_histogram.html");
        plot.to_html(file_name);
    } else {
        plot.show();
    }
}

fn plot_time_series(stats: &Stats, output_path: &Option<PathBuf>) {
    let mut plot = Plot::new();

    let mut ts_dates: Vec<f64> = Vec::with_capacity(stats.time_series.len());
    let mut ts_values = Vec::with_capacity(stats.time_series.len());

    for (date, value) in stats.time_series.iter() {
        ts_dates.push(*date);
        ts_values.push(*value);
    }

    let trace_ts = Scatter::new(ts_dates, ts_values)
        .mode(Mode::LinesMarkers)
        .line(Line::new().shape(LineShape::Hv));
    plot.add_trace(trace_ts);

    let ts_layout = Layout::new()
        .title(Title::new("Durations time series"))
        .x_axis(
            Axis::new()
                .title(Title::new("total duration"))
                .zero_line(true),
        )
        .y_axis(
            Axis::new()
                .title(Title::new("request durations"))
                .zero_line(true),
        );
    plot.set_layout(ts_layout);

    if let Some(path) = output_path {
        let file_name = path.join("durations_timeseries.html");
        plot.to_html(file_name);
    } else {
        plot.show();
    }
}
