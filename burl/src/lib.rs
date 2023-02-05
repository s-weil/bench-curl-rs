mod config;
mod errors;
mod reporting;
mod sampling;

pub use config::BenchConfig;
pub(crate) use config::ConcurrenyLevel;
pub use errors::{BurlError, BurlResult};

use chrono::Utc;
use log::{error, info};
use reporting::ReportSummary;
use sampling::{RequestFactory, SampleCollector};
use std::sync::Arc;
use tokio::time::Instant;

pub type ThreadIdx = usize;

pub struct BenchClient<'a> {
    request_factory: RequestFactory,
    config: &'a BenchConfig,
}

impl<'a> BenchClient<'a> {
    pub fn init(config: &'a BenchConfig) -> Result<Self, String> {
        let request_factory =
            RequestFactory::new(config.disable_certificate_validation.unwrap_or_default())
                .map_err(|err| format!("Could not initialize client: {}", err))?;

        Ok(Self {
            config,
            request_factory,
        })
    }

    pub async fn start_run(&self) -> Option<ReportSummary<'a>> {
        let report_start_time = Utc::now();

        let duration_scale = self.config.duration_scale();

        let n_runs = self.config.n_runs();

        let request_builder = match self.request_factory.assemble_request(self.config) {
            Some(req) => req,
            None => {
                error!("Failed to compile the request");
                return None;
            }
        };

        // Trigger non-timed requests, possibly to populate a cache or similiar
        info!("Warming up");
        for _ in 0..self.config.warmup_runs() {
            if let Err(error) = request_builder.try_clone().unwrap().send().await {
                error!("Warm up failed: {:?}", error);
                return None;
            }
        }

        // `global` timer over all threads
        let timer = Arc::new(Instant::now());

        let mut samples_by_thread = Vec::new();

        match self.config.concurrency_level() {
            ConcurrenyLevel::Sequential => {
                info!(
                    "Starting measurement of {} samples from {}",
                    n_runs, self.config.url,
                );
                let mut sampler =
                    SampleCollector::new(timer.clone(), 0, n_runs, duration_scale.clone());
                sampler.collect_samples(request_builder).await;
                samples_by_thread.push(sampler);
            }
            ConcurrenyLevel::Concurrent(n_threads) => {
                info!(
                    "Starting measurement of {} samples (on each of {} threads) from {}",
                    n_runs, n_threads, self.config.url
                );
                let mut tasks = Vec::with_capacity(n_threads);
                // NOTE: cannot use rayon due to unsatisfied trait bounds
                for thread_idx in 0..n_threads.max(1) {
                    let request_builder = request_builder.try_clone().unwrap();

                    let mut sampler = SampleCollector::new(
                        timer.clone(),
                        thread_idx,
                        n_runs,
                        duration_scale.clone(),
                    );

                    let sampler = tokio::spawn(async move {
                        sampler.collect_samples(request_builder).await;
                        sampler
                    });

                    tasks.push(sampler);
                }

                for task in tasks {
                    samples_by_thread.push(task.await.unwrap());
                }
            }
        };

        let report_end_time = Utc::now();
        Some(ReportSummary::new(
            report_start_time,
            report_end_time,
            self.config,
            samples_by_thread,
        ))
    }
}
