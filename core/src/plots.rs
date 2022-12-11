use std::path;

use crate::stats::Stats;
use log::info;
use plotly::box_plot::BoxPoints;
use plotly::common::{Line, LineShape, Marker, Mode, Title};
use plotly::layout::{Axis, BoxMode, Layout};
use plotly::{BoxPlot, Plot, Rgb, Scatter};

/// https://github.com/igiagkiozis/plotly/blob/master/examples/statistical_charts/src/main.rs///
/// https://igiagkiozis.github.io/plotly/content/recipes/statistical_charts/box_plots.html
///

// TODO: add plotoptions with outputpath, duration scale, title etc

pub fn plot(stats: Stats, output_path: Option<String>) {
    info!("plotting");
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
    // plot.set_layout(box_plot_layout);

    let trace_all = BoxPlot::new(stats.distribution)
        .name("")
        .jitter(0.7)
        .point_pos(-1.8)
        .marker(Marker::new().color(Rgb::new(7, 40, 89)))
        .box_points(BoxPoints::All);
    // plot.add_trace(trace_all);

    let mut ts_dates: Vec<f64> = Vec::with_capacity(stats.time_series.len());
    let mut ts_values = Vec::with_capacity(stats.time_series.len());

    for (date, value) in stats.time_series {
        ts_dates.push(date);
        ts_values.push(value);
    }

    let trace_ts = Scatter::new(ts_dates, ts_values)
        .mode(Mode::LinesMarkers)
        .name("hv")
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
    // .legend(
    //     Legend::new()
    //         .y(0.5)
    //         .trace_order("reversed")
    //         .font(Font::new().size(16)),
    // );
    // let trace_box = BoxPlot::new(stats.distribution.clone())
    //     .name("Suspected Outlier")
    //     .marker(
    //         Marker::new()
    //             .color(Rgb::new(0, 0, 156))
    //             .outlier_color(Rgba::new(219, 64, 82, 0.6))
    //             .line(
    //                 Line::new()
    //                     .outlier_color(Rgba::new(219, 64, 82, 1.0))
    //                     .outlier_width(2),
    //             ),
    //     )
    //     .box_points(BoxPoints::SuspectedOutliers);
    // // .box_mean(BoxMean::False)
    // // .orientation(Orientation::Horizontal);

    // plot.add_trace(trace_box);

    // let trace1 = BoxPlot::new(stats.distribution).name("Distribution");
    // plot.add_trace(trace1);
    // plot.add_trace(trace);

    if let Some(path) = output_path {
        // TODO: add title
        let file_name = path::Path::new(&path).join("box_plot.html");
        plot.to_html(file_name);
        info!("Saved plot to {}", &path);
    } else {
        plot.show();
    }
}
