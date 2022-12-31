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
use stats::{SampleCollector, Stats};
use std::{sync::Arc, time::Instant};

// TODO: move to requestfactory?
async fn timed_request(
    timer: Arc<Instant>,
    request: &RequestBuilder,
    stats_collector: &mut SampleCollector,
) {
    let request = request.try_clone().unwrap();
    let measurement_start = timer.elapsed();
    let start = Instant::now();

    match request.send().await {
        Ok(response) => {
            // TODO: better way of measuring the time?
            let duration = start.elapsed();
            let measurement_end = timer.elapsed();
            let status_code = response.status().as_u16() as usize;
            let content_length = response.content_length();
            drop(response);
            stats_collector.add(
                measurement_start,
                measurement_end,
                duration,
                status_code,
                content_length,
            );
        }
        Err(error) => {
            error!("Error during sending request: {:?}", error);
        }
    }
}

async fn collect_samples(
    thread_idx: usize,
    duration_scale: DurationScale,
    request_builder: RequestBuilder,
    n_runs: usize,
    timer: Arc<Instant>,
) -> SampleCollector {
    let mut stats_collector = SampleCollector::init(thread_idx, n_runs, duration_scale);
    for _ in 0..n_runs {
        timed_request(timer.clone(), &request_builder, &mut stats_collector).await;
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

    pub async fn start_run(&self) -> Option<Stats> {
        let duration_scale = self.config.duration_unit();

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
            if let Err(error) = request_builder.try_clone().unwrap().send().await {
                error!("Warm up failed: {:?}", error);
                return None;
            }
        }

        // `global` timer over all threads
        let timer = Arc::new(Instant::now());

        let stats = match self.config.concurrency_level() {
            ConcurrenyLevel::Sequential => {
                info!(
                    "Starting measurement of {} samples from {}",
                    n_runs, self.config.url,
                );
                let sc = collect_samples(0, duration_scale.clone(), request_builder, n_runs, timer)
                    .await;
                Stats::collect(&mut vec![sc].into_iter(), duration_scale)
            }
            ConcurrenyLevel::Concurrent(n_threads) => {
                // TODO: should we divide n-runs?
                info!(
                    "Starting measurement of {} samples (on each of {} threads) from {}",
                    n_runs, n_threads, self.config.url
                );
                let mut tasks = Vec::with_capacity(n_threads);
                // NOTE: cannot use rayon due to unsatisfied trait bounds
                for thread_idx in 0..n_threads.max(1) {
                    let request_builder = request_builder.try_clone().unwrap();
                    let scale = duration_scale.clone();
                    let timer = timer.clone();
                    let sampler = tokio::spawn(async move {
                        collect_samples(thread_idx, scale, request_builder, n_runs, timer).await
                    });

                    tasks.push(sampler);
                }

                let mut samples_by_thread = Vec::with_capacity(n_threads);
                for task in tasks {
                    samples_by_thread.push(task.await.unwrap());
                }

                Stats::collect(&mut samples_by_thread.into_iter(), duration_scale)
            }
        };

        stats
    }
}
