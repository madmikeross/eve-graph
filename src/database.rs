use std::sync::Arc;

use futures::StreamExt;
use neo4rs::{Graph, query};
use serde::Deserialize;
use crate::esi::SystemEsiResponse;

#[derive(Debug, Deserialize)]
pub(crate) struct System {
    pub constellation_id: Option<i64>,
    pub name: Option<String>,
    pub planets: Option<Vec<i64>>,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub security_class: Option<String>,
    pub security_status: f64,
    pub star_id: Option<i64>,
    pub stargates: Option<Vec<i64>>,
    pub system_id: i64,
}

pub(crate) async fn get_graph_client() -> Arc<Graph> {
    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let pass = "neo4jneo4j"; // assumes you have accessed via the browser and updated pass
    Arc::new(Graph::new(uri, user, pass).await.unwrap())
}

impl From<SystemEsiResponse> for System {
    fn from(s: SystemEsiResponse) -> Self {
        Self {
            constellation_id: s.constellation_id,
            name: s.name,
            planets: s.planets.map(|planets| planets.into_iter().map(|planet| planet.planet_id).collect()),
            x: s.position.x,
            y: s.position.y,
            z: s.position.z,
            security_class: s.security_class,
            security_status: s.security_status,
            star_id: s.star_id,
            stargates: s.stargates,
            system_id: s.system_id,
        }
    }
}



pub(crate) async fn system_id_exists(graph: &Graph, system_id: i64) -> Result<bool, neo4rs::Error> {
    let system_exists = "MATCH (s:System {system_id: $system_id}) RETURN COUNT(s) as count LIMIT 1";
    let mut result = graph.execute(query(system_exists).param("system_id", system_id)).await?;

    if let Some(row) = result.next().await? {
        Ok(row.get::<i64>("count").map_or(false, |count| count > 0))
    } else {
        Ok(false)
    }
}

pub(crate) async fn save_system(graph: &Arc<Graph>, system: &System) -> Result<(), neo4rs::Error> {
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
    let name_param = system.name.as_ref().map(|s| s.as_str()).unwrap_or("");
    let stargates = serde_json::to_string(&system.stargates).unwrap();
    let star_id = serde_json::to_string(&system.star_id).unwrap();

    graph.run(query(&create_statement)
        .param("system_id", system.system_id)
        .param("name", name_param)
        .param("constellation_id", constellation_id)
        .param("security_status", system.security_status)
        .param("star_id", star_id)
        .param("security_class", security_class_param)
        .param("x", system.x)
        .param("y", system.y)
        .param("z", system.z)
        .param("planets", planets_json)
        .param("stargates", stargates))
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use reqwest::Client;

    use crate::database::{get_graph_client, save_system};
    use crate::esi::{get_stargate, get_system_details};

    #[tokio::test]
    async fn can_save_system_to_database() {
        let client = Client::new();
        let graph = get_graph_client().await;

        let system_id = 30000201;
        let system = get_system_details(&client, system_id).await.unwrap();

        match save_system(&graph, &system).await {
            Ok(_) => {
                //TODO: Delete the record created
            }
            Err(_) => panic!("Could not save system")
        }
    }

    #[tokio::test]
    async fn can_retrieve_and_parse_stargate() {
        let client = Client::new();
        let stargate_id = 50011905;
        match get_stargate(&client, stargate_id).await {
            Ok(stargate) => {
                assert_eq!(stargate.stargate_id, stargate_id);
            }
            Err(err) => {
                panic!("Error in test: {}", err);
            }
        }
    }
}