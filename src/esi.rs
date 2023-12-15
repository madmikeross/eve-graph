use chrono::{DateTime, Utc};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};

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
) -> Result<SystemEsiResponse, reqwest::Error> {
    let system_detail_url = format!(
        "https://esi.evetech.net/latest/universe/systems/{}",
        system_id
    );
    let response = client.get(&system_detail_url).send().await?;
    response.json().await
}

pub async fn get_stargate_details(
    client: &Client,
    stargate_id: i64,
) -> Result<StargateEsiResponse, reqwest::Error> {
    let stargate_url = format!(
        "https://esi.evetech.net/latest/universe/stargates/{}",
        stargate_id
    );
    let response = client.get(&stargate_url).send().await?;
    response.json().await
}

pub async fn get_system_ids(client: &Client) -> Result<Vec<i64>, reqwest::Error> {
    let systems_url = "https://esi.evetech.net/latest/universe/systems/";
    let response = client.get(systems_url).send().await?;
    response.json().await
}

#[derive(Debug)]
pub struct SystemKillsResponse {
    pub last_modified: Option<DateTime<Utc>>,
    pub system_kills: Vec<SystemKills>,
}

#[derive(Debug, Deserialize)]
pub struct SystemKills {
    npc_kills: i64,
    pod_kills: i64,
    pub ship_kills: i32,
    pub system_id: i64,
}

pub async fn get_system_kills(client: &Client) -> Result<SystemKillsResponse, reqwest::Error> {
    let system_kills_url = "https://esi.evetech.net/latest/universe/system_kills/";
    let response = client.get(system_kills_url).send().await?;
    let last_modified = get_last_modified_date(&response);
    let system_kills = response.json().await?;
    Ok(SystemKillsResponse {
        last_modified,
        system_kills,
    })
}

fn get_last_modified_date(response: &Response) -> Option<DateTime<Utc>> {
    let last_modified: Option<DateTime<Utc>> = response
        .headers()
        .get("Last-Modified")
        .unwrap()
        .to_str()
        .ok()
        .and_then(|s| {
            DateTime::parse_from_rfc2822(s)
                .or_else(|_| DateTime::parse_from_rfc3339(s))
                .ok()
        })
        .map(|datetime| datetime.with_timezone(&Utc));
    last_modified
}

#[derive(Debug)]
pub struct SystemJumpsResponse {
    pub last_modified: Option<DateTime<Utc>>,
    pub system_jumps: Vec<SystemJumps>,
}

#[derive(Debug, Deserialize)]
pub struct SystemJumps {
    pub ship_jumps: i32,
    pub system_id: i64,
}

pub async fn get_system_jumps(client: &Client) -> Result<SystemJumpsResponse, reqwest::Error> {
    let system_jumps_url = "https://esi.evetech.net/latest/universe/system_jumps/";
    let response = client.get(system_jumps_url).send().await?;
    let last_modified = get_last_modified_date(&response);
    let system_jumps = response.json().await?;
    Ok(SystemJumpsResponse {
        last_modified,
        system_jumps,
    })
}
