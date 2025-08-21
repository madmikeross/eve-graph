use std::env;
use std::sync::Arc;
use std::time::Duration;

use neo4rs::{query, DeError, Error as Neo4rsError, Graph, Row};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

#[derive(Debug, thiserror::Error)]
#[error("GDS procedure call '{0}' did not return the expected row")]
pub struct GdsProcedureError(&'static str);

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Client(#[from] Neo4rsError),
    #[error(transparent)]
    Gds(#[from] GdsProcedureError),
}

pub async fn get_graph_client_with_retry(
    max_retries: usize,
    db_name: Option<&str>,
) -> Result<Arc<Graph>, Error> {
    info!("Connecting to Neo4j...");
    // Default to `localhost` for local tests, and `neo4j` for the Docker environment.
    // This can still be overridden by the NEO4J_HOSTNAME environment variable.
    let default_hostname = if cfg!(test) { "127.0.0.1" } else { "neo4j" };
    let neo4j_hostname =
        env::var("NEO4J_HOSTNAME").unwrap_or_else(|_| default_hostname.to_string());
    let db = db_name.unwrap_or("neo4j");
    let uri = format!("bolt://{neo4j_hostname}:7687?database={db}");
    let user = "neo4j";
    let pass = "neo4jneo4j";

    let mut last_error = None;

    for attempt in 1..=max_retries {
        info!(
            "Attempt {}/{} to connect to Neo4j at {}",
            attempt, max_retries, uri
        );
        match Graph::new(&uri, user, pass).await {
            Ok(graph) => {
                info!("Successfully connected to Neo4j.");
                return Ok(Arc::new(graph));
            }
            Err(err) => {
                warn!("Connection attempt {} failed: {}", attempt, err);
                last_error = Some(err);
                if attempt < max_retries {
                    let sleep_duration = Duration::from_secs(5);
                    warn!("Retrying in {:?}...", sleep_duration);
                    tokio::time::sleep(sleep_duration).await;
                }
            }
        }
    }

    error!("Failed to connect to Neo4j after {} attempts.", max_retries);
    Err(Error::Client(
        last_error.unwrap_or(Neo4rsError::ConnectionError),
    )) // Return the last error or a generic one
}

fn row_count_is_positive(row: Row) -> bool {
    row.get::<i64>("count").is_ok_and(|count| count > 0)
}

pub async fn stargate_id_exists(graph: Arc<Graph>, stargate_id: i64) -> Result<bool, Error> {
    let stargate_exists =
        "MATCH (s:Stargate {stargate_id: $stargate_id}) RETURN COUNT(s) as count LIMIT 1";
    let mut result = graph
        .execute(query(stargate_exists).param("stargate_id", stargate_id))
        .await?;

    match result.next().await? {
        Some(row) => Ok(row_count_is_positive(row)),
        None => Ok(false),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct System {
    pub constellation_id: i64,
    pub name: String,
    pub planets: Vec<i64>,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub security_class: String,
    pub security_status: f64,
    pub star_id: i64,
    pub stargates: Vec<i64>,
    pub system_id: i64,
    pub kills: u32,
    pub jumps: u32,
}

pub async fn save_system(graph: &Arc<Graph>, system: &System) -> Result<(), Error> {
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
            stargates: $stargates,
            kills: $kills,
            jumps: $jumps
        })";

    graph
        .run(
            query(create_statement)
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
                .param("stargates", system.stargates.clone())
                .param("kills", system.kills)
                .param("jumps", system.jumps),
        )
        .await?;
    Ok(())
}

pub async fn get_system(graph: Arc<Graph>, system_id: i64) -> Result<Option<System>, Error> {
    let get_system_statement =
        "MATCH (system:System {system_id: $system_id}) RETURN system LIMIT 1";
    let mut result = graph
        .execute(query(get_system_statement).param("system_id", system_id))
        .await?;

    match result.next().await? {
        Some(row) => Ok(row.get("system")?),
        None => Ok(None),
    }
}

pub async fn get_all_systems(graph: Arc<Graph>) -> Result<Vec<System>, Error> {
    let get_all_systems_statement = "MATCH (s:System) RETURN s as system";
    let mut result = graph.execute(query(get_all_systems_statement)).await?;
    let mut systems = Vec::new();

    while let Some(row) = result.next().await? {
        if let Ok(system) = row.get("system") {
            systems.push(system);
        }
    }

    Ok(systems)
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

pub async fn get_saved_system_count(graph: &Arc<Graph>) -> Result<i64, Error> {
    let get_system_count = "MATCH (s:System) RETURN COUNT(s) as count";
    let mut result = graph.execute(query(get_system_count)).await?;
    let row = result.next().await?;

    match row {
        None => Ok(0),
        Some(row) => Ok(row.get("count")?),
    }
}
pub async fn get_saved_stargate_count(graph: &Arc<Graph>) -> Result<i64, Error> {
    let get_stargate_count = "MATCH (sg:Stargate) RETURN COUNT(sg) as count";
    let mut result = graph.execute(query(get_stargate_count)).await?;
    let row = result.next().await?;

    match row {
        None => Ok(0),
        Some(row) => Ok(row.get("count")?),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Stargate {
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

pub async fn save_stargate(graph: Arc<Graph>, stargate: &Stargate) -> Result<(), Error> {
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

    graph
        .run(
            query(create_statement)
                .param("destination_stargate_id", stargate.destination_stargate_id)
                .param("destination_system_id", stargate.destination_system_id)
                .param("name", stargate.name.clone())
                .param("x", stargate.x)
                .param("y", stargate.y)
                .param("z", stargate.z)
                .param("stargate_id", stargate.stargate_id)
                .param("system_id", stargate.system_id)
                .param("type_id", stargate.type_id),
        )
        .await?;

    create_system_jump_if_missing(graph, stargate.system_id, stargate.destination_system_id)
        .await?;
    Ok(())
}

pub async fn save_wormhole(
    graph: Arc<Graph>,
    in_system_id: i64,
    out_system_id: i64,
) -> Result<(), Error> {
    debug!("Saving wormhole from {} to {}", in_system_id, out_system_id);
    create_system_jump(graph.clone(), in_system_id, out_system_id).await?;
    create_system_jump(graph.clone(), out_system_id, in_system_id).await
}

pub async fn set_last_hour_system_jumps(
    graph: Arc<Graph>,
    system_id: i64,
    jumps: i32,
) -> Result<(), Error> {
    let set_system_jumps_statement = "
        MATCH (s:System {system_id: $system_id})
        SET s.jumps = $jumps";

    graph
        .run(
            query(set_system_jumps_statement)
                .param("system_id", system_id)
                .param("jumps", jumps),
        )
        .await?;
    Ok(())
}

pub async fn set_last_hour_system_kills(
    graph: Arc<Graph>,
    system_id: i64,
    kills: i32,
) -> Result<(), Error> {
    let set_system_kills_statement = "
        MATCH (s:System {system_id: $system_id})
        SET s.kills = $kills";

    graph
        .run(
            query(set_system_kills_statement)
                .param("system_id", system_id)
                .param("kills", kills),
        )
        .await?;
    Ok(())
}

pub async fn set_system_jump_risk(
    graph: Arc<Graph>,
    system_id: i64,
    baseline_jump_risk: f64,
) -> Result<(), Error> {
    let system = match get_system(graph.clone(), system_id).await {
        Ok(Some(system)) => system,
        Ok(None) => return Ok(()),
        Err(e) => {
            warn!("System could not be retrieved when trying to set jump risk {system_id}: {e:?}");
            return Err(e);
        }
    };

    let total_risk = calculate_total_risk(system.kills, system.jumps, baseline_jump_risk);

    debug!("Setting jump risks into system {system_id} as {total_risk}");
    let set_system_risk = "
         MATCH (otherSystem)-[r:JUMP]->(s:System {system_id: $system_id})
         SET r.risk = $risk";
    graph
        .run(
            query(set_system_risk)
                .param("system_id", system_id)
                .param("risk", total_risk),
        )
        .await?;
    Ok(())
}

fn calculate_total_risk(kills: u32, jumps: u32, baseline_jump_risk: f64) -> f64 {
    let kills_squared = u32::pow(kills, 2);
    let system_jump_risk: f64 = if jumps > 0 {
        kills_squared as f64 / jumps as f64
    } else {
        kills_squared.into()
    };
    system_jump_risk + baseline_jump_risk
}

async fn jump_exists(
    graph: Arc<Graph>,
    source_system: i64,
    dest_system: i64,
) -> Result<bool, Error> {
    let jump_exists_statement = "\
        MATCH (:System {system_id: $source_system})-[r:JUMP]->(:System {system_id: $dest_system})
        RETURN COUNT(r) AS count";
    let mut result = graph
        .execute(
            query(jump_exists_statement)
                .param("source_system", source_system)
                .param("dest_system", dest_system),
        )
        .await?;
    match result.next().await? {
        Some(row) => Ok(row_count_is_positive(row)),
        None => Ok(false),
    }
}

async fn create_system_jump_if_missing(
    graph: Arc<Graph>,
    source_system: i64,
    dest_system: i64,
) -> Result<(), Error> {
    if !jump_exists(graph.clone(), source_system, dest_system).await? {
        create_system_jump(graph, source_system, dest_system).await?;
    }

    Ok(())
}

pub async fn create_system_jump(
    graph: Arc<Graph>,
    source_system: i64,
    dest_system: i64,
) -> Result<(), Error> {
    let inbound_connection = "\
        MATCH (source:System {system_id: $source_system_id})
        MATCH (dest:System {system_id: $dest_system_id})
        CREATE (source)-[:JUMP {cost: 1}]->(dest)";

    graph
        .run(
            query(inbound_connection)
                .param("source_system_id", source_system)
                .param("dest_system_id", dest_system),
        )
        .await?;
    Ok(())
}

pub async fn graph_exists(graph: &Arc<Graph>, graph_name: String) -> Result<bool, Error> {
    let list_of_graphs_query = "CALL gds.graph.list";
    let mut result = graph.execute(query(list_of_graphs_query)).await?;

    while let Some(row) = result.next().await? {
        if let Ok(name) = row.get::<String>("graphName") {
            if name == graph_name {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

pub async fn drop_system_jump_graph(graph: &Arc<Graph>) -> Result<String, Error> {
    let drop_graph = "CALL gds.graph.drop('system-map')";
    let mut result = graph.execute(query(drop_graph)).await?;
    let row = result
        .next()
        .await?
        .ok_or(GdsProcedureError("drop system-map"))?;
    Ok(row.get("graphName")?)
}

pub async fn drop_jump_risk_graph(graph: &Arc<Graph>) -> Result<String, Error> {
    let drop_graph = "CALL gds.graph.drop('jump-risk')";
    let mut result = graph.execute(query(drop_graph)).await?;
    let row = result
        .next()
        .await?
        .ok_or(GdsProcedureError("drop jump-risk"))?;
    Ok(row.get("graphName")?)
}

pub async fn build_system_jump_graph(graph: Arc<Graph>) -> Result<String, Error> {
    let build_graph = "\
        CALL gds.graph.project(
            'system-map',
            'System',
            'JUMP',
            {
                relationshipProperties: 'cost'
            }
        )";
    let mut result = graph.execute(query(build_graph)).await?;
    let row = result
        .next()
        .await?
        .ok_or(GdsProcedureError("project system-map"))?;
    Ok(row.get("graphName")?)
}

pub async fn build_jump_risk_graph(graph: Arc<Graph>) -> Result<String, Error> {
    let build_graph = "\
        CALL gds.graph.project(
            'jump-risk',
            'System',
            'JUMP',
            {
                relationshipProperties: 'risk'
            }
        )";
    let mut result = graph.execute(query(build_graph)).await?;
    let row = result
        .next()
        .await?
        .ok_or(GdsProcedureError("project jump-risk"))?;
    Ok(row.get("graphName")?)
}

pub async fn drop_system_connections(graph: &Arc<Graph>, system_name: &str) -> Result<(), Error> {
    let drop_thera_connections = "\
        MATCH (:System {name: $system_name})-[r]-()
        DELETE r";
    graph
        .run(query(drop_thera_connections).param("system_name", system_name))
        .await?;
    Ok(())
}

pub async fn refresh_jump_cost_graph(graph: Arc<Graph>) -> Result<(), Error> {
    if graph_exists(&graph, String::from("system-map")).await? {
        drop_system_jump_graph(&graph).await?;
    }
    let _ = build_system_jump_graph(graph).await?;
    Ok(())
}

pub async fn refresh_jump_risk_graph(graph: Arc<Graph>) -> Result<(), Error> {
    if graph_exists(&graph, String::from("jump-risk")).await? {
        drop_jump_risk_graph(&graph).await?;
    }
    let _ = build_jump_risk_graph(graph.clone()).await?;
    Ok(())
}

pub async fn find_shortest_route(
    graph: Arc<Graph>,
    from_system_name: String,
    to_system_name: String,
) -> Result<Option<Vec<String>>, Error> {
    let shortest_path_query = "\
        MATCH (source:System {name: $from_system_name}), (target:System {name: $to_system_name})
        CALL gds.shortestPath.dijkstra.stream('system-map', {
            sourceNode: source,
            targetNode: target,
            relationshipWeightProperty: 'cost'
        })
        YIELD index, sourceNode, targetNode, totalCost, nodeIds, costs, path
        RETURN
            [nodeId IN nodeIds | gds.util.asNode(nodeId).name] AS nodeNames
    ";

    let mut result = graph
        .execute(
            query(shortest_path_query)
                .param("from_system_name", from_system_name)
                .param("to_system_name", to_system_name),
        )
        .await?;

    match result.next().await? {
        Some(row) => row.get("nodeNames").map_err(Error::from),
        None => Ok(None),
    }
}

pub async fn find_safest_route(
    graph: Arc<Graph>,
    from_system_name: String,
    to_system_name: String,
) -> Result<Option<Vec<String>>, Error> {
    let shortest_path_query = "\
        MATCH (source:System {name: $from_system_name}), (target:System {name: $to_system_name})
        CALL gds.shortestPath.dijkstra.stream('jump-risk', {
            sourceNode: source,
            targetNode: target,
            relationshipWeightProperty: 'risk'
        })
        YIELD index, sourceNode, targetNode, totalCost, nodeIds, costs, path
        RETURN
            [nodeId IN nodeIds | gds.util.asNode(nodeId).name] AS nodeNames
    ";

    let mut result = graph
        .execute(
            query(shortest_path_query)
                .param("from_system_name", from_system_name)
                .param("to_system_name", to_system_name),
        )
        .await?;

    match result.next().await? {
        Some(row) => row.get("nodeNames").map_err(Error::from),
        None => Ok(None),
    }
}

pub async fn remove_duplicate_systems(graph: Arc<Graph>) -> Result<(), Error> {
    let remove_duplicates = "
        MATCH (s:System)
        WITH s.system_id AS systemId, COLLECT(s) AS duplicates, COUNT(*) AS count
        WHERE count > 1
        FOREACH (duplicate IN TAIL(duplicates) | DETACH DELETE duplicate)";

    graph.run(query(remove_duplicates)).await?;
    Ok(())
}

pub async fn remove_systems_by_id(graph: Arc<Graph>, system_ids: Vec<i64>) -> Result<(), Error> {
    let remove_by_ids = "
        MATCH (s:System)
        WHERE s.system_id IN $ids
        DETACH DELETE s";

    graph
        .run(query(remove_by_ids).param("ids", system_ids))
        .await?;
    Ok(())
}

pub async fn remove_duplicate_stargates(graph: Arc<Graph>) -> Result<(), Error> {
    let remove_duplicates = "
        MATCH (s:Stargate)
        WITH s.stargate_id AS stargateId, COLLECT(s) AS duplicates, COUNT(*) AS count
        WHERE count > 1
        FOREACH (duplicate IN TAIL(duplicates) | DETACH DELETE duplicate)";

    graph.run(query(remove_duplicates)).await?;
    Ok(())
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

pub async fn remove_stargates_by_id(
    graph: Arc<Graph>,
    stargate_ids: Vec<i64>,
) -> Result<(), Error> {
    let remove_by_ids = "
        MATCH (s:Stargate)
        WHERE s.stargate_id IN $ids
        DETACH DELETE s";

    graph
        .run(query(remove_by_ids).param("ids", stargate_ids))
        .await?;
    Ok(())
}

impl From<DeError> for Error {
    fn from(e: DeError) -> Self {
        Error::Client(Neo4rsError::DeserializationError(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_total_risk_no_activity() {
        // With no system activity, risk should just be the baseline.
        let risk = calculate_total_risk(0, 0, 0.1);
        assert_eq!(risk, 0.1);
    }

    #[test]
    fn test_calculate_total_risk_with_kills_no_jumps() {
        // With no jumps, risk is kills_squared + baseline.
        let risk = calculate_total_risk(5, 0, 0.1);
        assert_eq!(risk, 25.1);
    }

    #[test]
    fn test_calculate_total_risk_with_jumps_no_kills() {
        // With no kills, risk should just be the baseline.
        let risk = calculate_total_risk(0, 100, 0.1);
        assert_eq!(risk, 0.1);
    }
    #[test]
    fn test_calculate_total_risk_normal_activity() {
        // (10^2 / 200) + 0.1 = 100 / 200 + 0.1 = 0.5 + 0.1 = 0.6
        let risk = calculate_total_risk(10, 200, 0.1);
        assert!((risk - 0.6).abs() < f64::EPSILON);
    }
}
