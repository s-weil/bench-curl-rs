mod config;
mod plots;
mod stats;

use crate::config::ConcurrenyLevel;
use log::{error, info};
use reqwest::*;
use stats::{Stats, StatsCollector};
use std::time::Instant;

pub use config::BenchConfig;
pub use plots::plot;

pub struct BenchClient {
    client: blocking::Client,
    input: BenchConfig,
}

impl BenchClient {
    pub fn init(input: BenchConfig) -> Result<Self> {
        let client = reqwest::blocking::ClientBuilder::new().build()?;
        Ok(Self { input, client })
    }

    fn assemble_request(&self) -> reqwest::blocking::RequestBuilder {
        let mut request = match self.input.method {
            config::Method::GET => self.client.get(&self.input.url),
            _ => todo!("other methods"),
        };

        if let Some(token) = &self.input.bearer_token {
            request = request.bearer_auth(token);
        }

        request
    }

    fn timed_request(
        &self,
        // request: &reqwest::blocking::RequestBuilder,
        stats_collector: &mut StatsCollector,
    ) {
        // TODO: reuse the request
        let request = self.assemble_request();
        // let response = request.try_clone().unwrap();
        let start = Instant::now();

        match request.send() {
            Ok(response) => {
                // TODO: better way of measuring the time?
                let duration = start.elapsed();
                stats_collector.add(response, duration);
            }
            Err(error) => {
                error!("{:?}", error);
            }
        }
    }

    pub fn start_run(&self) -> Option<Stats> {
        let du = self.input.duration_unit();

        let n_runs = self.input.n_runs();
        let mut stats_collector = StatsCollector::init(n_runs, du);

        let request = self.assemble_request();

        match self.input.concurrency_level() {
            ConcurrenyLevel::Sequential => {
                for _ in 0..self.input.warmup_runs() {
                    // Trigger a first few requests, possibly to populate a cache or similiar
                    info!("Warm-up run");
                    if let Err(error) = request.try_clone().unwrap().send() {
                        error!("Warm up failed: {:?}", error);
                        return None;
                    }
                }
                info!("Starting measurement of {} samples", n_runs);
                for _ in 0..n_runs {
                    self.timed_request(&mut stats_collector);
                }
            }
            ConcurrenyLevel::Concurrent(_level) => {
                todo!("use rayon");
            }
        }

        let stats = stats_collector.collect();

        stats
    }
}
