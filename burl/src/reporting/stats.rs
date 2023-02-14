use crate::config::DurationScale;
use rand::distributions::Uniform;
use rand::Rng;
use rand::SeedableRng;
use statrs::distribution::ContinuousCDF;
use statrs::distribution::Normal;
use std::collections::HashSet;

pub type Probablity = f64; // values in [0,1]
pub type Percentage = f64; // values in [0,100]
pub type Percentile = f64;

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
pub enum PerformanceOutcome {
    Regressed { p_value: f64 },
    Improved { p_value: f64 },
    Inconclusive,
}

impl PerformanceOutcome {
    pub fn to_html(&self) -> String {
        match self {
            PerformanceOutcome::Improved { p_value } => {
                format!("<font color='green'>improved (p-value {})</font>", p_value)
            }
            PerformanceOutcome::Regressed { p_value } => {
                format!("<font color='red'>regressed (p-value {})</font>", p_value)
            }
            PerformanceOutcome::Inconclusive => "inconclusive (no significant change)".to_string(),
        }
    }
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
    alpha: Probablity,
) -> Option<PerformanceOutcome> {
    let p_value = unsigned_p_value(np_base, np)?;

    if p_value > alpha {
        return Some(PerformanceOutcome::Inconclusive);
    }

    // case of significant performance change
    if np_base.mean < np.mean {
        Some(PerformanceOutcome::Regressed { p_value })
    } else {
        Some(PerformanceOutcome::Improved { p_value })
    }
}

pub fn normal_qq(
    percentiles_by_level: &[(Percentage, Percentile)],
    np: &NormalParams,
) -> Vec<(Percentile, Percentile)> {
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

pub fn confidence_interval(distribution: &Vec<f64>, alpha: f64) -> Option<(f64, f64)> {
    if distribution.is_empty() {
        return None;
    }

    let mut sorted = distribution.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let alpha_2 = alpha / 2.0;
    let lower_bound = percentile(&sorted, alpha_2, distribution.len() as f64);
    let upper_bound = percentile(&sorted, 1.0 - alpha_2, distribution.len() as f64);
    Some((lower_bound, upper_bound))
}

pub struct BootstrapSampler<'a> {
    samples: &'a [f64],
}

impl<'a> BootstrapSampler<'a> {
    pub fn new(samples: &'a [f64]) -> Self {
        Self { samples }
    }

    fn simulate_sample_distr<F: rand::Rng>(&self, rng: &mut F, n_distr: usize) -> Vec<f64> {
        let distr = Uniform::new(0, self.samples.len());
        let sampler = rng.sample_iter(distr);
        sampler.take(n_distr).map(|idx| self.samples[idx]).collect()
    }

    fn bootstrap_samples<F: rand::Rng>(
        &self,
        rng: &mut F,
        n_distr: usize,
        n_samples: usize,
    ) -> Vec<Vec<f64>> {
        let mut samples = Vec::with_capacity(n_samples);

        for _ in 0..n_samples {
            let resampled = self.simulate_sample_distr(rng, n_distr);
            samples.push(resampled);
        }
        samples
    }

    pub fn sample_means(&self, n: usize, n_samples: usize) -> Vec<f64> {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);

        let bs_samples = self.bootstrap_samples(&mut rng, n, n_samples);

        let means = bs_samples
            .iter()
            .map(|resampled| sum(resampled) / resampled.len() as f64)
            .collect();

        means
    }
}

/// The null hypothesis of the [Permutation test](https://en.wikipedia.org/wiki/Permutation_test)
/// is that all samples come from the same distribution;
/// or in other words, there is no 'significant distinction' between both.
/// It is used as a proof by contradiction where a p-value below alpha will be reject the null hypothesis.
pub struct PermutationTester<'a> {
    current_samples: &'a [f64],
    baseline_samples: &'a [f64],
    current_len: usize,
    baseline_len: usize,
    total_len: usize,
}

impl<'a> PermutationTester<'a> {
    pub fn new(current_samples: &'a [f64], baseline_samples: &'a [f64]) -> Self {
        Self {
            current_len: current_samples.len(),
            baseline_len: baseline_samples.len(),
            total_len: current_samples.len() + baseline_samples.len(),
            current_samples,
            baseline_samples,
        }
    }

    fn idx_value(&self, idx: usize) -> Option<f64> {
        if idx < self.baseline_len {
            Some(self.baseline_samples[idx])
        } else if self.baseline_len <= idx && idx < self.total_len {
            Some(self.current_samples[idx - self.baseline_len])
        } else {
            None
        }
    }

    fn simulate_paired_distribution<F: rand::Rng>(&self, rng: &mut F) -> (Vec<f64>, Vec<f64>) {
        let distr = Uniform::new(0, self.total_len);
        let mut sampler = rng.sample_iter(distr);

        let mut baseline_indices = HashSet::new();
        let mut baseline_distr = Vec::with_capacity(self.baseline_len);

        while baseline_indices.len() < self.baseline_len {
            if let Some(idx) = sampler.next() {
                if !baseline_indices.contains(&idx) {
                    baseline_indices.insert(idx);

                    if let Some(v) = self.idx_value(idx) {
                        baseline_distr.push(v);
                    }
                }
            } else {
                // should not happen but avoid infty loops
                break;
            }
        }

        // the baseline_indices now have the size of baseline_len; create as the difference set
        let mut current_indices = Vec::with_capacity(self.current_len);
        for idx in 0..self.total_len {
            if !baseline_indices.contains(&idx) {
                if let Some(v) = self.idx_value(idx) {
                    current_indices.push(v);
                }
            }
        }

        (baseline_distr, current_indices)
    }

    fn sample_mean_differences<F: rand::Rng>(&self, rng: &mut F, n_samples: usize) -> Vec<f64> {
        let mut samples = Vec::with_capacity(n_samples);

        for _ in 0..n_samples {
            let (baseline, current) = self.simulate_paired_distribution(rng);
            let baseline_mean = sum(&baseline) / self.baseline_len as f64;
            let current_mean = sum(&current) / self.current_len as f64;
            let diff = baseline_mean - current_mean;
            samples.push(diff);
        }
        samples
    }

    pub fn test(&self, n_samples: usize, alpha: f64) -> Option<PerformanceOutcome> {
        if self.baseline_len == 0 || self.current_len == 0 {
            return None;
        }

        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);

        let mean_diff_samples = self.sample_mean_differences(&mut rng, n_samples);

        let baseline_mean = sum(self.baseline_samples) / self.baseline_len as f64;
        let current_mean = sum(self.current_samples) / self.current_len as f64;
        let test_diff = baseline_mean - current_mean;

        let n_extreme_diffs = mean_diff_samples.iter().fold(0, |acc, diff| {
            // baseline_mean >= current_mean || aseline_mean <= current_mean
            if (0.0 <= test_diff && test_diff <= *diff) || (*diff <= test_diff && test_diff < 0.0) {
                acc + 1
            } else {
                acc
            }
        });

        let p_value = n_extreme_diffs as f64 / n_samples as f64;

        if p_value > alpha {
            return Some(PerformanceOutcome::Inconclusive);
        }

        // case of significant performance change
        if baseline_mean < current_mean {
            Some(PerformanceOutcome::Regressed { p_value })
        } else {
            Some(PerformanceOutcome::Improved { p_value })
        }
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
    fn confidence_interval() {
        let mut distr = Vec::with_capacity(100);

        for idx in 0..=100 {
            distr.push(idx as f64);
        }

        let ci = super::confidence_interval(&distr, 0.1);
        assert_eq!(ci, Some((5.0, 95.0)));
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
        assert_eq!(perf_outcome.unwrap(), PerformanceOutcome::Inconclusive);

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

    #[test]
    fn bootstrap_sample_means() {
        let samples = [10.0, 11.0, 12.0, 10.5, 17.0, 33.0, 42.0, 2.0, 15.0, 14.0];
        let samples_mean = super::sum(&samples) / 10.0;
        assert_eq!(samples_mean, 16.65);

        let bs_sampler = BootstrapSampler::new(&samples);

        // 1000 bootstrap samples
        let n_bs_samples = 1_000;
        let sample_means = bs_sampler.sample_means(5, n_bs_samples);

        assert_eq!(sample_means.len(), n_bs_samples);

        let bs_mean = super::sum(&sample_means) / n_bs_samples as f64;
        assert_eq!(bs_mean, 16.765399999999996);

        // increase bootstrap samples, mean should converge
        let n_bs_samples = 100_000;
        let sample_means = bs_sampler.sample_means(5, n_bs_samples);

        assert_eq!(sample_means.len(), n_bs_samples);

        let bs_mean = super::sum(&sample_means) / n_bs_samples as f64;
        assert_eq!(bs_mean, 16.650610000000217);
    }

    #[test]
    fn permutation_test() {
        let baseline_samples = vec![10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0];

        let current_samples: Vec<f64> = vec![10.5, 10.5, 10.5, 9.5, 9.5, 9.5];
        let p_test = PermutationTester::new(&current_samples, &baseline_samples);
        assert_eq!(
            p_test.test(1000, 0.1),
            Some(PerformanceOutcome::Inconclusive)
        );

        let current_samples: Vec<f64> = vec![11.5, 11.5, 11.5, 11.0, 10.0, 9.5];
        let p_test = PermutationTester::new(&current_samples, &baseline_samples);
        assert_eq!(
            p_test.test(1000, 0.1),
            Some(PerformanceOutcome::Regressed { p_value: 0.008 })
        );

        let current_samples: Vec<f64> = vec![10.5, 10.0, 9.5, 9.0, 8.5, 8.5, 8.5];
        let p_test = PermutationTester::new(&current_samples, &baseline_samples);
        assert_eq!(
            p_test.test(1000, 0.1),
            Some(PerformanceOutcome::Improved { p_value: 0.013 })
        );
    }
}
