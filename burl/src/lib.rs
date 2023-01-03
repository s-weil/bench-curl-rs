mod config;
mod plots;
mod report;
mod request_factory;
mod sampler;
mod stats;

pub use config::BenchConfig;
pub(crate) use config::ConcurrenyLevel;
pub use report::ReportSummary;
pub(crate) use sampler::SampleCollector;

use log::{error, info};
use request_factory::RequestFactory;
use reqwest::*;
use std::sync::Arc;
use tokio::time::Instant;

pub type ThreadIdx = usize;

pub struct BenchClient<'a> {
    request_factory: RequestFactory,
    config: &'a BenchConfig,
}

impl<'a> BenchClient<'a> {
    pub fn init(config: &'a BenchConfig) -> Result<Self> {
        let request_factory = request_factory::RequestFactory::new(
            config.disable_certificate_validation.unwrap_or_default(),
        )?;

        Ok(Self {
            config,
            request_factory,
        })
    }

    pub async fn start_run(&self) -> Option<ReportSummary<'a>> {
        let duration_scale = self.config.duration_scale();

        let n_runs = self.config.n_runs();

        let request_builder = match self.request_factory.assemble_request(self.config) {
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
                // TODO: should we divide n-runs?
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

        Some(ReportSummary::new(self.config, samples_by_thread))
    }
}
