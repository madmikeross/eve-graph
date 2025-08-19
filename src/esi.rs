use reqwest::{Client, Error, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::error;
use warp::http;

use crate::esi::RequestError::HttpError;
use crate::esi::RequestError::ParseError;

#[derive(Debug, Deserialize)]
pub struct SystemEsiResponse {
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
pub struct StargateEsiResponse {
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
) -> Result<SystemEsiResponse, RequestError> {
    let system_detail_url = format!("https://esi.evetech.net/latest/universe/systems/{system_id}");
    let response = client.get(&system_detail_url).send().await?;
    response.json().await.map_err(HttpError)
}

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("failed to retrieve data from the source")]
    HttpError(#[from] Error),
    #[error("failed to parse data")]
    ParseError(#[from] serde_json::Error),
}

pub async fn get_stargate_details(
    client: &Client,
    stargate_id: i64,
) -> Result<StargateEsiResponse, RequestError> {
    let stargate_url = format!("https://esi.evetech.net/latest/universe/stargates/{stargate_id}");
    let response = client.get(&stargate_url).send().await?;
    let status_code = response.status();

    // Manually implement response.json so we can preserve bytes if we need to understand the error
    let response_bytes = response.bytes().await?;
    match serde_json::from_slice::<StargateEsiResponse>(&response_bytes).map_err(ParseError) {
        Ok(parsed_stargate) => Ok(parsed_stargate),
        Err(err) => {
            // Rebuild a response so we can print the text
            let response = Response::from(
                http::Response::builder()
                    .status(status_code)
                    .body(response_bytes)
                    .expect("Failed to rebuild response"),
            );
            error!(
                "{} {}: {}",
                status_code,
                stargate_url,
                response.text().await.unwrap()
            );
            Err(err)
        }
    }
}

pub async fn get_system_ids(client: &Client) -> Result<Vec<i64>, Error> {
    let systems_url = "https://esi.evetech.net/latest/universe/systems/";
    let response = client.get(systems_url).send().await?;
    response.json().await
}

#[derive(Debug)]
pub struct SystemKillsResponse {
    pub system_kills: Vec<SystemKills>,
}

#[derive(Debug, Deserialize)]
pub struct SystemKills {
    pub ship_kills: i32,
    pub system_id: i64,
}

pub async fn get_system_kills(client: &Client) -> Result<SystemKillsResponse, RequestError> {
    let system_kills_url = "https://esi.evetech.net/latest/universe/system_kills/";
    let response = client.get(system_kills_url).send().await?;
    let system_kills = response.json().await?;
    Ok(SystemKillsResponse { system_kills })
}

#[derive(Debug)]
pub struct SystemJumpsResponse {
    pub system_jumps: Vec<SystemJumps>,
}

#[derive(Debug, Deserialize)]
pub struct SystemJumps {
    pub ship_jumps: i32,
    pub system_id: i64,
}

pub async fn get_system_jumps(client: &Client) -> Result<SystemJumpsResponse, RequestError> {
    let system_jumps_url = "https://esi.evetech.net/latest/universe/system_jumps/";
    let response = client.get(system_jumps_url).send().await?;
    let system_jumps = response.json().await?;
    Ok(SystemJumpsResponse { system_jumps })
}
