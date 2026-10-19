//! Shared HTTP clients for GPUI assets and OpenAI-compatible chat completion APIs.

use std::{sync::Arc, time::Duration};

use futures::{FutureExt as _, future::BoxFuture, io::AsyncReadExt as _};
use gpui_kit::http_client::{self, HttpClient};
use reqwest::{Url, blocking::Client, redirect::Policy};
use serde_json::{Value, json};

const MAX_ERROR_BODY_CHARS: usize = 600;

pub fn gpui_http_client() -> Arc<dyn HttpClient> {
    Arc::new(GpuiHttpClient {
        client: Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .redirect(Policy::limited(10))
            .user_agent(concat!("Jump1/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to initialize GPUI HTTP client"),
        user_agent: http_client::http::HeaderValue::from_static(concat!(
            "Jump1/",
            env!("CARGO_PKG_VERSION")
        )),
    })
}

struct GpuiHttpClient {
    client: Client,
    user_agent: http_client::http::HeaderValue,
}

impl HttpClient for GpuiHttpClient {
    fn user_agent(&self) -> Option<&http_client::http::HeaderValue> {
        Some(&self.user_agent)
    }

    fn proxy(&self) -> Option<&http_client::Url> {
        None
    }

    fn send(
        &self,
        request: http_client::Request<http_client::AsyncBody>,
    ) -> BoxFuture<'static, http_client::Result<http_client::Response<http_client::AsyncBody>>>
    {
        let client = self.client.clone();
        async move {
            let (parts, mut body) = request.into_parts();
            let mut body_bytes = Vec::new();
            body.read_to_end(&mut body_bytes).await?;

            let method = reqwest::Method::from_bytes(parts.method.as_str().as_bytes())?;
            let mut request = client
                .request(method, parts.uri.to_string())
                .body(body_bytes);
            for (name, value) in parts.headers.iter() {
                request = request.header(name.as_str(), value.as_bytes());
            }

            let response = request.send()?;
            let status = http_client::StatusCode::from_u16(response.status().as_u16())?;
            let mut response_builder = http_client::Response::builder().status(status);
            for (name, value) in response.headers() {
                response_builder = response_builder.header(name.as_str(), value.as_bytes());
            }
            let body = response.bytes()?.to_vec();
            Ok(response_builder.body(http_client::AsyncBody::from(body))?)
        }
        .boxed()
    }
}

pub struct ChatCompletionClient {
    client: Client,
}

impl ChatCompletionClient {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(90))
            .redirect(Policy::none())
            .user_agent(concat!("Jump1/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| format!("Could not initialize HTTP client: {error}"))?;
        Ok(Self { client })
    }

    pub fn complete(
        &self,
        endpoint: &str,
        model: &str,
        api_key: &str,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, String> {
        let endpoint = validate_endpoint(endpoint)?;
        let model = model.trim();
        let api_key = api_key.trim();
        if model.is_empty() {
            return Err("Enter a model ID".to_owned());
        }
        if api_key.is_empty() {
            return Err("Save an API key in Settings first".to_owned());
        }

        let response = self
            .client
            .post(endpoint)
            .bearer_auth(api_key)
            .json(&json!({
                "model": model,
                "messages": [
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": user_prompt}
                ],
                "stream": false
            }))
            .send()
            .map_err(|error| format!("Request failed: {error}"))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            let message = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|body| {
                    body.get("error")
                        .and_then(|error| error.get("message"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| body.chars().take(MAX_ERROR_BODY_CHARS).collect());
            return Err(if message.trim().is_empty() {
                format!("API returned status {status}")
            } else {
                format!("API returned status {status}: {}", message.trim())
            });
        }

        let body = response
            .json::<Value>()
            .map_err(|error| format!("Could not parse API response: {error}"))?;
        let content = body
            .pointer("/choices/0/message/content")
            .ok_or_else(|| "The API response did not contain translation text".to_owned())?;
        match content {
            Value::String(text) => Ok(text.trim().to_owned()),
            Value::Array(parts) => {
                let text = parts
                    .iter()
                    .filter_map(|part| part.get("text").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join("");
                if text.trim().is_empty() {
                    Err("The API response did not contain translation text".to_owned())
                } else {
                    Ok(text.trim().to_owned())
                }
            }
            _ => Err("The API response did not contain translation text".to_owned()),
        }
    }
}

fn validate_endpoint(endpoint: &str) -> Result<Url, String> {
    let url = Url::parse(endpoint.trim()).map_err(|error| format!("Invalid endpoint: {error}"))?;
    let local_http = url.scheme() == "http"
        && url
            .host_str()
            .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1"));
    if (url.scheme() != "https" && !local_http)
        || url.username() != ""
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "Endpoint must use HTTPS (HTTP is allowed for localhost) and cannot contain credentials or query parameters".to_owned(),
        );
    }
    Ok(url)
}
