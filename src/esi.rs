use reqwest::{Client, Error, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::error;

#[derive(Debug, Deserialize)]
pub struct SystemResponse {
    pub constellation_id: Option<i64>,
    pub name: Option<String>,
    pub planets: Option<Vec<Planet>>,
    pub position: Position,
    pub security_class: Option<String>,
    pub security_status: f64,
    pub star_id: Option<i64>,
    pub stargates: Option<Vec<i64>>,
    pub system_id: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Planet {
    pub planet_id: i64,
    pub asteroid_belts: Option<Vec<i64>>,
    pub moons: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Deserialize)]
pub struct StargateResponse {
    pub destination: Destination,
    pub name: String,
    pub position: Position,
    pub stargate_id: i64,
    pub system_id: i64,
    pub type_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Destination {
    pub stargate_id: i64,
    pub system_id: i64,
}

pub async fn get_system_details(
    client: &Client,
    system_id: i64,
) -> Result<SystemResponse, RequestError> {
    let system_detail_url = format!("https://esi.evetech.net/latest/universe/systems/{system_id}");
    let response = client.get(&system_detail_url).send().await?;
    response.json().await.map_err(RequestError::HttpError)
}

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("Request failed: {0}")]
    HttpError(#[from] Error),
    #[error("Failed to parse response body: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("Rate limited by ESI: {body}")]
    RateLimited { body: String },
    #[error("Resource not found (404): {body}")]
    NotFound { body: String },
    #[error("ESI server error ({status}): {body}")]
    ServerError { status: u16, body: String },
    #[error("Unexpected ESI error ({status}): {body}")]
    UnexpectedError { status: u16, body: String },
}

pub async fn get_stargate_details(
    client: &Client,
    stargate_id: i64,
) -> Result<StargateResponse, RequestError> {
    let stargate_url = format!("https://esi.evetech.net/latest/universe/stargates/{stargate_id}");
    let response = client.get(stargate_url).send().await?;
    process_response(response).await
}

pub async fn get_system_ids(client: &Client) -> Result<Vec<i64>, RequestError> {
    let systems_url = "https://esi.evetech.net/latest/universe/systems/";
    let response = client.get(systems_url).send().await?;
    process_response(response).await
}

#[derive(Debug, Deserialize)]
pub struct SystemKills {
    pub ship_kills: i32,
    pub system_id: i64,
}

pub async fn get_system_kills(client: &Client) -> Result<Vec<SystemKills>, RequestError> {
    let system_kills_url = "https://esi.evetech.net/latest/universe/system_kills/";
    let response = client.get(system_kills_url).send().await?;
    process_response(response).await
}

#[derive(Debug, Deserialize)]
pub struct SystemJumps {
    pub ship_jumps: i32,
    pub system_id: i64,
}

pub async fn get_system_jumps(client: &Client) -> Result<Vec<SystemJumps>, RequestError> {
    let system_jumps_url = "https://esi.evetech.net/latest/universe/system_jumps/";
    let response = client.get(system_jumps_url).send().await?;
    process_response(response).await
}

async fn process_response<T: for<'de> Deserialize<'de>>(
    response: Response,
) -> Result<T, RequestError> {
    let status = response.status();
    let url = response.url().clone();

    if status.is_success() {
        return response.json::<T>().await.map_err(RequestError::HttpError);
    }

    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "Could not read error body".to_string());
    error!(
        "ESI request to {} failed with status {}: {}",
        url, status, body
    );

    match status.as_u16() {
        404 => Err(RequestError::NotFound { body }),
        420 | 429 => Err(RequestError::RateLimited { body }),
        500..=599 => Err(RequestError::ServerError {
            status: status.as_u16(),
            body,
        }),
        _ => Err(RequestError::UnexpectedError {
            status: status.as_u16(),
            body,
        }),
    }
}
