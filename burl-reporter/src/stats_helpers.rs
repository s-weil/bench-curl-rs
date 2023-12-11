use burl::stats::{AnalyticTester, NormalParams, PermutationTester, StatsSummary, TestOutcome};

pub(crate) struct StatisticalTester<'a> {
    pub(crate) current_stats: &'a StatsSummary,
    pub(crate) baseline_stats: &'a StatsSummary,
}

impl<'a> StatisticalTester<'a> {
    pub(crate) fn try_new(
        current_stats: &'a StatsSummary,
        baseline_stats: &'a StatsSummary,
    ) -> Option<Self> {
        if current_stats.scale != baseline_stats.scale {
            return None;
        }
        Some(Self {
            current_stats,
            baseline_stats,
        })
    }

    pub(crate) fn performance_test(
        &self,
        n_bootstrap_samples: usize,
        alpha: f64,
    ) -> Option<TestOutcome> {
        let current_durations = &self.current_stats.durations;
        let baseline_durations = &self.baseline_stats.durations;

        let permutation_tester = PermutationTester::new(current_durations, baseline_durations);
        permutation_tester.test(n_bootstrap_samples, alpha)
    }

    pub(crate) fn analytic_test(&self, alpha: f64) -> Option<TestOutcome> {
        let current_normal = NormalParams::from(self.current_stats);
        let baseline_normal = NormalParams::from(self.baseline_stats);
        let analytic_test = AnalyticTester::new(&baseline_normal, &current_normal);
        analytic_test.test(alpha)
    }
}
