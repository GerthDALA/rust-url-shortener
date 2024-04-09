use axum::{body::Body, extract::{Path, State}, http::StatusCode, response::{IntoResponse, Response}, Json};
use serde::{Deserialize, Serialize};
use sqlx::error::ErrorKind;
use sqlx::{Error, PgPool};
use url::Url;
use metrics::counter;

use crate::utils::{generate_id, internal_error};

const DEFAULT_CACHE_CONTROL_HEADER_VALUE: &str =
    "public, max-age=300, s-maxage=300, stale-while-revalidate=300, stale-if-error=300";
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub id: String,
    pub target_url: String
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewLink {
    pub target_url: String
}

pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, "The Service is HEALTHY")
}

pub async fn redirect(State(pool): State<PgPool>, Path(requesed_link_id): Path<String>) -> Result<Response, (StatusCode, String)> {
    let select_timeout  = tokio::time::Duration::from_millis(300);

    let link = tokio::time::timeout(
            select_timeout,
            sqlx::query_as!(
            Link,
            "SELECT id, target_url FROM links WHERE id = $1",
            requesed_link_id
        )
        .fetch_optional(&pool)
    )
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?
    .ok_or_else(|| "Not found".to_string())
    .map_err(|err| (StatusCode::NOT_FOUND, err))?;

    tracing::debug!(
        "Rederict link id {} to {}",
        requesed_link_id,
        link.id
    );
    Ok(Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header("Location", link.target_url)
        .header("Cache-Control", DEFAULT_CACHE_CONTROL_HEADER_VALUE)
        .body(Body::empty())
        .expect("This reponse should always be constructable"))
}

pub async fn create_link(State(pool): State<PgPool>, Json(new_link): Json<NewLink>) -> Result<Json<Link>, (StatusCode, String)> {
    let url = Url::parse(&new_link.target_url)
        .map_err(|_| (StatusCode::CONFLICT, "url malformed".into()))?
        .to_string();

    let insert_link_timout = tokio::time::Duration::from_millis(300);

    for _ in 1..=3 {
    
        let new_link_id = generate_id();

        let new_link = tokio::time::timeout(

        insert_link_timout,
        sqlx::query_as!(
                    Link,
                    r#"
                    WITH inserted_link AS (
                        INSERT INTO links(id, target_url)
                        VALUES ($1, $2)
                        RETURNING id, target_url
                    )
                    SELECT id, target_url FROM inserted_link
                    "#,
                    &new_link_id, // Also ensure this variable matches what you intend to insert.
                    &url
            )
                .fetch_one(&pool)
        ).await
        .map_err(internal_error)?;

        match new_link {
            Ok(link) => {
                tracing::debug!("Created new link with id {} targeting {}", new_link_id, url);
                return Ok(Json(link))
            },
            Err(err) => match err {
                Error::Database(db_err) if db_err.kind() == ErrorKind::UniqueViolation => {}
                _ => return Err(internal_error(err))
            }
        }
    }

    tracing::error!("Could ,oy perist nwe short link. Exhausted all retires of generating a unique id");
    counter!("saving_link_impossible_no_unique_id");
    Err((StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".into()))
}