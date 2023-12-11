use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};

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
pub(crate) struct StargateEsiResponse {
    pub destination: Destination,
    pub name: String,
    pub position: Position,
    pub stargate_id: i64,
    pub system_id: i64,
    pub type_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Destination {
    pub stargate_id: i64,
    pub system_id: i64,
}

pub(crate) async fn get_system_details(client: &Client, system_id: i64) -> Result<SystemEsiResponse, reqwest::Error> {
    let system_detail_url = format!("https://esi.evetech.net/latest/universe/systems/{}", system_id);
    let response = client.get(&system_detail_url).send().await?;
    response.json().await
}

pub(crate) async fn get_stargate_details(client: &Client, stargate_id: i64) -> Result<StargateEsiResponse, reqwest::Error> {
    let stargate_url = format!("https://esi.evetech.net/latest/universe/stargates/{}", stargate_id);
    let response = client.get(&stargate_url).send().await?;
    response.json().await
}

pub(crate) async fn get_system_ids(client: &Client) -> Result<Vec<i64>, reqwest::Error> {
    let systems_url = "https://esi.evetech.net/latest/universe/systems/";
    let response = client.get(systems_url).send().await?;
    response.json().await
}
 #[derive(Debug)]
pub(crate) struct SystemKillsResponse {
    last_modified: Option<DateTime<Utc>>,
    system_kills: Vec<SystemKills>
}

#[derive(Debug, Deserialize)]
pub(crate) struct SystemKills {
    npc_kills: i64,
    pod_kills: i64,
    ship_kills: i64,
    system_id: i64,
}

pub(crate) async fn get_system_kills(client: &Client) -> Result<SystemKillsResponse, reqwest::Error> {
    let system_kills_url = "https://esi.evetech.net/latest/universe/system_kills/";
    let response = client.get(system_kills_url).send().await?;
    let last_modified: Option<DateTime<Utc>> = response.headers().get("Last-Modified")
        .unwrap()
        .to_str()
        .ok()
        .and_then(|s| {
            DateTime::parse_from_rfc2822(s)
                .or_else(|_| DateTime::parse_from_rfc3339(s))
                .ok()
        })
        .map(|datetime| datetime.with_timezone(&Utc));
    let system_kills = response.json().await?;
    Ok(SystemKillsResponse {
        last_modified,
        system_kills
    })
}

#[cfg(test)]
mod tests {
    use reqwest::Client;

    use crate::esi::get_system_kills;

    #[tokio::test]
    async fn should_get_system_kills() {
        let client = Client::new();
        let system_kills_response = get_system_kills(&client).await.unwrap();

        assert!(&system_kills_response.last_modified.is_some());
        assert!(&system_kills_response.system_kills.len() > &0);
    }
}