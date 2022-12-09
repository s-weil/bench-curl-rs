mod parameter;
mod stats;

use crate::parameter::ConcurrenyLevel;
use reqwest::*;
use stats::{Stats, StatsCollector};
use std::time::Instant;

pub use parameter::BenchInput;

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
            parameter::Method::GET => self.client.get(&self.input.url),
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

    pub fn start_run(&self) -> Option<Stats> {
        let du = self.input.duration_unit();

        let n_runs = self.input.n_runs();
        let mut stats_collector = StatsCollector::init(n_runs, du);

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

        stats
        // // TODO: print and plot
        // println!("SUMMARY: {:?}", stats);
    }
}
