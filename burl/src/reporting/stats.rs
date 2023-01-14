use crate::{
    config::DurationScale,
    sampling::{RequestResult, SampleCollector, StatusCode},
    ThreadIdx,
};
use log::warn;
use serde::Serialize;
use statrs::distribution::ContinuousCDF;
use statrs::distribution::Normal;
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

fn sum(durations: &[f64]) -> f64 {
    durations.iter().fold(0.0, |acc, dur| acc + dur)
}

/// Calculates the [empirical percentile](https://en.wikipedia.org/wiki/Percentile).
/// Due to earlier validation, `durations` is a non-empty, sorted vector at this point and `n` > 0
fn percentile(samples: &[f64], level: f64, n: f64) -> f64 {
    // NOTE: have to add `-1` below due to (mathematical) idx start of 1 (rather than 0)
    let candidate_idx = n * level;
    let floored = candidate_idx.floor() as usize;

    // case candidate is an integer
    if candidate_idx == floored as f64 {
        let idx_bottom = (floored - 1).max(0);
        let idx_top = floored.min(n as usize);
        return 0.5 * (samples[idx_bottom] + samples[idx_top]);
    }
    let idx = ((candidate_idx + 1.0).floor().min(n) as usize - 1).max(0);
    samples[idx]
}

/// The biased sample standard deviation.
fn standard_deviation(samples: &[f64], mean: f64) -> Option<f64> {
    let len = samples.len();
    if len <= 1 {
        return None;
    }
    let squared_errors = samples.iter().fold(0.0, |acc, d| {
        let error = (d - mean).powi(2);
        acc + error
    });

    let mean_squared_errors = squared_errors / len as f64; //(len - 1) as f64; which version to go with, biased or unbiased?
    let std = mean_squared_errors.sqrt();
    Some(std)
}

struct NormalParams {
    mean: f64,
    std: f64,
    n_samples: usize,
}

#[derive(Debug, PartialEq)]
enum PerformanceOutcome {
    Regressed { p_value: f64 },
    Improved { p_value: f64 },
    NoChange,
}

/// We assume:
/// - the samples (of durations) to be independent, identical Gaussian random variables
/// - the number of samples (for each collection) to be sufficiently large, so that the estimated std deviations are good approximations
/// - the two sample collections (of the baseline and the the current run) to be independent with known standard deviations (see prev assumption)
fn test_statistics(np_base: &NormalParams, np: &NormalParams) -> Option<f64> {
    // the 'combined' standard deviation
    // let s2: f64 = ((np.n_samples as f64 - 1.0) * np.std.powi(2)
    //     + (np_base.n_samples as f64 - 1.0) * np_base.std.powi(2))
    //     * (((np.n_samples + np_base.n_samples)
    //         / (np.n_samples * np_base.n_samples * (np.n_samples + np_base.n_samples - 2)))
    //         as f64);

    // the 'combined' standard deviation
    let s2 =
        np_base.std.powi(2) / (np_base.n_samples as f64) + np.std.powi(2) / (np.n_samples as f64);

    if s2.abs() < 1.0e-12 {
        return None;
    }

    // value of the test-variable
    let t = (np_base.mean - np.mean) / s2.sqrt();
    Some(t)
}

fn unsigned_p_value(np_base: &NormalParams, np: &NormalParams) -> Option<f64> {
    // value of the test-variable
    let t = test_statistics(np_base, np)?;
    let n = Normal::new(0.0, 1.0).unwrap();
    let cdf_t = n.cdf(t.abs());
    let p_value = 1.0 - cdf_t;
    Some(p_value)
}
fn performance_outcome(
    np_base: &NormalParams,
    np: &NormalParams,
    alpha: f64,
) -> Option<PerformanceOutcome> {
    let p_value = unsigned_p_value(&np_base, &np)?;

    if p_value < alpha {
        return Some(PerformanceOutcome::NoChange);
    }

    // case of significant performance change
    if np_base.mean < np.mean {
        Some(PerformanceOutcome::Regressed { p_value })
    } else {
        Some(PerformanceOutcome::Improved { p_value })
    }
}

#[derive(Debug, Serialize)]
pub struct ThreadStats {
    #[serde(skip_serializing)]
    errors: HashMap<StatusCode, i32>,
    #[serde(skip_serializing)] // serialize or not?
    pub durations: Vec<f64>,

    pub total_bytes: u64,
    pub n_ok: usize,
    pub n_errors: usize,

    pub total_duration: Option<f64>,
    pub mean: Option<f64>,
    pub max: Option<f64>,
    pub min: Option<f64>,
    pub std: Option<f64>,
}

impl From<&SampleCollector> for ThreadStats {
    fn from(samples: &SampleCollector) -> Self {
        let mut durations = Vec::with_capacity(samples.n_runs);
        let mut errors = HashMap::new();
        let mut sample_results = Vec::with_capacity(samples.n_runs);

        let mut total_bytes = 0;
        let mut n_ok = 0;
        let mut n_errors = 0;
        let mut max = 0.0_f64;
        let mut min = f64::MAX;

        for result in samples.results.iter() {
            match result {
                RequestResult::Ok(duration_point) => {
                    sample_results.push(duration_point);
                    durations.push(duration_point.duration);
                    max = max.max(duration_point.duration);
                    min = min.min(duration_point.duration);
                    if let Some(bytes) = duration_point.content_length {
                        total_bytes += bytes;
                    }
                    n_ok += 1;
                }
                RequestResult::Failed(status_code) => {
                    errors
                        .entry(*status_code)
                        .and_modify(|count| *count += 1)
                        .or_insert(1);
                    n_errors += 1;
                }
            }
        }

        let n = durations.len();
        let (total_duration, mean, std, max, min) = if n > 0 {
            let sum = sum(&durations);
            let mean = sum / (n as f64);
            let std = standard_deviation(&durations, mean);
            (Some(sum), Some(mean), std, Some(max), Some(min))
        } else {
            (None, None, None, None, None)
        };

        Self {
            total_bytes,
            durations,
            errors,
            n_ok,
            n_errors,
            total_duration,
            mean,
            std,
            max,
            min,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Stats {
    // #[serde(skip_serializing_if = "Map::is_empty")]
    #[serde(skip_serializing)]
    pub errors: HashMap<StatusCode, i32>,
    #[serde(skip_serializing)] // serialize or not?
    pub durations: Vec<f64>,

    pub scale: DurationScale,
    pub total_duration: f64,
    pub total_bytes: u64,
    pub mean: f64,
    pub median: f64,
    pub quartile_fst: f64,
    pub quartile_trd: f64,
    pub min: f64,
    pub max: f64,
    pub std: Option<f64>,
    pub n_ok: usize,
    pub n_errors: usize,

    pub stats_by_thread: HashMap<ThreadIdx, ThreadStats>,
    /// Percentiles 1% 5% 10% 20% 30% 40% 50% 60% 70% 80% 90% 95% 99%
    percentiles: Vec<(f64, f64)>,
    // TODO: provide overview of errors - tbd if actually interestering or a corner case
    // TODO: outliers
}

impl Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        writeln!(
            f,
            "_______SUMMARY_[in {}s, on {} threads]___________",
            &self.scale,
            &self.stats_by_thread.len()
        )?;
        writeln!(f, "Total bytes      | {}", self.total_bytes)?;
        writeln!(f, "Number ok        | {}", self.n_ok)?;
        writeln!(f, "Number failed    | {}", self.n_errors)?;
        writeln!(f, "_______DURATIONS_______________________________")?;
        writeln!(f, "Total            | {}", self.total_duration)?;
        writeln!(f, "Mean             | {}", self.mean)?;
        // writeln!(f, "Requests per sec | {}", self.mean)?;

        if let Some(std) = self.std {
            writeln!(f, "StdDev           | {}", std)?;
        }
        writeln!(f, "Min              | {}", self.min)?;
        writeln!(f, "Quartile 1st     | {}", self.quartile_fst)?;
        writeln!(f, "Median           | {}", self.median)?;
        writeln!(f, "Quartile 3rd     | {}", self.quartile_trd)?;
        writeln!(f, "Max              | {}", self.max)?;

        if self.n_ok >= 12 {
            writeln!(f, "_______PERCENTILES_____________________________")?;
            for (level, percentile) in self.percentiles.iter() {
                writeln!(f, "{}%    {}", level, percentile)?;
            }
        }

        if self.stats_by_thread.len() > 1 {
            let format_option = |option_v: Option<f64>| {
                if let Some(v) = option_v {
                    v.round().to_string()
                } else {
                    "".to_string()
                }
            };

            writeln!(f, "_______THREADS_________________________________")?;
            writeln!(f, "(Idx : num ok) | total | mean | std | min | max")?;
            for (thread_idx, thread_stats) in self.stats_by_thread.iter() {
                writeln!(
                    f,
                    "({}: {}) | {} | {} | {} | {} | {}",
                    thread_idx,
                    thread_stats.n_ok,
                    format_option(thread_stats.total_duration),
                    format_option(thread_stats.mean),
                    format_option(thread_stats.std),
                    format_option(thread_stats.min),
                    format_option(thread_stats.max)
                )?;
            }
        }

        writeln!(f, "_______________________________________________")
    }
}

static PERCENTILE_LEVELS: [f64; 13] = [
    0.01, 0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.95, 0.99,
];

impl Stats {
    /// Collect the sample results from the threads' samples.
    pub fn collect(
        samples_by_thread: &Vec<SampleCollector>,
        duration_scale: DurationScale,
    ) -> Option<Self> {
        let mut durations = Vec::new();
        let mut stats_by_thread = HashMap::new();
        let mut total_bytes = 0;
        let mut n_errors = 0;
        let mut errors: HashMap<StatusCode, i32> = HashMap::new();

        for samples in samples_by_thread {
            let idx = samples.thread_idx;
            let thread_stats = ThreadStats::from(samples);

            n_errors += thread_stats.n_errors;
            total_bytes += thread_stats.total_bytes;

            durations.extend(thread_stats.durations.clone());

            for (status_code, n_errors) in thread_stats.errors.iter() {
                errors
                    .entry(*status_code)
                    .and_modify(|count| *count += *n_errors)
                    .or_insert(*n_errors);
            }

            stats_by_thread.insert(idx, thread_stats);
        }

        Self::calculate(
            duration_scale,
            n_errors,
            total_bytes,
            durations,
            errors,
            stats_by_thread,
        )
    }

    pub fn calculate(
        scale: DurationScale,
        n_errors: usize,
        total_bytes: u64,
        mut durations: Vec<f64>,
        errors: HashMap<StatusCode, i32>,
        stats_by_thread: HashMap<ThreadIdx, ThreadStats>,
    ) -> Option<Self> {
        let n = durations.len();
        if n == 0 {
            warn!(
                "Measurement yielded no valid results. Distribution of status codes: {:?}",
                errors
            );
            return None;
        }

        let sum = sum(&durations);
        let mean = sum / (n as f64);
        let std = standard_deviation(&durations, mean);

        // sort the durations for quantiles
        durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let quartile_fst = percentile(&durations, 0.25, n as f64);
        let median = percentile(&durations, 0.5, n as f64);
        let quartile_trd = percentile(&durations, 0.75, n as f64);

        // NOTE: durations is sorted and of len >= 1
        let min = *durations.first().unwrap();
        let max = *durations.last().unwrap();

        let percentiles: Vec<(f64, f64)> = PERCENTILE_LEVELS
            .into_iter()
            .map(|level| (level * 100.0, percentile(&durations, level, n as f64)))
            .collect();

        Some(Stats {
            scale,
            durations,
            total_duration: sum,
            total_bytes,
            mean,
            median,
            min,
            max,
            std,
            quartile_fst,
            quartile_trd,
            n_errors,
            errors,
            n_ok: n - n_errors,
            stats_by_thread,
            percentiles,
        })
    }
}

#[cfg(test)]
mod tests {
    use statrs::distribution::Normal;

    use super::*;

    #[test]
    fn test_percentile() {
        let mut samples = vec![82., 91., 12., 92., 63., 9., 28., 55., 96., 97.];
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let median = percentile(&samples, 0.5, 10.0);
        assert_eq!(median, 72.5);

        let quartile_fst = percentile(&samples, 0.25, 10.0);
        assert_eq!(quartile_fst, 28.0);

        let quartile_trd = percentile(&samples, 0.75, 10.0);
        assert_eq!(quartile_trd, 92.0);
    }

    #[test]
    fn test_standard_deviation() {
        let samples = vec![2., 4., 4., 4., 5., 5., 7., 9.];

        let mean = sum(&samples) / 8.0;
        assert_eq!(mean, 5.0);
        let std = standard_deviation(&samples, mean);
        assert!(std.is_some());
        assert_eq!(std.unwrap(), 2.0);
    }

    #[test]
    fn test_t_stats() {
        let np_base = NormalParams {
            mean: 520.0,
            std: 50.0,
            n_samples: 80,
        };
        let np_new = NormalParams {
            mean: 500.0,
            std: 45.0,
            n_samples: 50,
        };
        let t_stats = test_statistics(&np_base, &np_new);

        assert!(t_stats.is_some());
        assert_eq!(t_stats.unwrap(), 2.361125344403821);
    }

    #[test]
    fn test_unsigned_p_value() {
        let np_base = NormalParams {
            mean: 520.0,
            std: 50.0,
            n_samples: 80,
        };
        let np_new = NormalParams {
            mean: 500.0,
            std: 45.0,
            n_samples: 50,
        };
        let u_p_value = unsigned_p_value(&np_base, &np_new);

        assert!(u_p_value.is_some());
        let n = Normal::new(0.0, 1.0).unwrap();
        assert_eq!(u_p_value.unwrap(), 0.009109785650170843);
    }

    #[test]
    fn test_performance_outcome() {
        let np_base = NormalParams {
            mean: 520.0,
            std: 50.0,
            n_samples: 80,
        };
        let np_new = NormalParams {
            mean: 500.0,
            std: 45.0,
            n_samples: 50,
        };

        let perf_outcome = performance_outcome(&np_base, &np_new, 0.01);
        assert!(perf_outcome.is_some());
        assert_eq!(perf_outcome.unwrap(), PerformanceOutcome::NoChange);

        let perf_outcome = performance_outcome(&np_base, &np_new, 0.005);
        assert!(perf_outcome.is_some());
        assert_eq!(
            perf_outcome.unwrap(),
            PerformanceOutcome::Improved {
                p_value: 0.009109785650170843
            }
        );

        let perf_outcome = performance_outcome(&np_new, &np_base, 0.005);
        assert!(perf_outcome.is_some());
        assert_eq!(
            perf_outcome.unwrap(),
            PerformanceOutcome::Regressed {
                p_value: 0.009109785650170843
            }
        );
    }
}
