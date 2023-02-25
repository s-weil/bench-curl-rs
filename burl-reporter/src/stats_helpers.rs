use burl::{
    stats::{AnalyticTester, NormalParams, PermutationTester, StatsSummary, TestOutcome},
    StatsConfig,
};

pub struct StatisticalTester<'a> {
    current_stats: &'a StatsSummary,
    baseline_stats: &'a StatsSummary,
    stats_config: &'a StatsConfig,
}

impl<'a> StatisticalTester<'a> {
    pub fn try_new(
        current_stats: &'a StatsSummary,
        baseline_stats: &'a StatsSummary,
        stats_config: &'a StatsConfig,
    ) -> Option<Self> {
        if current_stats.scale != baseline_stats.scale {
            return None;
        }
        Some(Self {
            current_stats,
            baseline_stats,
            stats_config,
        })
    }

    fn performance_test(&self, alpha: f64) -> Option<TestOutcome> {
        let current_durations = &self.current_stats.durations;
        let baseline_durations = &self.baseline_stats.durations;

        let permutation_tester = PermutationTester::new(current_durations, baseline_durations);
        let n_samples = self.stats_config.n_bootstrap_samples.unwrap_or(1_000);
        let permutation_outcome = permutation_tester.test(n_samples, alpha);
        permutation_outcome
    }

    fn analytic_test(&self, alpha: f64) -> Option<TestOutcome> {
        let current_normal = NormalParams::from(self.current_stats);
        let baseline_normal = NormalParams::from(self.baseline_stats);
        let analytic_test = AnalyticTester::new(&baseline_normal, &current_normal);
        let performance_outcome = analytic_test.test(alpha);
        performance_outcome
    }
}
