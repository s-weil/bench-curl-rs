use crate::stats::Stats;
use log::info;
use plotly::box_plot::BoxPoints;
use plotly::common::{Line, LineShape, Marker, Mode, Title};
use plotly::histogram::HistNorm;
use plotly::layout::{Axis, BarMode, BoxMode, Layout};
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

fn rgb_color(thread_idx: usize, n_threads: usize) -> Rgb {
    let min = 50;
    let max = 255;
    let step_size = (max - min) / n_threads;
    let scale = (min + thread_idx * step_size) as u8;
    Rgb::new(scale, min as u8, scale)
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

    let trace_durations_box_plot = BoxPlot::new(stats.durations)
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
        .bar_mode(BarMode::Overlay)
        .title(Title::new("Durations frequency distribution"))
        .x_axis(Axis::new().title(Title::new("durations")).zero_line(true))
        .y_axis(Axis::new().title(Title::new("frequency")).zero_line(true));
    plot.set_layout(layout);

    // TODO: improve on n buckets, size, and overlay

    let n_buckets = 20; // stats.n_ok / 10_usize

    let total_histogram = Histogram::new(stats.durations.clone())
        .hist_norm(HistNorm::Probability)
        .name("total")
        // .opacity(0.2)
        .marker(Marker::new().color(NamedColor::Blue))
        .n_bins_x(n_buckets);

    plot.add_trace(total_histogram);

    if stats.stats_by_thread.len() > 1 {
        for (thread_idx, thread_stats) in stats.stats_by_thread.iter() {
            let thread_color = rgb_color(*thread_idx, stats.stats_by_thread.len());
            let thread_hist = Histogram::new(thread_stats.durations.clone())
                .name(thread_idx.to_string().as_str())
                .hist_norm(HistNorm::Probability)
                .opacity(0.5)
                .marker(Marker::new().color(thread_color))
                .n_bins_x(n_buckets);
            plot.add_trace(thread_hist)
        }
    }

    if let Some(path) = output_path {
        let file_name = path.join("durations_histogram.html");
        plot.to_html(file_name);
    } else {
        plot.show();
    }
}

fn plot_time_series(stats: &Stats, output_path: &Option<PathBuf>) {
    let mut plot = Plot::new();

    for (thread_idx, ts) in stats.stats_by_thread.iter() {
        let ts = &ts.time_series;
        let mut ts_dates: Vec<f64> = Vec::with_capacity(ts.len());
        let mut ts_values = Vec::with_capacity(ts.len());

        for ts_point in ts.iter() {
            let (time, v) = ts_point.as_graph_point();
            ts_dates.push(time);
            ts_values.push(v);
        }

        let thread_color = rgb_color(*thread_idx, stats.stats_by_thread.len());

        let trace_ts = Scatter::new(ts_dates, ts_values)
            .name(thread_idx.to_string().as_str())
            .mode(Mode::LinesMarkers)
            .line(Line::new().shape(LineShape::Hv))
            .marker(Marker::new().color(thread_color));
        plot.add_trace(trace_ts);
    }

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
