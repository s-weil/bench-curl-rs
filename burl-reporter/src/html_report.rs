use crate::stats_helpers::StatisticalTester;
use burl::{
    stats::{StatsSummary, TestOutcome},
    BurlResult,
};
use std::{fs, path::PathBuf};

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

pub(crate) fn write_summary_html(stats: &StatsSummary, file: PathBuf) -> BurlResult<()> {
    let mut template = include_str!("./templates/summary_template.html").to_string();
    template = template.replace("$SCALE$", stats.scale.clone().to_string().as_str());

    let mut replace_key_value =
        |(key, v): (&str, f64)| template = template.replace(key, v.to_string().as_str());

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

    fs::write(file, template)?;
    Ok(())
}

pub(crate) fn write_baseline_summary_html(
    stats: &StatsSummary,
    baseline_stats: &StatsSummary,
    n_bootstrap_samples: usize,
    alpha: f64,
    file: PathBuf,
) -> BurlResult<()> {
    let mut template = include_str!("./templates/baseline_summary_template.html").to_string();
    template = template.replace("$SCALE$", stats.scale.clone().to_string().as_str());
    template = template.replace(
        "$SCALE_BASELINE$",
        baseline_stats.scale.clone().to_string().as_str(),
    );

    let stats_tester = StatisticalTester::try_new(stats, baseline_stats);
    match stats_tester {
        Some(tester) => {
            let performance_outcome_disp = match tester.analytic_test(alpha) {
                Some(outcome) => test_outcome_html(&outcome),
                None => "could not be determined".to_string(),
            };
            template = template.replace("$PERFORMANCE_OUTCOME$", performance_outcome_disp.as_str());

            let permutation_outcome_disp = match tester.performance_test(n_bootstrap_samples, alpha)
            {
                Some(outcome) => test_outcome_html(&outcome),
                None => "could not be determined".to_string(),
            };
            template = template.replace(
                "$PERMUTATION_PERFORMANCE_OUTCOME$",
                permutation_outcome_disp.as_str(),
            );
        }

        None => {
            template = template.replace(
                "$PERFORMANCE_OUTCOME$",
                "cannot be compared due to different time scales",
            );
            template = template.replace(
                "$PERMUTATION_PERFORMANCE_OUTCOME$",
                "cannot be compared due to different time scales",
            );
        }
    }

    let mut replace_key_value =
        |(key, v): (&str, f64)| template = template.replace(key, v.to_string().as_str());

    // TODO: add JS to summary template instead
    replace_key_value(("$TOTAL_BYTES$", stats.total_bytes as f64));
    replace_key_value(("$TOTAL_BYTES_BASELINE$", baseline_stats.total_bytes as f64));
    replace_key_value(("$N_OK$", stats.n_ok as f64));
    replace_key_value(("$N_OK_BASELINE$", baseline_stats.n_ok as f64));
    replace_key_value(("$N_FAILED$", stats.n_errors as f64));
    replace_key_value(("$N_FAILED_BASELINE$", baseline_stats.n_errors as f64));
    replace_key_value(("$N_THREADS$", stats.stats_by_thread.len() as f64));
    replace_key_value((
        "$N_THREADS_BASELINE$",
        baseline_stats.stats_by_thread.len() as f64,
    ));
    replace_key_value(("$TOTAL_DURATION$", stats.total_duration));
    replace_key_value(("$TOTAL_DURATION_BASELINE$", baseline_stats.total_duration));
    replace_key_value(("$MEAN$", stats.mean));
    replace_key_value(("$MEAN_BASELINE$", baseline_stats.mean));
    replace_key_value(("$RPS$", stats.mean_rps.unwrap_or(f64::NAN)));
    replace_key_value((
        "$RPS_BASELINE$",
        baseline_stats.mean_rps.unwrap_or(f64::NAN),
    ));
    replace_key_value(("$STDEV$", stats.std.unwrap_or(f64::NAN)));
    replace_key_value(("$STDEV_BASELINE$", baseline_stats.std.unwrap_or(f64::NAN)));
    replace_key_value(("$MIN$", stats.min));
    replace_key_value(("$MIN_BASELINE$", baseline_stats.min));
    replace_key_value(("$MAX$", stats.max));
    replace_key_value(("$MAX_BASELINE$", baseline_stats.max));
    replace_key_value(("$Q1$", stats.quartile_fst));
    replace_key_value(("$Q1_BASELINE$", baseline_stats.quartile_fst));
    replace_key_value(("$Q2$", stats.median));
    replace_key_value(("$Q2_BASELINE$", baseline_stats.median));
    replace_key_value(("$Q3$", stats.quartile_trd));
    replace_key_value(("$Q3_BASELINE$", baseline_stats.quartile_trd));

    fs::write(file, template)?;
    Ok(())
}
