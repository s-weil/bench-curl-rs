use crate::BenchConfig;
use log::{error, warn};
use reqwest::{Client, ClientBuilder, RequestBuilder, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct GqlQuery<'a> {
    query: &'a String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub enum Method {
    #[default]
    Get,
    Post,
    Put,
    Delete,
}

// pub struct RequestConfig {
//     pub url: String,
//     pub method: Method,
//     pub headers: Option<String>, // TODO: make a KV collection
//     #[serde(rename = "jsonPayload")]
//     pub json_payload: Option<String>,
//     #[serde(rename = "gqlQuery")]
//     pub gql_query: Option<String>,

//     // #[serde(rename = "bearerToken")]
//     pub bearer_token: Option<String>,
// }

pub struct RequestFactory {
    client: Client,
}

impl RequestFactory {
    pub fn new(disable_certificate_validation: bool) -> Result<Self> {
        let client = ClientBuilder::new()
            // .redirect(redirect::Policy::none())
            .danger_accept_invalid_certs(disable_certificate_validation)
            .build()?;
        Ok(Self { client })
    }

    pub fn assemble_request(&self, config: &BenchConfig) -> Option<RequestBuilder> {
        let mut request = match config.method {
            Method::Get => self.client.get(&config.url),
            Method::Post => {
                let request = self.client.post(&config.url);
                if let Some(json) = config.json_payload() {
                    request.body(json)
                } else if let Some(query) = &config.gql_query {
                    let gql_query_payload = GqlQuery { query };
                    request.json(&gql_query_payload)
                } else {
                    error!("Expected either `json_payload` or `gql_query` in the config.");
                    return None;
                }
            }
            _ => unimplemented!("todo"),
        };

        if let Some(token) = &config.bearer_token {
            request = request.bearer_auth(token);
        }

        if let Some(headers) = &config.headers {
            for (header_name, value) in headers.iter() {
                request = request.header(header_name, value);
            }
        } else if config.method == Method::Post {
            warn!("The method is 'POST' but no request headers are configured");
        }

        // NOTE: should be redundant (as default in HTTP/1.1) but to make sure
        request = request.header("Connection", "keep-alive");

        Some(request)
    }
}
