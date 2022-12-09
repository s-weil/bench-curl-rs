use serde::Deserialize;

#[derive(Default, Deserialize, Debug, Clone)]
pub enum DurationUnit {
    Nano,
    #[default]
    Micro,
    Milli,
}

#[derive(Default, Debug, Deserialize)]
pub enum ConcurrenyLevel {
    #[default]
    Sequential,
    /// Concurrency level
    Concurrent(usize),
}

#[derive(Deserialize, Debug, Default)]
pub enum Method {
    #[default]
    GET,
    POST,
}

// TODO: structure into sub types
#[derive(Deserialize, Debug, Default)]

pub struct BenchInput {
    pub url: String,
    pub method: Method,
    headers: Option<String>, // TODO: make a KV collection
    #[serde(rename = "jsonPayload")]
    json_payload: Option<String>,
    #[serde(rename = "gqlQuery")]
    gql_query: Option<String>,

    // #[serde(rename = "bearerToken")]
    pub bearer_token: Option<String>,

    // #[serde(rename = "durationUnit")]
    duration_unit: Option<DurationUnit>,

    // #[serde(rename = "numberRuns")]
    n_runs: Option<usize>,
    // #[serde(rename = "numberWarmupRuns")]
    n_warmup_runs: Option<usize>,

    // #[serde(rename = "concurrencyLevel")]
    concurrency_level: Option<usize>,

    pub plot_folder: Option<String>,
    // TODO:
    // * output path for results etc
    // * randomized requests / vec of payloads
    // * logging param with level?
    // #[serde(rename = "jsonPayloads")]
    // json_payloads: Option<Vec<String>>,
}

impl BenchInput {
    pub fn new(url: String) -> Self {
        Self {
            url,
            ..Self::default()
        }
    }

    pub fn n_runs(&self) -> usize {
        self.n_runs.unwrap_or(100).max(0)
    }

    pub fn concurrency_level(&self) -> ConcurrenyLevel {
        match self.concurrency_level {
            Some(level) if level > 1 => ConcurrenyLevel::Concurrent(level),
            _ => ConcurrenyLevel::Sequential,
        }
    }

    pub fn duration_unit(&self) -> DurationUnit {
        self.duration_unit.clone().unwrap_or_default()
    }

    pub fn warmup_runs(&self) -> usize {
        self.n_warmup_runs.unwrap_or(0).max(0)
    }
}
