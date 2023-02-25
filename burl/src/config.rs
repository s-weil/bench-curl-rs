use crate::sampling::Method;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DurationScale {
    Nano,
    #[default]
    Micro,
    Milli,
    Secs,
}

impl fmt::Display for DurationScale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DurationScale::Nano => write!(f, "n"),
            DurationScale::Micro => write!(f, "µ"),
            DurationScale::Milli => write!(f, "m"),
            DurationScale::Secs => write!(f, ""),
        }
    }
}

impl DurationScale {
    pub fn scale(&self) -> usize {
        match self {
            DurationScale::Nano => 1_000_000_000,
            DurationScale::Micro => 1_000_000,
            DurationScale::Milli => 1_000,
            DurationScale::Secs => 1,
        }
    }

    /// The factor for `self / other`.
    pub fn factor(&self, other: &DurationScale) -> f64 {
        let f_self = self.scale();
        let f_other = other.scale();
        f_self as f64 / f_other as f64
    }
}

#[derive(Default, Debug, Deserialize)]
pub enum ConcurrenyLevel {
    #[default]
    Sequential,
    /// Concurrency level
    Concurrent(usize),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatsConfig {
    /// the confidence / significance level
    pub alpha: Option<f64>,
    pub n_bootstrap_samples: Option<usize>,
    pub n_bootstrap_draw_size: Option<usize>,
}

const ALPHA: f64 = 0.05;

// impl Default for StatsConfig {
//     fn default() -> Self {
//         Self {
//             alpha: Some(ALPHA),
//             n_bootstrap_samples: 1000,
//             n_bootstrap_draw_size: 100,
//         }
//     }
// }

// TODO: structure into sub types
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct BenchConfig {
    pub url: String,
    pub method: Method,
    #[serde(alias = "disableCertificateValidation")]
    pub disable_certificate_validation: Option<bool>,
    // pub headers: HashMap<String, String>,
    pub headers: Option<Vec<(String, String)>>,
    #[serde(alias = "jsonPayload")]
    pub json_payload: Option<String>,
    #[serde(alias = "jsonPayloadReference")]
    #[serde(alias = "jsonPayloadRef")]
    pub json_payload_ref: Option<String>,
    #[serde(alias = "gqlQuery")]
    pub gql_query: Option<String>,

    #[serde(alias = "bearerToken")]
    pub bearer_token: Option<String>,

    #[serde(alias = "durationScale")]
    duration_scale: Option<DurationScale>,

    #[serde(alias = "numberRuns")]
    #[serde(alias = "nRuns")]
    n_runs: Option<usize>,
    #[serde(alias = "numberWarmupRuns")]
    #[serde(alias = "nWarmupRuns")]
    n_warmup_runs: Option<usize>,

    #[serde(alias = "concurrencyLevel")]
    concurrency_level: Option<usize>,

    #[serde(alias = "reportDirectory")]
    pub report_directory: Option<String>,
    #[serde(alias = "baselinePath")]
    pub baseline_path: Option<String>,
    // TODO:
    // * randomized requests / vec of payloads
    // * logging param with level?
    // #[serde(alias = "jsonPayloads")]
    // json_payloads: Option<Vec<String>>,
    #[serde(alias = "statsConfig")]
    #[serde(alias = "statisticsConfig")]
    pub stats_config: Option<StatsConfig>,
}

const DEFAULT_NRUNS: usize = 300;

impl BenchConfig {
    pub fn new(url: String) -> Self {
        Self {
            url,
            ..Self::default()
        }
    }

    pub fn n_runs(&self) -> usize {
        self.n_runs.unwrap_or(DEFAULT_NRUNS).max(0)
    }

    pub fn concurrency_level(&self) -> ConcurrenyLevel {
        match self.concurrency_level {
            Some(level) if level > 1 => ConcurrenyLevel::Concurrent(level),
            _ => ConcurrenyLevel::Sequential,
        }
    }

    pub fn duration_scale(&self) -> DurationScale {
        self.duration_scale.clone().unwrap_or_default()
    }

    pub fn warmup_runs(&self) -> usize {
        self.n_warmup_runs.unwrap_or(0).max(0)
    }

    pub fn json_payload(&self) -> Option<String> {
        if self.json_payload.is_some() {
            return self.json_payload.clone();
        }

        if let Some(_file_name) = &self.json_payload_ref {
            todo!("read in file with json payload");
        }

        None
    }

    pub fn alpha(&self) -> f64 {
        self.stats_config
            .as_ref()
            .and_then(|scfg| scfg.alpha)
            .unwrap_or(ALPHA)
    }

    pub fn n_bootstrap_draw_size(&self) -> usize {
        self.stats_config
            .as_ref()
            .and_then(|scfg| scfg.n_bootstrap_draw_size)
            .unwrap_or(100)
    }

    pub fn n_bootstrap_samples(&self) -> usize {
        self.stats_config
            .as_ref()
            .and_then(|scfg| scfg.n_bootstrap_samples)
            .unwrap_or(1_000)
    }

    // pub fn stats_config(&self) -> StatsConfig {
    //     StatsConfig {
    //         alpha: self.alpha(),
    //         n_bootstrap_samples: self.n_bootstrap_samples(),
    //         n_bootstrap_draw_size: self.n_bootstrap_draw_size(),
    //     }
    // }
}
