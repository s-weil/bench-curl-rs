use std::path;

use crate::stats::Stats;
use log::info;
use plotly::box_plot::BoxPoints;
use plotly::common::{Marker, Title};
use plotly::layout::{Axis, BoxMode, Layout};
use plotly::{BoxPlot, Plot, Rgb};

/// https://github.com/igiagkiozis/plotly/blob/master/examples/statistical_charts/src/main.rs///
/// https://igiagkiozis.github.io/plotly/content/recipes/statistical_charts/box_plots.html
///

// TODO: add plotoptions with outputpath, duration scale, title etc

pub fn plot(stats: Stats, output_path: Option<String>) {
    info!("plotting");
    // let trace = Histogram::new(stats.distribution).name("h");
    let mut plot = Plot::new();
    let layout = Layout::new()
        .title(Title::new("Box Plot"))
        .y_axis(
            Axis::new()
                .title(Title::new("durations [unit]"))
                .zero_line(true),
        )
        .box_mode(BoxMode::Group);
    plot.set_layout(layout);

    let trace_all = BoxPlot::new(stats.distribution)
        .name("")
        .jitter(0.7)
        .point_pos(-1.8)
        .marker(Marker::new().color(Rgb::new(7, 40, 89)))
        .box_points(BoxPoints::All);
    plot.add_trace(trace_all);

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
