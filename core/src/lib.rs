mod config;
mod plots;
mod stats;

use crate::config::ConcurrenyLevel;
use log::{error, info};
use reqwest::*;
use serde::Serialize;
use stats::{Stats, StatsCollector};
use std::time::Instant;

pub use config::BenchConfig;
pub use plots::plot;

#[derive(Serialize)]
struct GqlQuery<'a> {
    query: &'a String,
}

pub struct BenchClient {
    client: blocking::Client,
    config: BenchConfig,
}

impl BenchClient {
    pub fn init(config: BenchConfig) -> Result<Self> {
        let client = blocking::ClientBuilder::new().build()?;
        Ok(Self { config, client })
    }

    fn assemble_request(&self) -> Option<blocking::RequestBuilder> {
        let mut request = match self.config.method {
            config::Method::Get => self.client.get(&self.config.url),
            config::Method::Post => {
                let request = self.client.post(&self.config.url);

                if let Some(json) = &self.config.json_payload {
                    request.json(json)
                } else if let Some(query) = &self.config.gql_query {
                    let gql_query_payload = GqlQuery { query };
                    request.json(&gql_query_payload)
                } else {
                    error!("Expected either `json_payload` or `gql_query` in the config.");
                    return None;
                }
            }
            _ => unimplemented!("todo"),
        };

        if let Some(token) = &self.config.bearer_token {
            request = request.bearer_auth(token);
        }

        if let Some(_headers) = &self.config.headers {
            todo!("add headermap");
        }
        Some(request)
    }

    fn timed_request(
        &self,
        request: &reqwest::blocking::RequestBuilder,
        stats_collector: &mut StatsCollector,
    ) {
        let request = request.try_clone().unwrap();
        let start = Instant::now();

        match request.send() {
            Ok(response) => {
                // TODO: better way of measuring the time?
                let duration = start.elapsed();
                stats_collector.add(response, duration);
            }
            Err(error) => {
                error!("Error during sending request: {:?}", error);
            }
        }
    }

    pub fn start_run(&self) -> Option<Stats> {
        let du = self.config.duration_unit();

        let n_runs = self.config.n_runs();
        let mut stats_collector = StatsCollector::init(n_runs, du);

        let request = match self.assemble_request() {
            Some(req) => req,
            None => {
                error!("Failed to compile the request");
                return None;
            }
        };

        match self.config.concurrency_level() {
            ConcurrenyLevel::Sequential => {
                for _ in 0..self.config.warmup_runs() {
                    // Trigger a first few requests, possibly to populate a cache or similiar
                    info!("Warm-up run");
                    if let Err(error) = request.try_clone().unwrap().send() {
                        error!("Warm up failed: {:?}", error);
                        return None;
                    }
                }
                info!("Starting measurement of {} samples", n_runs);
                for _ in 0..n_runs {
                    self.timed_request(&request, &mut stats_collector);
                }
            }
            ConcurrenyLevel::Concurrent(_level) => {
                todo!("use rayon");
            }
        }

        stats_collector.collect()
    }
}
