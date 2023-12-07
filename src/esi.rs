use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::database::System;

#[derive(Debug, Deserialize)]
pub(crate) struct SystemEsiResponse {
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
pub(crate) struct Planet {
    pub planet_id: i64,
    pub asteroid_belts: Option<Vec<i64>>,
    pub moons: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Stargate {
    pub destination: Destination,
    pub name: String,
    pub position: Position,
    pub stargate_id: i64,
    pub system_id: i64,
    pub type_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Destination {
    stargate_id: i64,
    system_id: i64,
}

pub(crate) async fn get_system_details(client: &Client, system_id: i64) -> Result<System, reqwest::Error> {
    let system_detail_url = format!("https://esi.evetech.net/latest/universe/systems/{}", system_id);
    let response = client.get(&system_detail_url).send().await?;
    let system_esi_response: SystemEsiResponse = response.json().await?;

    Ok(System::from(system_esi_response))
}

pub(crate) async fn get_stargate(client: &Client, stargate_id: i64) -> Result<Stargate, reqwest::Error> {
    let stargate_url = format!("https://esi.evetech.net/latest/universe/stargates/{}", stargate_id);
    let response = client.get(&stargate_url).send().await?;
    response.json().await
}

pub(crate) async fn get_system_ids(client: &Client) -> Result<Vec<i64>, reqwest::Error> {
    let systems_url = "https://esi.evetech.net/latest/universe/systems/";
    let response = client.get(systems_url).send().await?;
    response.json().await
}