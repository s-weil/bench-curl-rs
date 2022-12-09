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

    fn assemble_request(&self) -> reqwest::blocking::RequestBuilder {
        let mut request = match self.input.method {
            parameter::Method::GET => self.client.get(&self.input.url),
            _ => todo!("other methods"),
        };

        if let Some(token) = &self.input.bearer_token {
            request = request.bearer_auth(token);
        }

        request
    }

    fn timed_request(
        &self,
        request: &reqwest::blocking::RequestBuilder,
        stats_collector: &mut StatsCollector,
    ) {
        // let mut request = match self.input.method {
        //     parameter::Method::GET => self.client.get(&self.input.url),
        //     _ => todo!("other methods"),
        // };

        // if let Some(token) = &self.input.bearer_token {
        //     request = request.bearer_auth(token);
        // }

        // start the timing once the request is ready to go
        let start = Instant::now();
        let response = request.try_clone().unwrap().send().unwrap(); // TODO: how to handle?

        // TODO: better way of measuring the time?
        let duration = start.elapsed();
        stats_collector.add(response, duration);
    }

    pub fn start_run(&self) -> Option<Stats> {
        let du = self.input.duration_unit();

        let n_runs = self.input.n_runs();
        let mut stats_collector = StatsCollector::init(n_runs, du);

        let request = self.assemble_request();

        match self.input.concurrency_level() {
            ConcurrenyLevel::Sequential => {
                for _ in 0..self.input.warmup_runs() {
                    println!("Warum up run");
                    let _ = request.try_clone().unwrap().send().unwrap();
                    // TODO: how to handle?
                }
                println!("Starting measurment of {} samples", n_runs);
                for _ in 0..n_runs {
                    self.timed_request(&request, &mut stats_collector);
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
