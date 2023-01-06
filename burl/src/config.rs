use crate::sampling::Method;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Default, Deserialize, Debug, Clone, Serialize)]
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

#[derive(Default, Debug, Deserialize)]
pub enum ConcurrenyLevel {
    #[default]
    Sequential,
    /// Concurrency level
    Concurrent(usize),
}

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

    pub report_folder: Option<String>,
    // TODO:
    // * output path for results etc
    // * randomized requests / vec of payloads
    // * logging param with level?
    // #[serde(alias = "jsonPayloads")]
    // json_payloads: Option<Vec<String>>,
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
}
