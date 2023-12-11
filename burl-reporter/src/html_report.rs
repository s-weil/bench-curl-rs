use crate::{stats_helpers::StatisticalTester, ComponentWriter};
use burl::stats::{StatsSummary, TestOutcome};
use std::{fs, path::Path};

fn test_outcome_html(test_outcome: &TestOutcome) -> String {
    match test_outcome {
        TestOutcome::Improved { p_value } => {
            format!("<font color='green'>improved (p-value {})</font>", p_value)
        }
        TestOutcome::Regressed { p_value } => {
            format!("<font color='red'>regressed (p-value {})</font>", p_value)
        }
        TestOutcome::Inconclusive => "inconclusive (no significant change)".to_string(),
    }
}

// TODO: refactor, too much state updates & intransparent
pub struct SummaryComponent<'a> {
    html: String,
    current_stats: Option<&'a StatsSummary>,
    baseline_stats: Option<StatsSummary>,
}

impl<'a> ComponentWriter for SummaryComponent<'a> {
    fn write(&self, file: &Path) -> burl::BurlResult<()> {
        fs::write(file, &self.html)?;
        Ok(())
    }
}

impl<'a> SummaryComponent<'a> {
    pub fn new() -> Self {
        Self {
            html: include_str!("./templates/summary_template.html").to_string(),
            current_stats: None,
            baseline_stats: None,
        }
    }

    fn update_current(&mut self, stats: &StatsSummary) {
        self.html = self
            .html
            .replace("$SCALE$", stats.scale.clone().to_string().as_str());

        let mut replace_key_value =
            |(key, v): (&str, f64)| self.html = self.html.replace(key, v.to_string().as_str());

        // TODO: add JS to summary template instead
        replace_key_value(("$TOTAL_BYTES$", stats.total_bytes as f64));
        replace_key_value(("$N_OK$", stats.n_ok as f64));
        replace_key_value(("$N_FAILED$", stats.n_errors as f64));
        replace_key_value(("$N_THREADS$", stats.stats_by_thread.len() as f64));
        replace_key_value(("$TOTAL_DURATION$", stats.total_duration));
        replace_key_value(("$MEAN$", stats.mean));
        replace_key_value(("$RPS$", stats.mean_rps.unwrap_or(f64::NAN)));
        replace_key_value(("$STDEV$", stats.std.unwrap_or(f64::NAN)));
        replace_key_value(("$MIN$", stats.min));
        replace_key_value(("$MAX$", stats.max));
        replace_key_value(("$Q1$", stats.quartile_fst));
        replace_key_value(("$Q2$", stats.median));
        replace_key_value(("$Q3$", stats.quartile_trd));
    }

    fn update_baseline(
        &mut self,
        stats: StatsSummary,
        stats_tester: Option<StatisticalTester>,
        alpha: f64,
        n_bootstrap_samples: usize,
    ) {
        self.html = self
            .html
            .replace("$SCALE_BASELINE$", stats.scale.clone().to_string().as_str());

        match stats_tester {
            Some(tester) => {
                let performance_outcome_disp = match tester.analytic_test(alpha) {
                    Some(outcome) => test_outcome_html(&outcome),
                    None => "could not be determined".to_string(),
                };
                self.html = self
                    .html
                    .replace("$PERFORMANCE_OUTCOME$", performance_outcome_disp.as_str());

                let permutation_outcome_disp =
                    match tester.performance_test(n_bootstrap_samples, alpha) {
                        Some(outcome) => test_outcome_html(&outcome),
                        None => "could not be determined".to_string(),
                    };
                self.html = self.html.replace(
                    "$PERMUTATION_PERFORMANCE_OUTCOME$",
                    permutation_outcome_disp.as_str(),
                );
            }

            None => {
                self.html = self.html.replace(
                    "$PERFORMANCE_OUTCOME$",
                    "cannot be compared due to different time scales",
                );
                self.html = self.html.replace(
                    "$PERMUTATION_PERFORMANCE_OUTCOME$",
                    "cannot be compared due to different time scales",
                );
            }
        }

        let mut replace_key_value =
            |(key, v): (&str, f64)| self.html = self.html.replace(key, v.to_string().as_str());

        // TODO: add JS to summary template instead
        replace_key_value(("$TOTAL_BYTES_BASELINE$", stats.total_bytes as f64));
        replace_key_value(("$N_OK_BASELINE$", stats.n_ok as f64));
        replace_key_value(("$N_FAILED_BASELINE$", stats.n_errors as f64));
        replace_key_value(("$N_THREADS_BASELINE$", stats.stats_by_thread.len() as f64));
        replace_key_value(("$TOTAL_DURATION_BASELINE$", stats.total_duration));
        replace_key_value(("$MEAN_BASELINE$", stats.mean));
        replace_key_value(("$RPS_BASELINE$", stats.mean_rps.unwrap_or(f64::NAN)));
        replace_key_value(("$STDEV_BASELINE$", stats.std.unwrap_or(f64::NAN)));
        replace_key_value(("$MIN_BASELINE$", stats.min));
        replace_key_value(("$MAX_BASELINE$", stats.max));
        replace_key_value(("$Q1_BASELINE$", stats.quartile_fst));
        replace_key_value(("$Q2_BASELINE$", stats.median));
        replace_key_value(("$Q3_BASELINE$", stats.quartile_trd));
    }

    pub fn add_current(&mut self, stats: &'a StatsSummary) {
        // cannot yet udpate the template string, as baseline stats might be added which requires another template
        self.current_stats = Some(stats);
    }

    pub fn add_baseline(&mut self, stats: StatsSummary) {
        self.html = include_str!("./templates/baseline_summary_template.html").to_string();
        self.baseline_stats = Some(stats);
    }

    pub fn compile(&mut self, _alpha: f64, _n_bootstrap_samples: usize) {
        if let Some(stats) = self.current_stats {
            self.update_current(stats);

            // if let Some(baseline_stats) = &self.baseline_stats {
            //     let stats_tester = StatisticalTester::try_new(stats, &baseline_stats);
            //     self.update_baseline(
            //         baseline_stats.clone(),
            //         stats_tester,
            //         alpha,
            //         n_bootstrap_samples,
            //     );
            // }
        }
    }
}

// pub(crate) fn write_baseline_summary_html(
//     stats: &StatsSummary,
//     baseline_stats: &StatsSummary,
//     n_bootstrap_samples: usize,
//     alpha: f64,
//     file: PathBuf,
// ) -> BurlResult<()> {
//     let mut template = include_str!("./templates/baseline_summary_template.html").to_string();
//     template = template.replace("$SCALE$", stats.scale.clone().to_string().as_str());
//     template = template.replace(
//         "$SCALE_BASELINE$",
//         baseline_stats.scale.clone().to_string().as_str(),
//     );

//     let stats_tester = StatisticalTester::try_new(stats, baseline_stats);
//     match stats_tester {
//         Some(tester) => {
//             let performance_outcome_disp = match tester.analytic_test(alpha) {
//                 Some(outcome) => test_outcome_html(&outcome),
//                 None => "could not be determined".to_string(),
//             };
//             template = template.replace("$PERFORMANCE_OUTCOME$", performance_outcome_disp.as_str());

//             let permutation_outcome_disp = match tester.performance_test(n_bootstrap_samples, alpha)
//             {
//                 Some(outcome) => test_outcome_html(&outcome),
//                 None => "could not be determined".to_string(),
//             };
//             template = template.replace(
//                 "$PERMUTATION_PERFORMANCE_OUTCOME$",
//                 permutation_outcome_disp.as_str(),
//             );
//         }

//         None => {
//             template = template.replace(
//                 "$PERFORMANCE_OUTCOME$",
//                 "cannot be compared due to different time scales",
//             );
//             template = template.replace(
//                 "$PERMUTATION_PERFORMANCE_OUTCOME$",
//                 "cannot be compared due to different time scales",
//             );
//         }
//     }

//     let mut replace_key_value =
//         |(key, v): (&str, f64)| template = template.replace(key, v.to_string().as_str());

//     // TODO: add JS to summary template instead
//     replace_key_value(("$TOTAL_BYTES$", stats.total_bytes as f64));
//     replace_key_value(("$TOTAL_BYTES_BASELINE$", baseline_stats.total_bytes as f64));
//     replace_key_value(("$N_OK$", stats.n_ok as f64));
//     replace_key_value(("$N_OK_BASELINE$", baseline_stats.n_ok as f64));
//     replace_key_value(("$N_FAILED$", stats.n_errors as f64));
//     replace_key_value(("$N_FAILED_BASELINE$", baseline_stats.n_errors as f64));
//     replace_key_value(("$N_THREADS$", stats.stats_by_thread.len() as f64));
//     replace_key_value((
//         "$N_THREADS_BASELINE$",
//         baseline_stats.stats_by_thread.len() as f64,
//     ));
//     replace_key_value(("$TOTAL_DURATION$", stats.total_duration));
//     replace_key_value(("$TOTAL_DURATION_BASELINE$", baseline_stats.total_duration));
//     replace_key_value(("$MEAN$", stats.mean));
//     replace_key_value(("$MEAN_BASELINE$", baseline_stats.mean));
//     replace_key_value(("$RPS$", stats.mean_rps.unwrap_or(f64::NAN)));
//     replace_key_value((
//         "$RPS_BASELINE$",
//         baseline_stats.mean_rps.unwrap_or(f64::NAN),
//     ));
//     replace_key_value(("$STDEV$", stats.std.unwrap_or(f64::NAN)));
//     replace_key_value(("$STDEV_BASELINE$", baseline_stats.std.unwrap_or(f64::NAN)));
//     replace_key_value(("$MIN$", stats.min));
//     replace_key_value(("$MIN_BASELINE$", baseline_stats.min));
//     replace_key_value(("$MAX$", stats.max));
//     replace_key_value(("$MAX_BASELINE$", baseline_stats.max));
//     replace_key_value(("$Q1$", stats.quartile_fst));
//     replace_key_value(("$Q1_BASELINE$", baseline_stats.quartile_fst));
//     replace_key_value(("$Q2$", stats.median));
//     replace_key_value(("$Q2_BASELINE$", baseline_stats.median));
//     replace_key_value(("$Q3$", stats.quartile_trd));
//     replace_key_value(("$Q3_BASELINE$", baseline_stats.quartile_trd));

//     fs::write(file, template)?;
//     Ok(())
// }

// pub(crate) fn write_summary_html(stats: &StatsSummary, file: PathBuf) -> BurlResult<()> {
// let mut template = include_str!("./templates/summary_template.html").to_string();
// template = template.replace("$SCALE$", stats.scale.clone().to_string().as_str());

// let mut replace_key_value =
// |(key, v): (&str, f64)| template = template.replace(key, v.to_string().as_str());

// // TODO: add JS to summary template instead
// replace_key_value(("$TOTAL_BYTES$", stats.total_bytes as f64));
// replace_key_value(("$N_OK$", stats.n_ok as f64));
// replace_key_value(("$N_FAILED$", stats.n_errors as f64));
// replace_key_value(("$N_THREADS$", stats.stats_by_thread.len() as f64));
// replace_key_value(("$TOTAL_DURATION$", stats.total_duration));
// replace_key_value(("$MEAN$", stats.mean));
// replace_key_value(("$RPS$", stats.mean_rps.unwrap_or(f64::NAN)));
// replace_key_value(("$STDEV$", stats.std.unwrap_or(f64::NAN)));
// replace_key_value(("$MIN$", stats.min));
// replace_key_value(("$MAX$", stats.max));
// replace_key_value(("$Q1$", stats.quartile_fst));
// replace_key_value(("$Q2$", stats.median));
// replace_key_value(("$Q3$", stats.quartile_trd));

// fs::write(file, template)?;
// Ok(())
// }
