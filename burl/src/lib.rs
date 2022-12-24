mod config;
mod plots;
mod request_factory;
mod stats;

use crate::config::ConcurrenyLevel;
use log::{error, info};
use request_factory::RequestFactory;
use reqwest::*;
use stats::{Stats, StatsCollector};
use std::time::Instant;

pub use config::BenchConfig;
pub use plots::plot_stats;

pub struct BenchClient {
    request_factory: RequestFactory,
    config: BenchConfig,
}

impl BenchClient {
    pub fn init(config: BenchConfig) -> Result<Self> {
        let request_factory = request_factory::RequestFactory::new()?;

        Ok(Self {
            config,
            request_factory,
        })
    }

    fn timed_request(
        &self,
        timer: &Instant,
        request: &blocking::RequestBuilder,
        stats_collector: &mut StatsCollector,
    ) {
        let request = request.try_clone().unwrap();
        let measurement_start = timer.elapsed();
        let start = Instant::now();

        match request.send() {
            Ok(response) => {
                // TODO: better way of measuring the time?
                let duration = start.elapsed();
                let measurement_end = timer.elapsed();
                stats_collector.add(measurement_start, measurement_end, duration, response);
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

        let request = match self.request_factory.assemble_request(&self.config) {
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
                    if let Err(error) = dbg!(request.try_clone().unwrap().send()) {
                        error!("Warm up failed: {:?}", error);
                        return None;
                    }
                }
                info!(
                    "Starting measurement of {} samples from {}",
                    n_runs, self.config.url
                );
                let timer = Instant::now();
                for _ in 0..n_runs {
                    self.timed_request(&timer, &request, &mut stats_collector);
                }
            }
            ConcurrenyLevel::Concurrent(_level) => {
                todo!("use rayon");
            }
        }

        stats_collector.collect()
    }
}
