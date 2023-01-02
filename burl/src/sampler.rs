use crate::{config::DurationScale, ThreadIdx};
use log::{error, warn};
use reqwest::RequestBuilder;
use std::{sync::Arc, time::Duration};
use tokio::time::Instant; // TODO: check against std::time::Instant

// impl DurationScale {
//     pub fn elapsed(&self, duration: &Duration) -> f64 {
//         match self {
//             DurationScale::Nano => duration.as_nanos() as f64,
//             DurationScale::Micro => duration.as_micros() as f64,
//             DurationScale::Milli => duration.as_millis() as f64,
//             DurationScale::Secs => duration.as_secs() as f64,
//         }
//     }
// }

pub struct SampleResult {
    pub duration_from_start: Duration,
    pub duration_request_end: Duration,
    pub request_duration: Duration,
    pub content_length: Option<u64>,
}

pub enum RequestResult {
    /// Contains the status code.
    Failed(usize),
    /// Contains the duration of the request.
    Ok(SampleResult),
}

pub type StatusCode = usize;

pub struct SampleCollector {
    timer: Arc<Instant>,
    pub duration_scale: DurationScale, // TODO: Arc?
    pub thread_idx: ThreadIdx,
    pub n_runs: usize,
    pub results: Vec<RequestResult>,
}

impl SampleCollector {
    pub fn new(
        timer: Arc<Instant>,
        thread_idx: ThreadIdx,
        n_runs: usize,
        duration_scale: DurationScale,
    ) -> Self {
        Self {
            timer,
            duration_scale,
            thread_idx,
            n_runs,
            results: Vec::with_capacity(n_runs),
        }
    }

    fn add(
        &mut self,
        duration_since_start: Duration,
        duration_request_end: Duration,
        request_duration: Duration,
        status_code: StatusCode,
        content_length: Option<u64>,
    ) {
        let result = match status_code {
            200 => RequestResult::Ok(SampleResult {
                duration_from_start: duration_since_start,
                duration_request_end,
                request_duration,
                content_length,
            }),
            sc => {
                warn!("Received response with status code {}", sc);
                RequestResult::Failed(sc)
            }
        };

        self.results.push(result);
        // self.n_runs += 1;
    }

    async fn timed_request(&mut self, request: &RequestBuilder) {
        let request = request.try_clone().unwrap();
        let measurement_start = self.timer.elapsed();
        let start = Instant::now();

        match request.send().await {
            Ok(response) => {
                // TODO: better way of measuring the time?
                let duration = start.elapsed();
                let measurement_end = self.timer.elapsed();
                let status_code = response.status().as_u16() as usize;
                let content_length = response.content_length();
                drop(response);
                self.add(
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

    pub async fn collect_samples(&mut self, request_builder: RequestBuilder) {
        for _ in 0..self.n_runs {
            self.timed_request(&request_builder).await;
        }
    }
}
