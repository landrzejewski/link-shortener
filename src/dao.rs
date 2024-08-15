use crate::model::{Link, LinkStatistics};
use chrono::{DateTime, Utc};
use sqlx::postgres::PgQueryResult;
use sqlx::{Error, Pool, Postgres};

pub async fn save(
    db_connection_pool: Pool<Postgres>,
    id: &str,
    url: &str,
    expiration: DateTime<Utc>,
) -> Result<Link, Error> {
    sqlx::query_as(
        r#"
              with inserted_link as (
                  insert into links(id, target_url, expiration) values ($1, $2, $3) returning id, target_url, expiration
              )
              select id, target_url, expiration from inserted_link
            "#
    )
        .bind(id)
        .bind(url)
        .bind(expiration)
        .fetch_one(&db_connection_pool)
        .await
}

pub async fn get_by_id(
    db_connection_pool: Pool<Postgres>,
    id: &str,
) -> Result<Option<Link>, Error> {
    sqlx::query_as("select id, target_url, expiration from links where id = $1")
        .bind(&id)
        .fetch_optional(&db_connection_pool)
        .await
}

pub async fn update(
    db_connection_pool: Pool<Postgres>,
    id: &str,
    url: &str,
    expiration: DateTime<Utc>,
) -> Result<Link, Error> {
    sqlx::query_as(
        r#"
              with updated_link as (
                  update links set target_url = $1, expiration = $2 where id = $3 returning id, target_url, expiration
              )
              select id, target_url, expiration from updated_link
            "#,
    )
        .bind(&url)
        .bind(&expiration)
        .bind(&id)
        .fetch_one(&db_connection_pool)
        .await
}

pub async fn delete_expired(db_connection_pool: Pool<Postgres>) {
    sqlx::query("delete from links where expiration <= $1")
        .bind(Utc::now())
        .execute(&db_connection_pool)
        .await
        .expect("Error while deleting expired links");
}

pub async fn get_statistics(
    db_connection_pool: Pool<Postgres>,
    link_id: &str,
) -> Result<Vec<LinkStatistics>, Error> {
    sqlx::query_as(
        r#"
              select count(*) as hits, referer, user_agent from link_statistics group by link_id, referer, user_agent having link_id = $1
            "#,
    )
        .bind(&link_id)
        .fetch_all(&db_connection_pool)
        .await
}

pub async fn update_statistics(
    db_connection_pool: Pool<Postgres>,
    id: &str,
    referer: &Option<String>,
    user_agent: &Option<String>,
) -> Result<PgQueryResult, Error> {
    sqlx::query(
        r#"
              insert into link_statistics(link_id, referer, user_agent) values ($1, $2, $3)
            "#,
    )
    .bind(&id)
    .bind(&referer)
    .bind(&user_agent)
    .execute(&db_connection_pool.clone())
    .await
}
