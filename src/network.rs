//! Shared HTTP client for OpenAI-compatible chat completion APIs.

use std::time::Duration;

use reqwest::{Url, blocking::Client, redirect::Policy};
use serde_json::{Value, json};

const MAX_ERROR_BODY_CHARS: usize = 600;

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
