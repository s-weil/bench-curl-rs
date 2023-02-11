use super::{
    confidence_interval, normal_qq, percentile, requests_per_sec, standard_deviation,
    stats::NormalParams, sum, BootstrapSampler,
};
use crate::{
    config::DurationScale,
    sampling::{RequestResult, SampleCollector, StatusCode},
    ThreadIdx,
};
use log::warn;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display};

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreadStats {
    #[serde(skip_deserializing)]
    #[serde(skip_serializing)] // serialize or not?
    errors: HashMap<StatusCode, i32>,
    #[serde(skip_deserializing)]
    #[serde(skip_serializing)] // serialize or not?
    pub durations: Vec<f64>,

    pub total_bytes: u64,
    pub n_ok: usize,
    pub n_errors: usize,

    pub total_duration: Option<f64>,
    // pub mean_rps: Option<f64>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct StatsSummary {
    // #[serde(skip_serializing_if = "Map::is_empty")]
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub errors: HashMap<StatusCode, i32>,
    #[serde(skip_deserializing)]
    #[serde(skip_serializing)] // serialize or not?
    pub durations: Vec<f64>,

    pub scale: DurationScale,
    pub total_duration: f64,
    pub total_bytes: u64,
    pub mean_rps: Option<f64>,

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
    #[serde(skip_deserializing)]
    pub display_percentiles: Vec<(f64, f64)>,

    pub qq_percentiles: Vec<(f64, f64)>,
    // TODO: provide overview of errors - tbd if actually interestering or a corner case
    // TODO: outliers
}

const N_PERCENTILES: usize = 20;

impl Display for StatsSummary {
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
        if let Some(rps) = self.mean_rps {
            writeln!(f, "Mean requests/s | {}", rps)?;
        }

        writeln!(f, "_______DURATIONS_______________________________")?;
        writeln!(f, "Total          | {}", self.total_duration)?;
        writeln!(f, "Mean           | {}", self.mean)?;
        // writeln!(f, "Requests per sec | {}", self.mean)?;

        if let Some(std) = self.std {
            writeln!(f, "StdDev         | {}", std)?;
        }
        writeln!(f, "Min            | {}", self.min)?;
        writeln!(f, "Quartile 1st   | {}", self.quartile_fst)?;
        writeln!(f, "Median         | {}", self.median)?;
        writeln!(f, "Quartile 3rd   | {}", self.quartile_trd)?;
        writeln!(f, "Max            | {}", self.max)?;

        if self.n_ok >= N_PERCENTILES {
            writeln!(f, "_______PERCENTILES_____________________________")?;
            for (level, percentile) in self.display_percentiles.iter() {
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

impl From<&StatsSummary> for NormalParams {
    fn from(stats: &StatsSummary) -> Self {
        NormalParams {
            mean: stats.mean,
            std: stats.std.unwrap(), // TODO: handle
            n_samples: stats.n_ok,
        }
    }
}

static PERCENTILE_LEVELS: [f64; 13] = [
    0.01, 0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.95, 0.99,
];

impl StatsSummary {
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

    pub fn normal_qq_curve(&self) -> Vec<(f64, f64)> {
        if let Some(std) = self.std {
            let np = NormalParams {
                mean: self.mean,
                std,
                n_samples: self.n_ok,
            };

            normal_qq(&self.qq_percentiles, &np)
        } else {
            Vec::with_capacity(0)
        }
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

        let mean_rps = requests_per_sec(mean, &scale);

        // sort the durations for quantiles
        durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let quartile_fst = percentile(&durations, 0.25, n as f64);
        let median = percentile(&durations, 0.5, n as f64);
        let quartile_trd = percentile(&durations, 0.75, n as f64);

        // NOTE: durations is sorted and of len >= 1
        let min = *durations.first().unwrap();
        let max = *durations.last().unwrap();

        let display_percentiles: Vec<(f64, f64)> = PERCENTILE_LEVELS
            .into_iter()
            .map(|level| (level * 100.0, percentile(&durations, level, n as f64)))
            .collect();

        let n_percentiles = durations.len() / 10;
        let qq_percentiles = (1..n_percentiles)
            .map(|level| {
                (
                    level as f64 * 100.0 / (n_percentiles as f64),
                    percentile(&durations, level as f64 / (n_percentiles as f64), n as f64),
                )
            })
            .collect();

        Some(StatsSummary {
            scale,
            durations,
            total_duration: sum,
            total_bytes,
            mean_rps,
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
            qq_percentiles,
            display_percentiles,
        })
    }

    pub fn bootstrap_summary(
        &self,
        n_draws: usize,
        n_samples: usize,
        alpha: f64,
    ) -> (Vec<f64>, Option<(f64, f64)>) {
        let bootstrap_means =
            BootstrapSampler::new(&self.durations).sample_means(n_draws, n_samples);
        let confidence_interval = confidence_interval(&bootstrap_means, alpha);
        (bootstrap_means, confidence_interval)
    }
}
