use std::ops::Deref;
use std::sync::Arc;

use neo4rs::{Error, Graph, query};
use serde::{Deserialize, Serialize};
use std::error;
use crate::evescout::EveScoutSignature;


pub(crate) async fn get_graph_client() -> Arc<Graph> {
    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let pass = "neo4jneo4j"; // assumes you have accessed via the browser and updated pass
    Arc::new(Graph::new(uri, user, pass).await.unwrap())
}

pub(crate) async fn system_id_exists(graph: &Graph, system_id: i64) -> Result<bool, Error> {
    let system_exists = "MATCH (s:System {system_id: $system_id}) RETURN COUNT(s) as count LIMIT 1";
    let mut result = graph.execute(query(system_exists).param("system_id", system_id)).await?;

    match result.next().await? {
        Some(row) => Ok(row.get::<i64>("count").map_or(false, |count| count > 0)),
        None => Ok(false)
    }
}

pub(crate) async fn stargate_id_exists(graph: Arc<Graph>, stargate_id: i64) -> Result<bool, Error> {
    let stargate_exists = "MATCH (s:Stargate {stargate_id: $stargate_id}) RETURN COUNT(s) as count LIMIT 1";
    let mut result = graph.execute(query(stargate_exists).param("stargate_id", stargate_id)).await?;

    match result.next().await? {
        Some(row) => Ok(row.get::<i64>("count").map_or(false, |count| count > 0)),
        None => Ok(false)
    }
}

#[derive(Debug, Serialize, Deserialize)]
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

pub(crate) async fn save_system(graph: &Arc<Graph>, system: &System) -> Result<(), Error> {
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

    graph.run(query(&create_statement)
        .param("system_id", system.system_id)
        .param("name", system.name.clone())
        .param("constellation_id", system.constellation_id)
        .param("security_status", system.security_status)
        .param("star_id", system.star_id)
        .param("security_class", system.security_class.clone())
        .param("x", system.x)
        .param("y", system.y)
        .param("z", system.z)
        .param("planets", system.planets.clone())
        .param("stargates", system.stargates.clone()))
        .await?;

    Ok(())
}

pub(crate) async fn get_system(graph: Arc<Graph>, system_id: i64) -> Result<Option<System>, Error> {
    let get_system_statement = "MATCH (system:System {system_id: $system_id}) RETURN system LIMIT 1";
    let mut result = graph.execute(query(get_system_statement).param("system_id", system_id)).await?;

    match result.next().await? {
        Some(row) => {
            Ok(row.get("system").ok())
        }
        None => Ok(None),
    }
}

pub(crate) async fn get_stargate(graph: Arc<Graph>, stargate_id: i64) -> Result<Option<Stargate>, Error> {
    let get_stargate_statement = "MATCH (stargate:Stargate {stargate_id: $stargate_id}) RETURN stargate LIMIT 1";
    let mut result = graph.execute(query(get_stargate_statement).param("stargate_id", stargate_id)).await?;

    match result.next().await? {
        Some(row) => {
            Ok(row.get("stargate").ok())
        }
        None => Ok(None),
    }
}

pub async fn get_all_system_ids(graph: Arc<Graph>) -> Result<Vec<i64>, Error> {
    let get_all_system_ids_statement = "MATCH (s:System) RETURN s.system_id AS system_id";

    let mut result = graph.execute(query(get_all_system_ids_statement)).await?;
    let mut system_ids = Vec::new();

    while let Some(row) = result.next().await? {
        if let Ok(system_id) = row.get("system_id") {
            system_ids.push(system_id);
        }
    }

    Ok(system_ids)
}

pub async fn get_all_stargate_ids(graph: Arc<Graph>) -> Result<Vec<i64>, Error> {
    let get_all_stargate_ids_statement = "MATCH (s:Stargate) RETURN s.stargate_id AS stargate_id";

    let mut result = graph.execute(query(get_all_stargate_ids_statement)).await?;
    let mut stargate_ids = Vec::new();

    while let Some(row) = result.next().await? {
        if let Ok(stargate_id) = row.get("stargate_id") {
            stargate_ids.push(stargate_id);
        }
    }

    Ok(stargate_ids)
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Stargate {
    pub destination_stargate_id: i64,
    pub destination_system_id: i64,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub stargate_id: i64,
    pub system_id: i64,
    pub type_id: i64,
}

pub(crate) async fn save_stargate(graph: Arc<Graph>, stargate: &Stargate) -> Result<(), Error> {
    let create_statement = "
        CREATE (sg:Stargate {
            destination_stargate_id: $destination_stargate_id,
            destination_system_id: $destination_system_id,
            name: $name,
            x: $x,
            y: $y,
            z: $z,
            stargate_id: $stargate_id,
            system_id: $system_id,
            type_id: $type_id
        })";

    graph.run(query(&create_statement)
        .param("destination_stargate_id", stargate.destination_stargate_id)
        .param("destination_system_id", stargate.destination_system_id)
        .param("name", stargate.name.clone())
        .param("x", stargate.x)
        .param("y", stargate.y)
        .param("z", stargate.z)
        .param("stargate_id", stargate.stargate_id)
        .param("system_id", stargate.system_id)
        .param("type_id", stargate.type_id))
        .await?;

    Ok(())
}

pub(crate) async fn relate_stargate(graph: Arc<Graph>, stargate_id: i64) -> Result<(), Error> {
    match get_stargate(graph.clone(), stargate_id).await {
        Ok(stargate) => {
            match stargate {
                Some(stargate) => {
                    match create_system_jump(graph, stargate.system_id, stargate.destination_system_id).await {
                        Ok(_) => {}
                        Err(_) => {
                            println!("Error saving stargate relations {}", stargate_id);
                        }
                    }
                }
                None => {
                    println!("Stargate not found in database {}", stargate_id);
                }
            }
        }
        Err(_) => {
            println!("Error calling db to get stargate {}", stargate_id);
        }
    }

    Ok(())
}

pub(crate) async fn save_wormhole(graph: Arc<Graph>, signature: EveScoutSignature) -> Result<(), Error> {
    create_system_jump(graph.clone(), signature.in_system_id.clone(), signature.out_system_id.clone()).await?;
    create_system_jump(graph.clone(), signature.out_system_id.clone(), signature.in_system_id.clone()).await
}

pub(crate) async fn create_system_jump(graph: Arc<Graph>, source_system: i64, dest_system: i64) -> Result<(), Error> {
    let inbound_connection = "\
        MATCH (source:System {system_id: $source_system_id})\
        MATCH (dest:System {system_id: $dest_system_id})\
        CREATE (source)-[:JUMP {cost: 1}]->(dest)";

    graph.run(query(inbound_connection)
        .param("source_system_id", source_system)
        .param("dest_system_id", dest_system))
        .await
}

pub(crate) async fn drop_system_jump_graph(graph: &Arc<Graph>) -> Result<(), Error> {
    let drop_graph = "CALL gds.graph.drop('system-map')";
    graph.run(query(drop_graph)).await
}

pub(crate) async fn build_system_jump_graph(graph: Arc<Graph>) -> Result<(), Error> {
    let build_graph = "\
        CALL gds.graph.project(
            'system-map',
            'System',
            'JUMP',
            {
                relationshipProperties: 'cost'
            }
        )";
    graph.run(query(build_graph)).await
}

pub(crate) async fn drop_system_connections(graph: &Arc<Graph>, system_name: &str) -> Result<(), Error> {
    let drop_thera_connections = "\
        MATCH (:System {name: $system_name})-[r]-()
        DELETE r";

    graph.run(query(drop_thera_connections).param("system_name", system_name)).await?;
    Ok(())
}


pub(crate) async fn relate_all_systems(graph: Arc<Graph>) -> Result<(), Box<dyn error::Error + Send + Sync>> {
    let stargate_ids = get_all_stargate_ids(graph.clone()).await?;
    let stargate_relationships: Vec<_> = stargate_ids
        .iter()
        .map(|&stargate_id| tokio::spawn(relate_stargate(graph.clone(), stargate_id)))
        .collect();
    futures::future::try_join_all(stargate_relationships).await?;
    Ok(())
}

pub async fn rebuild_system_jump_graph(graph: Arc<Graph>) -> Result<(), neo4rs::Error> {
    drop_system_jump_graph(&graph).await?;
    build_system_jump_graph(graph).await
}

pub async fn relate_system_stargates(graph: Arc<Graph>, system_id: i64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let system = get_system(graph.clone(), system_id).await;

    let system_stargate_relationships: Vec<_> = system.unwrap().unwrap().stargates
        .unwrap()
        .iter()
        .map(|&stargate_id| tokio::spawn(relate_stargate(graph.clone(), stargate_id)))
        .collect();

    futures::future::try_join_all(system_stargate_relationships).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::database::{get_all_system_ids, get_graph_client, get_system};

    #[tokio::test]
    async fn should_read_system_from_database() {
        let system_id = 30001451;
        let system = get_system(get_graph_client().await, system_id).await;
        assert_eq!(system.unwrap().unwrap().system_id, system_id)
    }

    #[tokio::test]
    async fn should_get_all_saved_system_ids() {
        let system_ids = get_all_system_ids(get_graph_client().await).await;

        assert_eq!(system_ids.unwrap().len(), 8436)
    }
}