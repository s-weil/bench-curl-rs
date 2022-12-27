mod config;
mod plots;
mod request_factory;
mod stats;

pub use config::BenchConfig;
use config::DurationScale;
pub use plots::plot_stats;

use crate::config::ConcurrenyLevel;
use log::{error, info};
use request_factory::RequestFactory;
use reqwest::*;
use stats::{Stats, StatsCollector};
use std::time::Instant;

fn timed_request(
    timer: Instant,
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

fn collect_samples(
    request_builder: blocking::RequestBuilder,
    n_runs: usize,
    duration_scale: DurationScale,
) -> StatsCollector {
    let timer = Instant::now();
    let mut stats_collector = StatsCollector::init(n_runs, duration_scale);

    for _ in 0..n_runs {
        timed_request(timer, &request_builder, &mut stats_collector);
    }
    stats_collector
}

pub struct BenchClient {
    request_factory: RequestFactory,
    config: BenchConfig,
}

impl BenchClient {
    pub fn init(config: BenchConfig) -> Result<Self> {
        let request_factory = request_factory::RequestFactory::new(
            config.disable_certificate_validation.unwrap_or_default(),
        )?;

        Ok(Self {
            config,
            request_factory,
        })
    }

    pub fn start_run(&self) -> Option<Stats> {
        let du = self.config.duration_unit();

        let n_runs = self.config.n_runs();

        let request_builder = match self.request_factory.assemble_request(&self.config) {
            Some(req) => req,
            None => {
                error!("Failed to compile the request");
                return None;
            }
        };

        // Trigger un-timed requests, possibly to populate a cache or similiar
        info!("Warming up");
        for _ in 0..self.config.warmup_runs() {
            if let Err(error) = request_builder.try_clone().unwrap().send() {
                error!("Warm up failed: {:?}", error);
                return None;
            }
        }

        info!(
            "Starting measurement of {} samples from {} (concurrency level {})",
            n_runs,
            self.config.url,
            1 // TODO
        );

        match self.config.concurrency_level() {
            ConcurrenyLevel::Sequential => {
                collect_samples(request_builder, n_runs, du);
            }
            ConcurrenyLevel::Concurrent(n_threads) => {
                // TODO: should we divide n-runs?
                let mut stats_by_thread = Vec::with_capacity(n_threads);
                // NOTE: cannot use rayon due to unsatisfied trait bounds
                for _ in 0..n_threads.max(1) {
                    info!("started new thread");

                    let request_builder = request_builder.try_clone().unwrap();
                    let duration_scale = du.clone();

                    let sampler = std::thread::spawn(move || {
                        collect_samples(request_builder, n_runs, duration_scale)
                    });

                    stats_by_thread.push(sampler);
                }

                for sampler in stats_by_thread {
                    let stats = sampler.join().unwrap().collect();
                    // TODO: merge stats, careful with max time etc
                }

                // TODO: what to do with the timeseries graph?
            }
        }

        None
        // let guard = stats_collector.lock().unwrap();
        // guard.collect()
    }
}
