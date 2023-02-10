use crate::config::DurationScale;
use rand::Rng;
use statrs::distribution::ContinuousCDF;
use statrs::distribution::Normal;

const ZERO_THRESHOLD: f64 = 1e-16;

pub fn requests_per_sec(req_per_duration: f64, scale: &DurationScale) -> Option<f64> {
    if req_per_duration < ZERO_THRESHOLD {
        return None;
    }
    let rps = scale.factor(&DurationScale::Secs) / req_per_duration;
    Some(rps)
}

pub fn sum(durations: &[f64]) -> f64 {
    durations.iter().fold(0.0, |acc, dur| acc + dur)
}

/// Calculates the [empirical percentile](https://en.wikipedia.org/wiki/Percentile).
/// Due to earlier validation, `durations` is a non-empty, sorted vector at this point and `n` > 0
pub fn percentile(samples: &[f64], level: f64, n: f64) -> f64 {
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

/// The unbiased sample standard deviation.
pub fn standard_deviation(samples: &[f64], mean: f64) -> Option<f64> {
    let n_samples = samples.len();
    if n_samples <= 1 {
        return None;
    }
    let squared_errors = samples.iter().fold(0.0, |acc, d| {
        let error = (d - mean).powi(2);
        acc + error
    });

    let mean_squared_errors = squared_errors / (n_samples - 1) as f64;
    let std = mean_squared_errors.sqrt();
    Some(std)
}

pub struct NormalParams {
    pub mean: f64,
    pub std: f64,
    pub n_samples: usize,
}

#[derive(Debug, PartialEq)]
pub(crate) enum PerformanceOutcome {
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

pub(crate) fn performance_outcome(
    np_base: &NormalParams,
    np: &NormalParams,
    alpha: f64,
) -> Option<PerformanceOutcome> {
    let p_value = unsigned_p_value(np_base, np)?;

    if p_value > alpha {
        return Some(PerformanceOutcome::NoChange);
    }

    // case of significant performance change
    if np_base.mean < np.mean {
        Some(PerformanceOutcome::Regressed { p_value })
    } else {
        Some(PerformanceOutcome::Improved { p_value })
    }
}

pub fn normal_qq(percentiles_by_level: &[(f64, f64)], np: &NormalParams) -> Vec<(f64, f64)> {
    let normal = Normal::new(np.mean, np.std).unwrap();

    let qq = percentiles_by_level
        .iter()
        .map(|(level, percentile)| {
            let normal_percentile = normal.inverse_cdf(*level / 100.0);
            (normal_percentile, *percentile)
        })
        .collect();

    qq
}

pub struct BootstrapSampler<'a> {
    samples: &'a [f64],
}

use rand::distributions::Uniform;

impl<'a> BootstrapSampler<'a> {
    pub fn new(samples: &'a [f64]) -> Self {
        Self { samples }
    }

    fn seed_rng<F>(&self) -> F
    where
        F: rand::SeedableRng + rand::RngCore,
    {
        // let random_seed = rand::thread_rng().sample(rand_distr::Uniform::new(
        //     0u64,
        //     (self.samples.len() - 1) as u64,
        // ));
        F::seed_from_u64(0u64)
    }

    fn simulate_samples<F: rand::RngCore>(&self, rng: &mut F, n: usize) -> Vec<f64> {
        let distr = Uniform::new(0, self.samples.len());
        let sampler = rng.sample_iter(distr);
        sampler.take(n).map(|idx| self.samples[idx]).collect()
    }

    pub fn sample_means(&self, n: usize, n_samples: usize) -> Vec<f64> {
        let mut rng = rand::thread_rng();
        // let mut rng: rand::rngs::ThreadRng = self.rng();
        let mut samples = Vec::with_capacity(n_samples);

        for _ in 0..n_samples {
            let resampled = self.simulate_samples(&mut rng, n);
            let mean = sum(&resampled) / resampled.len() as f64;
            samples.push(mean);
        }
        samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_per_sec() {
        let mean = 0.0;
        let rps = super::requests_per_sec(mean, &DurationScale::Milli);
        assert!(rps.is_none());

        let mean = 100.0;
        let rps = super::requests_per_sec(mean, &DurationScale::Milli);
        assert_eq!(rps, Some(10.0));

        let mean = 100.0;
        let rps = super::requests_per_sec(mean, &DurationScale::Micro);
        assert_eq!(rps, Some(10_000.0));

        let mean = 100.0;
        let rps = super::requests_per_sec(mean, &DurationScale::Nano);
        assert_eq!(rps, Some(10_000_000.0));
    }

    #[test]
    fn percentile() {
        let mut samples = vec![82., 91., 12., 92., 63., 9., 28., 55., 96., 97.];
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let median = super::percentile(&samples, 0.5, 10.0);
        assert_eq!(median, 72.5);

        let quartile_fst = super::percentile(&samples, 0.25, 10.0);
        assert_eq!(quartile_fst, 28.0);

        let quartile_trd = super::percentile(&samples, 0.75, 10.0);
        assert_eq!(quartile_trd, 92.0);
    }

    #[test]
    fn standard_deviation() {
        let samples = vec![2., 4., 4., 4., 5., 5., 7., 9.];

        let mean = sum(&samples) / 8.0;
        assert_eq!(mean, 5.0);
        let std = super::standard_deviation(&samples, mean);
        assert!(std.is_some());
        // assert_eq!(std.unwrap(), 2.0);
        assert_eq!(std.unwrap(), 2.138089935299395);
    }

    #[test]
    fn t_stats() {
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
        let t_stats = super::test_statistics(&np_base, &np_new);

        assert!(t_stats.is_some());
        assert_eq!(t_stats.unwrap(), 2.361125344403821);
    }

    #[test]
    fn unsigned_p_value() {
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
        let u_p_value = super::unsigned_p_value(&np_base, &np_new);

        assert!(u_p_value.is_some());
        assert_eq!(u_p_value.unwrap(), 0.009109785650170843);
    }

    #[test]
    fn performance_outcome() {
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

        let perf_outcome = super::performance_outcome(&np_base, &np_new, 0.005);
        assert!(perf_outcome.is_some());
        assert_eq!(perf_outcome.unwrap(), PerformanceOutcome::NoChange);

        let perf_outcome = super::performance_outcome(&np_base, &np_new, 0.01);
        assert!(perf_outcome.is_some());
        assert_eq!(
            perf_outcome.unwrap(),
            PerformanceOutcome::Improved {
                p_value: 0.009109785650170843
            }
        );

        let perf_outcome = super::performance_outcome(&np_new, &np_base, 0.01);
        assert!(perf_outcome.is_some());
        assert_eq!(
            perf_outcome.unwrap(),
            PerformanceOutcome::Regressed {
                p_value: 0.009109785650170843
            }
        );
    }
}
