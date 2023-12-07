use std::sync::Arc;

use neo4rs::{Graph, query};
use serde::Deserialize;

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

pub(crate) async fn system_id_exists(graph: &Graph, system_id: i64) -> Result<bool, neo4rs::Error> {
    let system_exists = "MATCH (s:System {system_id: $system_id}) RETURN COUNT(s) as count LIMIT 1";
    let mut result = graph.execute(query(system_exists).param("system_id", system_id)).await?;

    match result.next().await? {
        Some(row) => Ok(row.get::<i64>("count").map_or(false, |count| count > 0)),
        None => Ok(false)
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

async fn get_system(graph: Arc<Graph>, system_id: i64) -> Result<Option<System>, neo4rs::Error> {
    let get_system_statement = "MATCH (system:System {system_id: $system_id}) RETURN system LIMIT 1";
    let mut result = graph.execute(query(get_system_statement).param("system_id", system_id)).await?;

    match result.next().await? {
        Some(row) => {
            let system: System = row.get("system").unwrap();
            Ok(Some(system))
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use crate::database::{get_graph_client, get_system};

    #[tokio::test]
    async fn should_read_system_from_database() {
        let system_id = 30000201;
        let system = get_system(get_graph_client().await, system_id).await;
        assert_eq!(system.unwrap().unwrap().system_id, system_id)
    }
}