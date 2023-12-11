use crate::{config::DurationScale, ThreadIdx};
use log::{error, warn};
use reqwest::RequestBuilder;
use serde::Serialize;
use std::{sync::Arc, time::Duration};
use tokio::time::Instant; // TODO: check against std::time::Instant

impl DurationScale {
    pub fn elapsed(&self, duration: &Duration) -> f64 {
        match self {
            DurationScale::Nano => duration.as_nanos() as f64,
            DurationScale::Micro => duration.as_micros() as f64,
            DurationScale::Milli => duration.as_millis() as f64,
            DurationScale::Secs => duration.as_secs() as f64,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct SampleResult {
    #[serde(skip_serializing)]
    pub duration_since_start: Duration,
    #[serde(skip_serializing)]
    pub duration_request_end: Duration,
    #[serde(skip_serializing)]
    pub request_duration: Duration,

    pub measurement_start: f64,
    pub measurement_end: f64,
    pub duration: f64,

    pub content_length: Option<u64>,
}

impl SampleResult {
    pub fn as_timeseries_point(&self) -> (f64, f64) {
        (self.measurement_start, self.duration)
    }
}

pub enum RequestResult {
    /// Contains the status code.
    Failed(usize),
    /// Contains the duration of the request.
    Ok(SampleResult),
}

impl RequestResult {
    pub fn as_result(&self) -> Option<&SampleResult> {
        match self {
            RequestResult::Ok(sr) => Some(sr),
            RequestResult::Failed(_) => None,
        }
    }
}

pub type StatusCode = usize;
const SUCCESS: usize = 200;

/// Creates and collects samples:
/// Iteratively sends the same request, measures timings and responses, and adds results.
pub struct SampleCollector {
    timer: Arc<Instant>, // TODO: as param? same as for requestBuilder?
    pub thread_idx: ThreadIdx,
    pub duration_scale: DurationScale,
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
            SUCCESS => RequestResult::Ok(SampleResult {
                measurement_start: self.duration_scale.elapsed(&duration_since_start),
                measurement_end: self.duration_scale.elapsed(&duration_request_end),
                duration: self.duration_scale.elapsed(&request_duration),
                duration_since_start,
                duration_request_end,
                request_duration,
                content_length,
            }),
            status_code => {
                warn!("Received response with status code {}", status_code);
                RequestResult::Failed(status_code)
            }
        };

        self.results.push(result);
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
                error!("Error while sending request: {:?}", error);
            }
        }
    }

    pub async fn collect_samples(&mut self, request_builder: RequestBuilder) {
        for _ in 0..self.n_runs {
            self.timed_request(&request_builder).await;
        }
    }
}
