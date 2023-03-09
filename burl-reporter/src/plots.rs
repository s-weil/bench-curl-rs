use crate::{ComponentBuilder, ComponentCreator, ComponentWriter};
use burl::stats::{StatsSummary, ThreadStats};
use burl::ThreadIdx;
use plotly::box_plot::{BoxMean, BoxPoints};
use plotly::common::{Line, LineShape, Marker, Mode, Title};
use plotly::histogram::{Bins, HistNorm};
use plotly::layout::{Axis, BarMode};
use plotly::{BoxPlot, Histogram, Layout, NamedColor, Plot, Rgb, Scatter};
use std::collections::HashMap;
use std::ops::Deref;
use std::path::PathBuf;

impl ComponentWriter for Plot {
    fn write(&self, file: PathBuf) -> burl::BurlResult<()> {
        self.to_html(file);
        Ok(())
    }
}

/// https://github.com/igiagkiozis/plotly/blob/master/examples/statistical_charts/src/main.rs///
/// https://igiagkiozis.github.io/plotly/content/recipes/statistical_charts/box_plots.html

fn rgb_color(thread_idx: usize, n_threads: usize) -> Rgb {
    let min = 50;
    let max = 255;
    let step_size = (max - min) / n_threads;
    let scale = (min + thread_idx * step_size) as u8;
    Rgb::new(scale, min as u8, scale)
}

pub struct BoxPlotComponent {
    plot: Plot, // durations: &'a [f64],
                // stats_by_thread: &'a Hashmap<ThreadIdx, ThreadStats>,
}

impl Deref for BoxPlotComponent {
    type Target = Plot;
    fn deref(&self) -> &Self::Target {
        &self.plot
    }
}

impl ComponentCreator for BoxPlotComponent {
    type Content = StatsSummary;
    fn init() -> Self {
        let mut box_plot = BoxPlotComponent { plot: Plot::new() };
        box_plot.set_layout();
        box_plot
    }

    fn add(&mut self, content: &Self::Content) {
        self.add_total(&content.durations);
        if content.stats_by_thread.len() > 1 {
            self.add_threads(&content.stats_by_thread);
        }
    }
}

// impl GraphComponent for BoxPlotComponent {
//     fn set_layout(&mut self) {
//         self.set_layout()
//     }
// }

impl BoxPlotComponent {
    fn set_layout(&mut self) {
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
        self.plot.set_layout(layout);
    }

    fn add_total(&mut self, durations: &Vec<f64>) {
        let trace_durations_box_plot = BoxPlot::new(durations.clone())
            .name("total")
            .jitter(0.7)
            .marker(Marker::new().color(Rgb::new(7, 40, 89)).size(6))
            .box_mean(BoxMean::StandardDeviation)
            .box_points(BoxPoints::All)
            .line(Line::new().width(2.0));

        self.plot.add_trace(trace_durations_box_plot);
    }

    fn add_threads(&mut self, stats_by_thread: &HashMap<ThreadIdx, ThreadStats>) {
        for (thread_idx, thread_stats) in stats_by_thread.iter() {
            let thread_color = rgb_color(*thread_idx, stats_by_thread.len());
            let thread_durations_box_plot = BoxPlot::new(thread_stats.durations.clone())
                .name(thread_idx.to_string().as_str())
                .jitter(0.7)
                .marker(Marker::new().color(thread_color).size(6))
                .box_mean(BoxMean::StandardDeviation)
                .box_points(BoxPoints::All)
                .line(Line::new().width(2.0));

            self.plot.add_trace(thread_durations_box_plot);
        }
    }
}

// pub fn plot_box_plot(
//     durations: &[f64],
//     stats_by_thread: &'a Hashmap<ThreadIdx, ThreadStats>,
//     output_path: &Option<PathBuf>,
// ) {
//     let mut plot = Plot::new();

//     let layout = Layout::new()
//         .title(Title::new("Durations box plot"))
//         .y_axis(
//             Axis::new()
//                 .title(Title::new("durations"))
//                 .show_grid(true)
//                 .zero_line(true)
//                 .grid_width(1)
//                 .zero_line_width(2),
//         );

//     let trace_durations_box_plot = BoxPlot::new(stats.durations.clone())
//         .name("total")
//         .jitter(0.7)
//         .marker(Marker::new().color(Rgb::new(7, 40, 89)).size(6))
//         .box_mean(BoxMean::StandardDeviation)
//         .box_points(BoxPoints::All)
//         .line(Line::new().width(2.0));

//     if stats.stats_by_thread.len() > 1 {
//         for (thread_idx, thread_stats) in stats.stats_by_thread.iter() {
//             let thread_color = rgb_color(*thread_idx, stats.stats_by_thread.len());
//             let thread_durations_box_plot = BoxPlot::new(thread_stats.durations.clone())
//                 .name(thread_idx.to_string().as_str())
//                 .jitter(0.7)
//                 .marker(Marker::new().color(thread_color).size(6))
//                 .box_mean(BoxMean::StandardDeviation)
//                 .box_points(BoxPoints::All)
//                 .line(Line::new().width(2.0));

//             plot.add_trace(thread_durations_box_plot);
//         }
//     }

//     plot.set_layout(layout);
//     plot.add_trace(trace_durations_box_plot);

//     if let Some(path) = output_path {
//         let file_name = path.join("durations_distribution.html");
//         plot.to_html(file_name);
//     } else {
//         plot.show();
//     }
// }

pub struct HistogramComponent {
    plot: Plot,
}

impl HistogramComponent {
    pub fn new() -> Self {
        let mut histogram = HistogramComponent { plot: Plot::new() };
        histogram.set_layout();
        histogram
    }

    fn set_layout(&mut self) {
        let layout = Layout::new()
            .bar_mode(BarMode::Overlay)
            .title(Title::new("Durations frequency distribution"))
            .x_axis(Axis::new().title(Title::new("durations")).zero_line(true))
            .y_axis(Axis::new().title(Title::new("frequency")).zero_line(true));
        self.plot.set_layout(layout);
    }

    fn bins(&self, stats: &StatsSummary) -> Bins {
        let n_buckets = 30;
        let bins = Bins::new(
            stats.min,
            stats.max,
            (stats.max - stats.min) / n_buckets as f64,
        );
        bins
    }

    fn add_total(&mut self, durations: &Vec<f64>, bins: &Bins) {
        let total_histogram = Histogram::new(durations.clone())
            .hist_norm(HistNorm::Probability)
            .name("total")
            .marker(Marker::new().color(NamedColor::Blue))
            .x_bins(bins.clone());

        self.plot.add_trace(total_histogram);
    }

    fn add_threads(&mut self, stats_by_thread: &HashMap<ThreadIdx, ThreadStats>, bins: &Bins) {
        for (thread_idx, thread_stats) in stats_by_thread.iter() {
            let thread_color = rgb_color(*thread_idx, stats_by_thread.len());
            let thread_hist = Histogram::new(thread_stats.durations.clone())
                .name(thread_idx.to_string().as_str())
                .hist_norm(HistNorm::Probability)
                .opacity(0.5)
                .marker(Marker::new().color(thread_color))
                .x_bins(bins.clone());
            self.plot.add_trace(thread_hist);
        }
    }
}

impl Deref for HistogramComponent {
    type Target = Plot;
    fn deref(&self) -> &Self::Target {
        &self.plot
    }
}

type Durations = Vec<f64>;
impl ComponentBuilder<Durations> for HistogramComponent {
    fn add(&self, content: &Durations) -> Self {}
}

impl ComponentCreator for HistogramComponent {
    type Content = StatsSummary;
    fn init() -> Self {
        let mut histogram = HistogramComponent { plot: Plot::new() };
        histogram.set_layout();
        histogram
    }

    fn add(&mut self, content: &Self::Content) {
        let bins = self.bins(&content);
        self.add_total(&content.durations, &bins);
        if content.stats_by_thread.len() > 1 {
            self.add_threads(&content.stats_by_thread, &bins);
        }
    }
}

// pub fn plot_histogram(stats: &StatsSummary, output_path: &Option<PathBuf>) {
//     let mut plot = Plot::new();

//     let layout = Layout::new()
//         .bar_mode(BarMode::Overlay)
//         .title(Title::new("Durations frequency distribution"))
//         .x_axis(Axis::new().title(Title::new("durations")).zero_line(true))
//         .y_axis(Axis::new().title(Title::new("frequency")).zero_line(true));
//     plot.set_layout(layout);

//     // TODO: consider to split total and thread histograms, the latter being stacked

//     let n_buckets = 30;
//     let bins = Bins::new(
//         stats.min,
//         stats.max,
//         (stats.max - stats.min) / n_buckets as f64,
//     );

//     let total_histogram = Histogram::new(stats.durations.clone())
//         .hist_norm(HistNorm::Probability)
//         .name("total")
//         .marker(Marker::new().color(NamedColor::Blue))
//         .x_bins(bins.clone());

//     plot.add_trace(total_histogram);

//     if stats.stats_by_thread.len() > 1 {
//         for (thread_idx, thread_stats) in stats.stats_by_thread.iter() {
//             let thread_color = rgb_color(*thread_idx, stats.stats_by_thread.len());
//             let thread_hist = Histogram::new(thread_stats.durations.clone())
//                 .name(thread_idx.to_string().as_str())
//                 .hist_norm(HistNorm::Probability)
//                 .opacity(0.5)
//                 .marker(Marker::new().color(thread_color))
//                 .x_bins(bins.clone());
//             plot.add_trace(thread_hist)
//         }
//     }

//     if let Some(path) = output_path {
//         let file_name = path.join("durations_histogram.html");
//         plot.to_html(file_name);
//     } else {
//         plot.show();
//     }
// }

// TOOD: extract histogram logic and re-use it above
pub fn plot_bs_histogram(
    bs_means: &[f64],
    (lower_bound, upper_bound): (f64, f64),
    output_path: &Option<PathBuf>,
) {
    let mut plot = Plot::new();

    let layout = Layout::new()
        .bar_mode(BarMode::Overlay)
        .title(Title::new("Bootstrap mean distribution"))
        .x_axis(
            Axis::new()
                .title(Title::new("mean durations"))
                .zero_line(true),
        )
        .y_axis(Axis::new().title(Title::new("frequency")).zero_line(true));
    plot.set_layout(layout);

    let total_histogram = Histogram::new(bs_means.to_owned())
        .hist_norm(HistNorm::Probability)
        .name("mean duration")
        .marker(Marker::new().color(NamedColor::Blue));
    plot.add_trace(total_histogram);

    // plot the confidence interval bounds
    let ys_vertical = vec![0.0, 0.1];
    let lb = vec![lower_bound, lower_bound];
    let ub = vec![upper_bound, upper_bound];
    let lb_trace = Scatter::new(lb, ys_vertical.clone())
        .name("lower confidence bound")
        .mode(Mode::Lines);
    plot.add_trace(lb_trace);
    let ub_trace = Scatter::new(ub, ys_vertical)
        .name("upper confidence bound")
        .mode(Mode::Lines);
    plot.add_trace(ub_trace);

    if let Some(path) = output_path {
        let file_name = path.join("bootstrap_histogram.html");
        plot.to_html(file_name);
    } else {
        plot.show();
    }
}

pub fn plot_time_series(
    ts_by_thread: &HashMap<ThreadIdx, Vec<(f64, f64)>>,
    output_path: &Option<PathBuf>,
) {
    let mut plot = Plot::new();

    for (thread_idx, ts) in ts_by_thread.iter() {
        let mut ts_dates: Vec<f64> = Vec::with_capacity(ts.len());
        let mut ts_values = Vec::with_capacity(ts.len());

        for (time, v) in ts.iter() {
            ts_dates.push(*time);
            ts_values.push(*v);
        }

        let thread_color = rgb_color(*thread_idx, ts_by_thread.len());

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

pub fn plot_qq_curve(
    qq_curve: &Vec<(f64, f64)>,
    baseline_qq_curve: Option<&Vec<(f64, f64)>>,
    output_path: &Option<PathBuf>,
) {
    let mut plot = Plot::new();

    let mut x_percentiles: Vec<f64> = Vec::with_capacity(qq_curve.len());
    let mut y_percentiles = Vec::with_capacity(qq_curve.len());

    for (x, y) in qq_curve.iter() {
        x_percentiles.push(*x);
        y_percentiles.push(*y);
    }

    let qq_trace = Scatter::new(x_percentiles.clone(), y_percentiles)
        .mode(Mode::Markers)
        .name("current run")
        // .line(Line::new().shape(LineShape::Hv))
        .marker(Marker::new().color(Rgb::new(0, 0, 200)));
    plot.add_trace(qq_trace);

    let mut reference_line: Vec<f64> = vec![];
    reference_line.extend(x_percentiles.iter());

    if let Some(bl_qq) = baseline_qq_curve {
        let mut x_percentiles: Vec<f64> = Vec::with_capacity(bl_qq.len());
        let mut y_percentiles = Vec::with_capacity(bl_qq.len());

        for (x, y) in bl_qq.iter() {
            x_percentiles.push(*x);
            y_percentiles.push(*y);
        }

        // the 45° line
        // let reference_trace = Scatter::new(x_percentiles.clone(), x_percentiles.clone())
        //     .mode(Mode::Lines)
        //     .name("")
        //     .marker(Marker::new().color(Rgb::new(0, 200, 0)));
        // plot.add_trace(reference_trace);

        let baseline_qq_trace = Scatter::new(x_percentiles.clone(), y_percentiles)
            .mode(Mode::Markers)
            .name("baseline")
            .marker(Marker::new().color(Rgb::new(200, 0, 0)));
        plot.add_trace(baseline_qq_trace);

        reference_line.extend(x_percentiles.iter());
    }

    // the 45° line
    let reference_trace = Scatter::new(reference_line.clone(), reference_line.clone())
        .mode(Mode::Lines)
        .name("")
        .marker(Marker::new().color(Rgb::new(0, 200, 0)));
    plot.add_trace(reference_trace);

    let ts_layout = Layout::new()
        .title(Title::new("QQ Plot"))
        .x_axis(
            Axis::new()
                .title(Title::new("percentiles of normal distribution"))
                .zero_line(true),
        )
        .y_axis(
            Axis::new()
                .title(Title::new("percentile of duration distribution"))
                .zero_line(true),
        );
    plot.set_layout(ts_layout);

    if let Some(path) = output_path {
        let file_name = path.join("qq_plot.html");
        plot.to_html(file_name);
    } else {
        plot.show();
    }
}
