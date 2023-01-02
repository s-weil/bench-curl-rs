use crate::stats::Stats;
use plotly::box_plot::{BoxMean, BoxPoints};
use plotly::common::{Line, LineShape, Marker, Mode, Title};
use plotly::histogram::{Bins, HistNorm};
use plotly::layout::{Axis, BarMode};
use plotly::{BoxPlot, Histogram, Layout, NamedColor, Plot, Rgb, Scatter};
use std::path::PathBuf;

/// https://github.com/igiagkiozis/plotly/blob/master/examples/statistical_charts/src/main.rs///
/// https://igiagkiozis.github.io/plotly/content/recipes/statistical_charts/box_plots.html

pub fn plot_stats(stats: Stats, plot_dir: Option<PathBuf>) {
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
    let mut plot = Plot::new();

    let layout = Layout::new()
        .title(Title::new("Durations box plot"))
        .y_axis(
            Axis::new()
                .title(Title::new("durations"))
                .show_grid(true)
                .zero_line(true)
                .grid_width(1)
                .zero_line_width(2),
        );

    let trace_durations_box_plot = BoxPlot::new(stats.durations)
        .name("total")
        .jitter(0.7)
        .marker(Marker::new().color(Rgb::new(7, 40, 89)).size(6))
        .box_mean(BoxMean::StandardDeviation)
        .box_points(BoxPoints::All)
        .line(Line::new().width(2.0));

    if stats.stats_by_thread.len() > 1 {
        for (thread_idx, thread_stats) in stats.stats_by_thread.iter() {
            let thread_color = rgb_color(*thread_idx, stats.stats_by_thread.len());
            let thread_durations_box_plot = BoxPlot::new(thread_stats.durations.clone())
                .name(thread_idx.to_string().as_str())
                .jitter(0.7)
                .marker(Marker::new().color(thread_color).size(6))
                .box_mean(BoxMean::StandardDeviation)
                .box_points(BoxPoints::All)
                .line(Line::new().width(2.0));

            plot.add_trace(thread_durations_box_plot);
        }
    }

    plot.set_layout(layout);
    plot.add_trace(trace_durations_box_plot);

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

    let n_buckets = 30;
    let bins = Bins::new(
        stats.min,
        stats.max,
        (stats.max - stats.min) / n_buckets as f64,
    );

    let total_histogram = Histogram::new(stats.durations.clone())
        .hist_norm(HistNorm::Probability)
        .name("total")
        .marker(Marker::new().color(NamedColor::Blue))
        .x_bins(bins.clone());

    plot.add_trace(total_histogram);

    if stats.stats_by_thread.len() > 1 {
        for (thread_idx, thread_stats) in stats.stats_by_thread.iter() {
            let thread_color = rgb_color(*thread_idx, stats.stats_by_thread.len());
            let thread_hist = Histogram::new(thread_stats.durations.clone())
                .name(thread_idx.to_string().as_str())
                .hist_norm(HistNorm::Probability)
                .opacity(0.5)
                .marker(Marker::new().color(thread_color))
                .x_bins(bins.clone());
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
