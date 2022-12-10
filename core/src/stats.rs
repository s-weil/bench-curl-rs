use crate::config::DurationScale;
use log::warn;
use reqwest::blocking::Response;
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    time::Duration,
};

impl DurationScale {
    fn elapsed(&self, duration: &Duration) -> f64 {
        match self {
            DurationScale::Nano => duration.as_nanos() as f64,
            DurationScale::Micro => duration.as_micros() as f64,
            DurationScale::Milli => duration.as_millis() as f64,
            DurationScale::Secs => duration.as_secs() as f64,
        }
    }
}

enum RequestResult {
    /// Cont ains the status code.
    Failed(usize), // TODO: maybe add also durations here?
    /// Contains the duration of the request.
    Ok(Duration),
}

pub struct StatsCollector {
    duration_unit: DurationScale,
    n_runs: usize,
    results: Vec<RequestResult>,
}

impl StatsCollector {
    pub fn init(n_runs: usize, duration_unit: DurationScale) -> Self {
        Self {
            n_runs: 0,
            duration_unit,
            results: Vec::with_capacity(n_runs),
        }
    }

    pub fn add(&mut self, response: Response, duration: Duration) {
        let result = match response.status().as_u16() as usize {
            200 => RequestResult::Ok(duration),
            sc => RequestResult::Failed(sc),
        };

        self.results.push(result);
        self.n_runs += 1;
    }

    pub fn collect(&self) -> Option<Stats> {
        Stats::calculate(self)
    }
}

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

#[derive(Debug)]
pub struct Stats {
    pub total: f64,
    pub mean: f64,
    pub median: f64,
    pub quartile_fst: f64,
    pub quartile_trd: f64,
    pub min: f64,
    pub max: f64,
    pub std: Option<f64>,
    pub distribution: Vec<f64>,
    pub n_ok: usize,
    pub n_errors: usize,
    // TODO: provide overview of errors - tbd if actually interestering or a corner case
    // TODO: outliers
}

impl Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        writeln!(f, "____________SUMMARY____________")?;
        writeln!(f, "number: ok: {} - failed: {}", self.n_ok, self.n_errors)?;
        writeln!(f, "Total Duration: {}", self.total)?;
        writeln!(f, "Mean: {}", self.mean)?;
        if let Some(std) = self.std {
            writeln!(f, "StdDev: {}", std)?;
        }
        writeln!(f, "Min: {}", self.min)?;
        writeln!(f, "Quartile 1st: {}", self.quartile_fst)?;
        writeln!(f, "Median: {}", self.median)?;
        writeln!(f, "Quartile 3rd: {}", self.quartile_trd)?;
        writeln!(f, "Max: {}", self.max)?;
        writeln!(f, "_______________________________")?;
        if self.distribution.len() <= 200 {
            writeln!(f, "Distribution (ordered):")?;
            writeln!(f, "{:?}", self.distribution)?;
        } else {
            writeln!(
                f,
                "Distribution cannot be displayed, length exceeding the limit"
            )?;
        }
        writeln!(f, "_______________________________")
    }
}

impl Stats {
    pub fn calculate(collected_stats: &StatsCollector) -> Option<Self> {
        if collected_stats.n_runs == 0 || collected_stats.results.is_empty() {
            return None;
        }

        let mut durations = Vec::with_capacity(collected_stats.results.len());
        let mut errors = HashMap::new();
        let mut n_errors = 0;

        let get_duration =
            |duration: &Duration| -> f64 { collected_stats.duration_unit.elapsed(duration) };

        for result in collected_stats.results.iter() {
            match result {
                RequestResult::Ok(duration) => durations.push(get_duration(duration)),
                RequestResult::Failed(status_code) => {
                    errors
                        .entry(status_code)
                        .and_modify(|count| *count += 1)
                        .or_insert(1);
                    n_errors += 1;
                }
            }
        }

        let n = durations.len();
        if n == 0 {
            warn!("Measurement yielded no valid results.");
            return None;
        }

        let sum = sum(&durations);
        let mean = sum / (n as f64);
        let std = standard_deviation(&durations, mean);

        // sort the durations for quantiles
        durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = percentile(&durations, 0.5, n as f64);
        let quartile_trd = percentile(&durations, 0.25, n as f64);
        let quartile_fst = percentile(&durations, 0.75, n as f64);

        // NOTE: durations is sorted and of len >= 1
        let min = *durations.first().unwrap();
        let max = *durations.last().unwrap();

        Some(Stats {
            total: sum,
            mean,
            median,
            min,
            max,
            std,
            quartile_fst,
            quartile_trd,
            distribution: durations,
            n_errors,
            n_ok: n - n_errors,
        })
    }
}

#[cfg(test)]
mod tests {
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
}
