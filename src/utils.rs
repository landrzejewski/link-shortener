use axum::http::{HeaderMap, StatusCode};
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use base64::Engine;
use rand::{thread_rng, Rng};
use sqlx::__rt::timeout;
use std::env;
use std::error::Error;
use std::future::Future;
use std::time::Duration;
use rand::distributions::Alphanumeric;
use url::Url;

pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: Error,
{
    tracing::error!("{}", err);
    //let labels = [("error", format!("{}", err))];
    //let counter = counter!("request_error", &labels);
    //counter.increment(1);
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

pub async fn with_timeout<F: Future>(
    duration_in_mills: u64,
    task: F,
) -> Result<<F as Future>::Output, (StatusCode, String)> {
    timeout(Duration::from_millis(duration_in_mills), task)
        .await
        .map_err(internal_error)
}

pub fn get_env(name: &str) -> String {
    env::var(name).expect(&format!("Environment variable {} is required", name))
}

pub fn get_header(name: &str, headers: &HeaderMap) -> Option<String> {
    headers
        .get(name)
        .map(|value| value.to_str().unwrap_or_default().to_string())
}

pub fn parse_url(text: &str) -> Result<String, (StatusCode, String)> {
    Url::parse(text)
        .map(|url| url.to_string())
        .map_err(|_| (StatusCode::BAD_REQUEST, "Malformed url".into()))
}

pub fn generate_id() -> String {
    let id: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();
    BASE64_URL_SAFE_NO_PAD.encode(id)
}
