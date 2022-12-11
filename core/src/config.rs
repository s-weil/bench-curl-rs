use std::collections::HashMap;

use serde::Deserialize;

use crate::request_factory::Method;

#[derive(Default, Deserialize, Debug, Clone)]
pub enum DurationScale {
    Nano,
    #[default]
    Micro,
    Milli,
    Secs,
}

#[derive(Default, Debug, Deserialize)]
pub enum ConcurrenyLevel {
    #[default]
    Sequential,
    /// Concurrency level
    Concurrent(usize),
}

// TODO: structure into sub types
#[derive(Deserialize, Debug, Default)]

pub struct BenchConfig {
    pub url: String,
    pub method: Method,
    // pub headers: HashMap<String, String>,
    pub headers: Option<Vec<(String, String)>>,
    // #[serde(rename = "jsonPayload")]
    pub json_payload: Option<String>,
    pub json_payload_ref: Option<String>,
    // #[serde(rename = "gqlQuery")]
    pub gql_query: Option<String>,

    // #[serde(rename = "bearerToken")]
    pub bearer_token: Option<String>,

    // #[serde(rename = "durationUnit")]
    duration_scale: Option<DurationScale>,

    // #[serde(rename = "numberRuns")]
    n_runs: Option<usize>,
    // #[serde(rename = "numberWarmupRuns")]
    n_warmup_runs: Option<usize>,

    // #[serde(rename = "concurrencyLevel")]
    concurrency_level: Option<usize>,

    pub results_folder: Option<String>,
    // TODO:
    // * output path for results etc
    // * randomized requests / vec of payloads
    // * logging param with level?
    // #[serde(rename = "jsonPayloads")]
    // json_payloads: Option<Vec<String>>,
}

impl BenchConfig {
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

    pub fn duration_unit(&self) -> DurationScale {
        self.duration_scale.clone().unwrap_or_default()
    }

    pub fn warmup_runs(&self) -> usize {
        self.n_warmup_runs.unwrap_or(0).max(0)
    }

    pub fn json_payload<'a>(&'a self) -> Option<&'a str> {
        // if self.json_payload.is_some() {
        //     return &self.json_payload.map(|json| json.as_str());
        // }
        return Some(
            r#"{
            "name": "John Doe",
            "price": 43.1
          }"#,
        );

        if let Some(_file_name) = &self.json_payload_ref {
            todo!("read in file with json payload");
        }

        None
    }
}
