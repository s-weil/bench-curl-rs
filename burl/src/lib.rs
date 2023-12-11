mod config;
mod errors;

pub mod parser;
pub mod sampling;
pub mod stats;

pub use crate::parser::parse_toml;
use crate::stats::StatsProcessor;
pub(crate) use config::ConcurrenyLevel;
pub use config::{BenchClientConfig, StatsConfig};
pub use errors::{BurlError, BurlResult};

use chrono::{DateTime, Utc};
use log::{error, info};
use sampling::{RequestFactory, SampleCollector};
use stats::StatsSummary;
use std::sync::Arc;
use tokio::time::Instant;

pub type ThreadIdx = usize;

pub struct RunSummary {
    pub stats_processor: StatsProcessor,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

impl RunSummary {
    pub fn stats(&self) -> Option<StatsSummary> {
        self.stats_processor.stats_summary()
    }
}

pub struct BenchClient<'a> {
    request_factory: RequestFactory,
    config: &'a BenchClientConfig,
}

impl<'a> BenchClient<'a> {
    pub fn init(config: &'a BenchClientConfig) -> Result<Self, String> {
        let request_factory =
            RequestFactory::new(config.disable_certificate_validation.unwrap_or_default())
                .map_err(|err| format!("Could not initialize client: {}", err))?;

        Ok(Self {
            config,
            request_factory,
        })
    }

    // TODO: split into collection of samples and report creation
    pub async fn run(&self) -> Option<RunSummary> {
        let start_time = Utc::now();

        let n_runs = self.config.n_runs();
        let scale = self.config.duration_scale();

        let request_builder = match self.request_factory.assemble_request(self.config) {
            Ok(req) => req,
            Err(error) => {
                error!("Failed to compile the request. {}", error);
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

        let n_threads = match self.config.concurrency_level() {
            ConcurrenyLevel::Sequential => {
                info!(
                    "Starting measurement of {} samples from {}",
                    n_runs, self.config.url,
                );
                1
            }
            ConcurrenyLevel::Concurrent(n_threads) => {
                info!(
                    "Starting measurement of {} samples (on each of {} threads) from {}",
                    n_runs, n_threads, self.config.url
                );
                n_threads.max(1)
            }
        };

        // `global` timer over all threads
        let timer = Arc::new(Instant::now());

        // TODO: consider to use thread scope below
        let mut tasks = Vec::with_capacity(n_threads);
        // NOTE: cannot use rayon due to unsatisfied trait bounds
        for thread_idx in 0..n_threads.max(1) {
            let request_builder = request_builder.try_clone().unwrap();

            let mut sampler = SampleCollector::new(
                timer.clone(),
                thread_idx,
                n_runs,
                self.config.duration_scale().clone(),
            );

            let sampler = tokio::spawn(async move {
                sampler.collect_samples(request_builder).await;
                sampler
            });

            tasks.push(sampler);
        }

        let mut samples_by_thread = Vec::new();
        for task in tasks {
            samples_by_thread.push(task.await.unwrap());
        }

        let end_time = Utc::now();
        let stats_processor = StatsProcessor::new(scale.clone(), samples_by_thread);
        Some(RunSummary {
            stats_processor,
            start_time,
            end_time,
        })
    }
}
