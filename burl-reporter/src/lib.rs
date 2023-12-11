mod html_report;
mod plots;
mod report;
mod stats_helpers;

use std::path::Path;

use burl::BurlResult;
pub use report::ReportFactory;

// pub trait ComponentCreator {
//     fn init() -> Self;
//     // fn add(&mut self, content: &Self::Content);
// }

pub trait ComponentBuilder<Content> {
    fn add(&self, content: &Content) -> &Self;
}

pub trait ComponentWriter {
    fn write(&self, file: &Path) -> BurlResult<()>;
}
