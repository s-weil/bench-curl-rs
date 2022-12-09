use reqwest::*;
use serde::{Deserialize, Serialize};

/*
    TODO:
        * cli
        * plotly
        * tokio support (tbd)
        * parallel via rayon?
        * input randomizer
*/

#[derive(Serialize, Deserialize, Debug)]
pub struct BenchInput {
    headers: String,
    #[serde(rename = "jsonPayload")]
    json_payload: String,
}

pub struct BenchConfig {
    n_runs: usize,
    run_parallel: bool,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            n_runs: 100,
            run_parallel: false,
        }
    }
}

pub struct BenchClient {
    client: Client,
    config: BenchConfig,
}

impl BenchClient {
    pub fn init(config: Option<BenchConfig>) -> Result<Self> {
        let client = reqwest::ClientBuilder::new().build()?;
        Ok(Self {
            client,
            config: config.unwrap_or_default(),
        })
    }
}

// fn main() -> Result<(), Box<dyn std::error::Error>> {

//     let cliet = reqwest::ClientBuilder::build(self)

//     let resp = reqwest::blocking::get("https://httpbin.org/ip")?
//         .json::<HashMap<String, String>>()?;
//     println!("{:#?}", resp);
//     Ok(())
// }
