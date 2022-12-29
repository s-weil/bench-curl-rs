use crate::config::DurationScale;
use log::warn;
use reqwest::blocking::Response;
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    time::Duration,
};

impl DurationScale {
    pub fn elapsed(&self, duration: &Duration) -> f64 {
        match self {
            DurationScale::Nano => duration.as_nanos() as f64,
            DurationScale::Micro => duration.as_micros() as f64,
            DurationScale::Milli => duration.as_millis() as f64,
            DurationScale::Secs => duration.as_secs() as f64,
        }
    }
}

struct SampleResult {
    duration_since_start: Duration,
    duration_request_end: Duration,
    request_duration: Duration,
    content_length: Option<u64>,
}

#[derive(Debug)]
pub enum TimeSeriesPoint {
    Start((f64, f64)),
    End(f64),
}

impl TimeSeriesPoint {
    pub fn as_graph_point(&self) -> (f64, f64) {
        match self {
            TimeSeriesPoint::Start((time, duration)) => (*time, *duration),
            TimeSeriesPoint::End(time) => (*time, 0.0),
        }
    }
}

enum RequestResult {
    /// Contains the status code.
    Failed(usize),
    /// Contains the duration of the request.
    Ok(SampleResult),
}

type ThreadIdx = usize;
type StatusCode = usize;

pub struct SampleCollector {
    duration_scale: DurationScale,
    thread_idx: ThreadIdx,
    n_runs: usize,
    results: Vec<RequestResult>,
}

impl SampleCollector {
    pub fn init(thread_idx: ThreadIdx, n_runs: usize, duration_scale: DurationScale) -> Self {
        Self {
            duration_scale,
            thread_idx,
            n_runs: 0,
            results: Vec::with_capacity(n_runs),
        }
    }

    pub fn add(
        &mut self,
        measurement_start: Duration,
        measurement_end: Duration,
        duration: Duration,
        response: Response,
    ) {
        let result = match response.status().as_u16() as usize {
            200 => RequestResult::Ok(SampleResult {
                duration_since_start: measurement_start,
                duration_request_end: measurement_end,
                request_duration: duration,
                content_length: response.content_length(),
            }),
            sc => {
                warn!("Received response with status code {}", sc);
                RequestResult::Failed(sc)
            }
        };

        self.results.push(result);
        self.n_runs += 1;
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
pub struct ThreadStats {
    // pub total: f64,
    pub total_bytes: u64,
    pub durations: Vec<f64>,
    // collection of start/end time of the sample from start of the measurement with the duration
    pub time_series: Vec<TimeSeriesPoint>,
    errors: HashMap<StatusCode, i32>,
    pub n_errors: usize,
}

impl From<SampleCollector> for ThreadStats {
    fn from(samples: SampleCollector) -> Self {
        let mut durations = Vec::with_capacity(samples.n_runs);
        let mut errors = HashMap::new();
        let mut time_series: Vec<TimeSeriesPoint> = Vec::with_capacity(2 * samples.n_runs);
        let mut total_bytes = 0;
        let mut n_errors = 0;

        let get_duration =
            |duration: &Duration| -> f64 { samples.duration_scale.elapsed(duration) };

        for result in samples.results.iter() {
            match result {
                RequestResult::Ok(duration_point) => {
                    let request_duration = get_duration(&duration_point.request_duration);
                    durations.push(request_duration);
                    let duration_since_start = get_duration(&duration_point.duration_since_start);
                    time_series.push(TimeSeriesPoint::Start((
                        duration_since_start,
                        request_duration,
                    )));
                    // let duration_request_end = get_duration(&duration_point.duration_request_end);
                    // time_series.push(TimeSeriesPoint::End(duration_request_end));
                    if let Some(bytes) = duration_point.content_length {
                        total_bytes += bytes;
                    }
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

        Self {
            total_bytes,
            durations,
            time_series,
            errors,
            n_errors,
        }
    }
}

#[derive(Debug)]
pub struct Stats {
    scale: DurationScale,
    pub total: f64,
    pub total_bytes: u64,
    pub mean: f64,
    pub median: f64,
    pub quartile_fst: f64,
    pub quartile_trd: f64,
    pub min: f64,
    pub max: f64,
    pub std: Option<f64>,
    pub durations: Vec<f64>,
    pub n_ok: usize,
    pub n_errors: usize,

    pub stats_by_thread: HashMap<ThreadIdx, ThreadStats>,
    // TODO: provide overview of errors - tbd if actually interestering or a corner case
    // TODO: outliers
    /// Percentiles 1% 5% 10% 20% 30% 40% 50% 60% 70% 80% 90% 95% 99%
    percentiles: Vec<(f64, f64)>,
}

impl Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        writeln!(
            f,
            "______________SUMMARY_[in {}s]______________",
            &self.scale
        )?;
        writeln!(f, "Total Duration   | {}", self.total)?;
        writeln!(f, "Total bytes      | {}", self.total_bytes)?;
        writeln!(f, "Number ok        | {}", self.n_ok)?;
        writeln!(f, "Number failed    | {}", self.n_errors)?;
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
            writeln!(f, "___________________________________________")?;
            writeln!(f, "Distribution of percentiles:")?;
            for (level, percentile) in self.percentiles.iter() {
                writeln!(f, "{}%    {}", level, percentile)?;
            }
        }
        writeln!(f, "___________________________________________")
    }
}

impl Stats {
    /// Collect the sample results from the threads' samples.
    pub fn collect<S>(samples_by_thread: &mut S, duration_scale: DurationScale) -> Option<Self>
    where
        S: Iterator<Item = SampleCollector>,
    {
        let mut durations = Vec::new();
        let mut stats_by_thread = HashMap::new();
        let mut total_bytes = 0;
        let mut n_errors = 0;

        while let Some(samples) = samples_by_thread.next() {
            let idx = samples.thread_idx;
            let thread_stats: ThreadStats = samples.into();

            n_errors += thread_stats.n_errors;
            total_bytes += thread_stats.total_bytes;

            durations.extend(thread_stats.durations.clone());
            stats_by_thread.insert(idx, thread_stats);
        }

        Self::calculate(
            duration_scale,
            n_errors,
            total_bytes,
            durations,
            stats_by_thread,
        )
    }

    pub fn calculate(
        scale: DurationScale,
        n_errors: usize,
        total_bytes: u64,
        mut durations: Vec<f64>,
        stats_by_thread: HashMap<ThreadIdx, ThreadStats>,
    ) -> Option<Self> {
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
        let quartile_fst = percentile(&durations, 0.25, n as f64);
        let median = percentile(&durations, 0.5, n as f64);
        let quartile_trd = percentile(&durations, 0.75, n as f64);

        // NOTE: durations is sorted and of len >= 1
        let min = *durations.first().unwrap();
        let max = *durations.last().unwrap();

        let percentiles: Vec<(f64, f64)> = [
            0.01, 0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.95, 0.99,
        ]
        .into_iter()
        .map(|level| (level * 100.0, percentile(&durations, level, n as f64)))
        .collect();

        Some(Stats {
            scale,
            durations,
            total: sum,
            total_bytes,
            mean,
            median,
            min,
            max,
            std,
            quartile_fst,
            quartile_trd,
            n_errors,
            n_ok: n - n_errors,
            stats_by_thread,
            percentiles,
        })
    }

    // pub fn calculate(collected_stats: &SampleCollector) -> Option<Self> {
    //     if collected_stats.n_runs == 0 || collected_stats.results.is_empty() {
    //         return None;
    //     }

    //     let mut durations = Vec::with_capacity(collected_stats.results.len());
    //     let mut total_bytes = 0;
    //     let mut errors = HashMap::new();
    //     let mut n_errors = 0;
    //     let mut time_series = Vec::with_capacity(collected_stats.results.len());

    //     let get_duration =
    //         |duration: &Duration| -> f64 { collected_stats.duration_scale.elapsed(duration) };

    //     for result in collected_stats.results.iter() {
    //         match result {
    //             RequestResult::Ok(duration_point) => {
    //                 let request_duration = get_duration(&duration_point.request_duration);
    //                 durations.push(request_duration);
    //                 let duration_since_start = get_duration(&duration_point.duration_since_start);
    //                 time_series.push((duration_since_start, request_duration));
    //                 let duration_request_end = get_duration(&duration_point.duration_request_end);
    //                 time_series.push((duration_request_end, 0.0));
    //                 if let Some(bytes) = duration_point.content_length {
    //                     total_bytes += bytes;
    //                 }
    //             }
    //             RequestResult::Failed(status_code) => {
    //                 errors
    //                     .entry(status_code)
    //                     .and_modify(|count| *count += 1)
    //                     .or_insert(1);
    //                 n_errors += 1;
    //             }
    //         }
    //     }

    //     let n = durations.len();
    //     if n == 0 {
    //         warn!("Measurement yielded no valid results.");
    //         return None;
    //     }

    //     let sum = sum(&durations);
    //     let mean = sum / (n as f64);
    //     let std = standard_deviation(&durations, mean);

    //     // sort the durations for quantiles
    //     durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
    //     let quartile_fst = percentile(&durations, 0.25, n as f64);
    //     let median = percentile(&durations, 0.5, n as f64);
    //     let quartile_trd = percentile(&durations, 0.75, n as f64);

    //     // NOTE: durations is sorted and of len >= 1
    //     let min = *durations.first().unwrap();
    //     let max = *durations.last().unwrap();

    //     let percentiles: Vec<(f64, f64)> = [
    //         0.01, 0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.95, 0.99,
    //     ]
    //     .into_iter()
    //     .map(|level| (level * 100.0, percentile(&durations, level, n as f64)))
    //     .collect();

    //     Some(Stats {
    //         scale: collected_stats.duration_scale.clone(),
    //         total: sum,
    //         total_bytes,
    //         mean,
    //         median,
    //         min,
    //         max,
    //         std,
    //         quartile_fst,
    //         quartile_trd,
    //         distribution: durations,
    //         n_errors,
    //         n_ok: n - n_errors,
    //         time_series,
    //         percentiles,
    //     })
    // }
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
