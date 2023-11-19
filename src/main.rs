use reqwest::{Client, Error};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct System {
    constellation_id: i64,
    name: String,
    planets: Vec<Planet>,
    position: Position,
    security_class: String,
    security_status: f64,
    star_id: i64,
    stargates: Vec<i64>,
    system_id: i64,
}

#[derive(Debug, Deserialize)]
struct Planet {
    planet_id: i64,
    asteroid_belts: Option<Vec<i64>>,
    moons: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
struct Position {
    x: f64,
    y: f64,
    z: f64,
}

#[tokio::main]
async fn main() {
    let client = Client::new();

    for system_id in get_system_ids(&client).await.unwrap() {
        let system_details = get_system_details(&client, system_id).await;
        println!("{:?}", system_details.unwrap());
    };
}

async fn get_system_details(client: &Client, system_id: i64) -> Result<System, Error> {
    let system_detail_url = format!("https://esi.evetech.net/latest/universe/systems/{}", system_id);
    let response = client.get(&system_detail_url).send().await?;
    let system_details: System = response.json().await?;
    Ok(system_details)
}

async fn get_system_ids(client: &Client) -> Result<Vec<i64>, Error> {
    let systems_url = "https://esi.evetech.net/latest/universe/systems/";
    let response = client.get(systems_url).send().await?;
    let system_ids: Vec<i64> = response.json().await?;
    Ok(system_ids)
}