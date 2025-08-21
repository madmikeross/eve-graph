use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Request to EVE-Scout API failed")]
    Request(#[from] reqwest::Error),
    #[error("EVE-Scout API server error ({status}): {body}")]
    ServerError { status: u16, body: String },
    #[error("Unexpected EVE-Scout API error ({status}): {body}")]
    UnexpectedError { status: u16, body: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EveScoutSignature {
    id: String,
    created_at: String,
    created_by_id: i64,
    created_by_name: String,
    updated_at: String,
    updated_by_id: i64,
    updated_by_name: String,
    completed_at: String,
    completed_by_id: i64,
    completed_by_name: String,
    completed: bool,
    wh_exits_outward: bool,
    wh_type: String,
    max_ship_size: String,
    expires_at: String,
    remaining_hours: i64,
    pub signature_type: String,
    pub out_system_id: i64,
    out_system_name: String,
    out_signature: String,
    pub in_system_id: i64,
    in_system_class: String,
    in_system_name: String,
    in_region_id: i64,
    in_region_name: String,
    in_signature: String,
    comment: Option<String>,
}

pub async fn get_public_signatures(client: Client) -> Result<Vec<EveScoutSignature>, Error> {
    let get_public_signatures_url = "https://api.eve-scout.com/v2/public/signatures";
    let response = client.get(get_public_signatures_url).send().await?;
    process_eve_scout_response(response).await
}

async fn process_eve_scout_response<T: for<'de> Deserialize<'de>>(
    response: Response,
) -> Result<T, Error> {
    let status = response.status();
    let url = response.url().clone();

    if status.is_success() {
        return response.json::<T>().await.map_err(Error::Request);
    }

    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "Could not read error body".to_string());
    error!(
        "EVE-Scout request to {} failed with status {}: {}",
        url, status, body
    );

    match status.as_u16() {
        500..=599 => Err(Error::ServerError {
            status: status.as_u16(),
            body,
        }),
        _ => Err(Error::UnexpectedError {
            status: status.as_u16(),
            body,
        }),
    }
}
