use axum::http::StatusCode;
use base64::{engine::general_purpose, Engine};
use metrics::counter;
use rand::Rng;


pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error
{
    tracing::error!("{}", err);

    let labels = [("error", format!("{}!", err))];

    counter!("request_error", &labels);

    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())

}

pub fn generate_id() -> String {
    let random_number = rand::thread_rng().gen_range(0..u32::MAX);
    general_purpose::URL_SAFE_NO_PAD.encode(random_number.to_string())
}