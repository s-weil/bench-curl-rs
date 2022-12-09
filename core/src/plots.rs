use std::path;

use crate::stats::Stats;
use log::info;
use plotly::common::{DashType, Line, Marker, Mode, Title};
use plotly::layout::{
    Axis, Layout, Legend, RangeSelector, RangeSlider, SelectorButton, SelectorStep, StepMode,
    TicksDirection,
};
use plotly::{BoxPlot, Histogram, Plot, Scatter};

/// https://github.com/igiagkiozis/plotly/blob/master/examples/statistical_charts/src/main.rs

pub fn plot(stats: Stats, output_path: Option<String>) {
    info!("plotting");
    let trace = Histogram::new(stats.distribution).name("h");
    let mut plot = Plot::new();
    plot.add_trace(trace);

    // let trace1 = BoxPlot::new(vec![1, 2, 3, 4, 4, 4, 8, 9, 10]).name("Set 1");
    // plot.add_trace(trace1);

    if let Some(path) = output_path {
        // plot.write_html("out.html");
        // let file_name = path::Path::new(&path).join("histogram.html");
        // info!("saved plot to {:?}", file_name.as_os_str().to_str());
        // let filename = file_name.as_os_str().to_str().unwrap();
        // plot.write_html(filename);
        // info!("saved plot to {}", &path);
    }

    plot.show();
}
