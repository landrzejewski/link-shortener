use crate::dao;
use crate::model::{Link, LinkSpecification, LinkStatistics};
use crate::utils::{generate_id, get_header, internal_error, parse_url, with_timeout};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use sqlx::error::ErrorKind;
use sqlx::{Error, PgPool};
use std::string::String;

const CACHE_CONTROL_HEADER_VALUE: &str =
    "public, max-age=300, s-maxage=300, stale-while-revalidate=300, stale-if-error=300";
const DEFAULT_TIMEOUT: u64 = 300;

pub async fn create_link(
    State(db_connection_pool): State<PgPool>,
    Json(link_specification): Json<LinkSpecification>,
) -> Result<Json<Link>, (StatusCode, String)> {
    let url = parse_url(&link_specification.target_url)?;
    for _ in 1..=5 {
        let link = with_timeout(
            DEFAULT_TIMEOUT,
            dao::save(
                db_connection_pool.clone(),
                &generate_id(),
                &url,
                link_specification.expiration,
            ),
        )
        .await?;
        match link {
            Ok(link) => {
                return Ok(Json(link));
            }
            Err(err) => match err {
                Error::Database(db_err) if db_err.kind() == ErrorKind::UniqueViolation => {}
                _ => return Err(internal_error(err)),
            },
        }
    }
    tracing::error!("Could not persist new link. Exhausted all retries of generating a unique id");
    //let counter = counter!("no_unique_link");
    //counter.increment(1);
    Err((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal server error".into(),
    ))
}

pub async fn get_link_statistics(
    State(db_connection_pool): State<PgPool>,
    Path(link_id): Path<String>,
) -> Result<Json<Vec<LinkStatistics>>, (StatusCode, String)> {
    let statistics = with_timeout(
        DEFAULT_TIMEOUT,
        dao::get_statistics(db_connection_pool.clone(), &link_id),
    )
    .await?
    .map_err(internal_error)?;
    Ok(Json(statistics))
}

pub async fn update_link(
    State(db_connection_pool): State<PgPool>,
    Path(link_id): Path<String>,
    Json(updated_link): Json<LinkSpecification>,
) -> Result<Json<Link>, (StatusCode, String)> {
    let url = parse_url(&updated_link.target_url)?;
    let link = with_timeout(
        DEFAULT_TIMEOUT,
        dao::update(
            db_connection_pool.clone(),
            &link_id,
            &url,
            updated_link.expiration,
        ),
    )
    .await?;
    match link {
        Ok(link) => Ok(Json(link)),
        _ => Err((StatusCode::NOT_FOUND, "Not found".into())),
    }
}

pub async fn redirect(
    State(db_connection_pool): State<PgPool>,
    Path(link_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    let select_timeout = tokio::time::Duration::from_millis(300);

    let link: Link = tokio::time::timeout(
        select_timeout,
        dao::get_by_id(db_connection_pool.clone(), &link_id),
    )
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?
    .ok_or_else(|| "Not found".to_string())
    .map_err(|err| (StatusCode::NOT_FOUND, err))?;

    let referer = get_header("Referer", &headers);
    let user_agent = get_header("User-Agent", &headers);
    let saved_statistics = with_timeout(
        DEFAULT_TIMEOUT,
        dao::update_statistics(db_connection_pool.clone(), &link_id, &referer, &user_agent),
    )
    .await;

    match saved_statistics {
        Err(elapsed) => tracing::error!("Saving new link stats timeout: {:?}", elapsed),
        Ok(Err(err)) => tracing::error!("Saving new link stats failed: {}", err),
        _ => tracing::debug!("New link stats persisted"),
    }

    Ok(Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header("Location", link.target_url)
        .header("Cache-Control", CACHE_CONTROL_HEADER_VALUE)
        .body(Body::empty())
        .expect("Response build failed"))
}

pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}
