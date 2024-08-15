use crate::utils::get_env;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::IntoResponse;
use sha3::{Digest, Sha3_256};

pub async fn auth(request: Request, next: Next) -> Result<impl IntoResponse, (StatusCode, String)> {
    // let labels = [("uri", format!("{}!", req.uri()))];
    // let counter = counter!("auth_error", &labels);
    let api_key_bytes = request
        .headers()
        .get("x-api-key")
        .map(|value| value.to_str().unwrap_or_default())
        .ok_or_else(|| {
            // tracing::error!("Unauthorized call to api");
            // counter.increment(1);
            (StatusCode::UNAUTHORIZED, "Unauthorized".into())
        })?
        .as_bytes();

    let mut hasher = Sha3_256::new();
    hasher.update(api_key_bytes);
    let provided_api_key = hasher.finalize();

    if get_env("ENCRYPTED_API_KEY") != format!("{provided_api_key:x}") {
        // tracing::error!("Unauthorized (invalid key)");
        // counter.increment(1);
        return Err((StatusCode::UNAUTHORIZED, "Unauthorized".into()));
    }
    Ok(next.run(request).await)
}
