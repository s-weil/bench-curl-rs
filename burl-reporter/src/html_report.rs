use burl::{
    stats::{AnalyticTester, NormalParams, PermutationTester, StatsSummary, TestOutcome},
    BurlResult,
};
use std::{fs, path::PathBuf};

// impl TestOutcome {
//     pub fn to_html(&self) -> String {
//         match self {
//             TestOutcome::Improved { p_value } => {
//                 format!("<font color='green'>improved (p-value {})</font>", p_value)
//             }
//             TestOutcome::Regressed { p_value } => {
//                 format!("<font color='red'>regressed (p-value {})</font>", p_value)
//             }
//             TestOutcome::Inconclusive => "inconclusive (no significant change)".to_string(),
//         }
//     }
// }

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
    alpha: f64,
    file: PathBuf,
) -> BurlResult<()> {
    let mut template = include_str!("./templates/baseline_summary_template.html").to_string();
    template = template.replace("$SCALE$", stats.scale.clone().to_string().as_str());
    template = template.replace(
        "$SCALE_BASELINE$",
        baseline_stats.scale.clone().to_string().as_str(),
    );

    // TODO: add it also to console

    //TODO:
    if stats.scale == baseline_stats.scale {
        let np = NormalParams::from(stats);
        let np_baseline = NormalParams::from(baseline_stats);
        let analytic_test = AnalyticTester::new(&np_baseline, &np);
        let performance_outcome = analytic_test.test(alpha);
        let performance_outcome_disp = match performance_outcome {
            Some(outcome) => test_outcome_html(&outcome),
            None => "could not be determined".to_string(),
        };
        template = template.replace("$PERFORMANCE_OUTCOME$", performance_outcome_disp.as_str());

        let permutation_tester =
            PermutationTester::new(&stats.durations, &baseline_stats.durations);
        let permutation_outcome = permutation_tester.test(1000, alpha);
        let permutation_outcome_disp = match permutation_outcome {
            Some(outcome) => test_outcome_html(&outcome),
            None => "could not be determined".to_string(),
        };
        template = template.replace(
            "$PERMUTATION_PERFORMANCE_OUTCOME$",
            permutation_outcome_disp.as_str(),
        );
    } else {
        template = template.replace(
            "$PERFORMANCE_OUTCOME$",
            "cannot be compared due to different time scales"
                .to_string()
                .as_str(),
        );
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
