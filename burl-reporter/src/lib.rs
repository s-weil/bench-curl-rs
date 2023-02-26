mod html_report;
mod plots;
mod report;
mod stats_helpers;

pub(crate) use html_report::{write_baseline_summary_html, write_summary_html};
pub use report::ReportFactory;
