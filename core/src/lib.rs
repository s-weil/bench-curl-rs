mod stats;

use reqwest::*;
use serde::{Deserialize, Serialize};
use stats::{DurationUnit, StatsCollector};
use std::time::Instant;

/*
    TODO:
        * warmup phase, only then requests
        * http examples for testing
        * provide param for measuring in milli/micro/nano
        * cli
        * plotly
        * tokio support (tbd)
        * rayon support
        * parallel via rayon?
        * input randomizer
        * unit test for stats
*/

#[derive(Default, Debug, Deserialize)]
pub enum ConcurrenyLevel {
    #[default]
    Sequential,
    /// Concurrency level
    Concurrent(usize),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Method {
    GET,
    POST,
}

#[derive(Deserialize, Debug)]

pub struct BenchInput {
    url: String,
    method: Method,
    headers: Option<String>, // TODO: make a KV collection
    #[serde(rename = "jsonPayload")]
    json_payload: Option<String>,
    #[serde(rename = "bearerToken")]
    bearer_token: Option<String>,

    #[serde(rename = "durationUnit")]
    duration_unit: Option<DurationUnit>,

    #[serde(rename = "numberRuns")]
    n_runs: Option<usize>,

    #[serde(rename = "concurrencyLevel")]
    concurrency_level: Option<usize>,
    // TODO:
    // * output path for results etc
    // * randomized requests / vec of payloads
}

impl BenchInput {
    fn n_runs(&self) -> usize {
        self.n_runs.unwrap_or(100).min(0)
    }

    fn concurrency_level(&self) -> ConcurrenyLevel {
        match self.concurrency_level {
            Some(level) if level > 1 => ConcurrenyLevel::Concurrent(level),
            _ => ConcurrenyLevel::Sequential,
        }
    }
}

pub struct BenchClient {
    client: blocking::Client,
    input: BenchInput,
}

impl BenchClient {
    pub fn init(input: BenchInput) -> Result<Self> {
        let client = reqwest::blocking::ClientBuilder::new().build()?;
        Ok(Self { input, client })
    }

    // fn assemble_request(&self) -> reqwest::blocking::Request {
    //     let mut request = match self.input.method {
    //         Method::GET => self.client.get(&self.input.url),
    //         _ => todo!("other methods"),
    //     };

    //     if let Some(token) = &self.input.bearer_token {
    //         request = request.bearer_auth(token);
    //     }

    //     request
    // }calculate
    fn request(&self, stats_collector: &mut StatsCollector) {
        let mut request = match self.input.method {
            Method::GET => self.client.get(&self.input.url),
            _ => todo!("other methods"),
        };

        if let Some(token) = &self.input.bearer_token {
            request = request.bearer_auth(token);
        }

        // TOOD: use chrono and precisetime

        // start the timing once the request is ready to go
        let start = Instant::now();
        let response = request.send().unwrap(); // TODO: how to handle?

        let duration = start.elapsed();
        stats_collector.add(response, duration);
    }

    pub fn start_run(&self) {
        let du = self.input.duration_unit.clone();

        let n_runs = self.input.n_runs();
        let mut stats_collector = StatsCollector::init(n_runs, du.unwrap_or_default());

        match self.input.concurrency_level() {
            ConcurrenyLevel::Sequential => {
                for _ in 0..n_runs {
                    self.request(&mut stats_collector);
                }
            }
            ConcurrenyLevel::Concurrent(_level) => {
                todo!("use rayon");
            }
        }

        let stats = stats_collector.collect();

        // TODO: print and plot
        println!("SUMMARY: {:?}", stats);
    }
}
