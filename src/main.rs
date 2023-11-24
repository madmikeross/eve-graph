use neo4rs::{Graph, query};
use std::sync::Arc;
use reqwest::{Client};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct System {
    constellation_id: Option<i64>,
    name: String,
    planets: Vec<Planet>,
    position: Position,
    security_class: Option<String>,
    security_status: f64,
    star_id: i64,
    stargates: Option<Vec<i64>>,
    system_id: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    let client = Client::new();
    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let pass = "neo4jneo4j"; // assumes you have accessed via the browser and updated pass

    let graph = Arc::new(Graph::new(uri, user, pass).await.unwrap());

    let system_ids = get_system_ids(&client).await.unwrap();
    // let mut handles = vec![];

    for system_id in system_ids {
        let system_details = get_system_details(&client, system_id).await?;
        save_system_to_neo4j(&graph, &system_details).await?;
        print!("{:?}, ", system_details.system_id);
    };

    Ok(())
}

async fn get_system_details(client: &Client, system_id: i64) -> Result<System, reqwest::Error> {
    let system_detail_url = format!("https://esi.evetech.net/latest/universe/systems/{}", system_id);
    let response = client.get(&system_detail_url).send().await?;
    let system_details: System = response.json().await?;
    Ok(system_details)
}

async fn get_system_ids(client: &Client) -> Result<Vec<i64>, reqwest::Error> {
    let systems_url = "https://esi.evetech.net/latest/universe/systems/";
    let response = client.get(systems_url).send().await?;
    let system_ids: Vec<i64> = response.json().await?;
    Ok(system_ids)
}

async fn save_system_to_neo4j(graph: &Arc<Graph>, system: &System) -> Result<(), neo4rs::Error> {
    let create_statement = "
        CREATE (s:System {
            system_id: $system_id,
            name: $name,
            constellation_id: $constellation_id,
            security_status: $security_status,
            star_id: $star_id,
            security_class: $security_class,
            x: $x,
            y: $y,
            z: $z,
            planets: $planets,
            stargates: $stargates
        })";

    let constellation_id = serde_json::to_string(&system.constellation_id).unwrap();
    let planets_json = serde_json::to_string(&system.planets).unwrap();
    let security_class_param = system.security_class.as_ref().map(|s| s.as_str()).unwrap_or("");
    let stargates = serde_json::to_string(&system.stargates).unwrap();

    graph.run(query(&create_statement)
        .param("system_id", system.system_id)
        .param("name", &*system.name)
        .param("constellation_id", constellation_id)
        .param("security_status", system.security_status)
        .param("star_id", system.star_id)
        .param("security_class", security_class_param)
        .param("x", system.position.x)
        .param("y", system.position.y)
        .param("z", system.position.z)
        .param("planets", planets_json)
        .param("stargates", stargates))
        .await?;

    Ok(())
}