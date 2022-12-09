use reqwest::blocking::Response;
use serde::Deserialize;
use std::{collections::HashMap, time::Duration};

#[derive(Default, Deserialize, Debug, Clone)]
pub enum DurationUnit {
    Nano,
    #[default]
    Micro,
    Milli,
}

impl DurationUnit {
    fn elapsed(&self, duration: &Duration) -> f64 {
        match self {
            DurationUnit::Nano => duration.as_nanos() as f64,
            DurationUnit::Micro => duration.as_micros() as f64,
            DurationUnit::Milli => duration.as_millis() as f64,
        }
    }
}

enum RequestResult {
    /// Contains the status code.
    Failed(usize), // TODO: maybe add also durations here?
    /// Contains the duration of the request.
    Ok(Duration),
}

pub struct StatsCollector {
    duration_unit: DurationUnit,
    n_runs: usize,
    results: Vec<RequestResult>,
}

impl StatsCollector {
    pub fn init(n_runs: usize, duration_unit: DurationUnit) -> Self {
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
fn percentile(durations: &[f64], level: f64, n: f64) -> f64 {
    // NOTE: have to add `-1` below due to (mathematical) idx start of 1 (rather than 0)
    let candidate_idx = n * level;
    let floored = candidate_idx.floor() as usize;

    // case candidate is an integer
    if candidate_idx == floored as f64 {
        let idx_bottom = (floored - 1).max(0);
        let idx_top = floored.min(n as usize);
        return 0.5 * (durations[idx_bottom] + durations[idx_top]);
    }
    let idx = ((candidate_idx + 1.0).floor().min(n) as usize - 1).max(0);
    durations[idx]
}

#[derive(Debug)]
pub struct Stats {
    pub total: f64,
    pub mean: f64,
    pub median: f64,
    pub quartile_fst: f64,
    pub quartile_trd: f64,
    // TODO: add buckets for histogramm and others instead
    pub distribution: Vec<f64>,
    pub n_errors: usize, // TODO: provide overview of errors - tbd if actually interestering or a corner case
}

impl Stats {
    pub fn calculate(collected_stats: &StatsCollector) -> Option<Self> {
        if collected_stats.n_runs == 0 || collected_stats.results.is_empty() {
            return None;
        }

        let n = collected_stats.n_runs;
        let mut durations = Vec::with_capacity(n);
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

        let sum = sum(&durations);
        let mean = sum / (n as f64);

        // sort the durations for quantiles
        durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = percentile(&durations, 0.5, n as f64);
        let quartile_trd = percentile(&durations, 0.25, n as f64);
        let quartile_fst = percentile(&durations, 0.75, n as f64);

        Some(Stats {
            total: sum,
            mean,
            median,
            quartile_fst,
            quartile_trd,
            distribution: durations,
            n_errors,
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
}
