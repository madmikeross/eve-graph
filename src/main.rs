use std::cmp::Ordering;
use std::convert::Infallible;
use std::sync::Arc;

use neo4rs::Graph;
use reqwest::Client;
use thiserror::Error;
use tokio::sync::Semaphore;
use tokio::task::{JoinError, JoinSet};
use tracing::{error, info, instrument};
use warp::hyper::StatusCode;
use warp::reject::Reject;
use warp::reply::json;
use warp::{reply, Filter, Rejection, Reply};

use crate::database::*;
use crate::esi::{
    get_stargate_details, get_system_details, get_system_ids, get_system_jumps, get_system_kills,
    RequestError, StargateEsiResponse, SystemEsiResponse,
};
use crate::eve_scout::get_public_signatures;
use crate::ReplicationError::Target;

mod database;
mod esi;
mod eve_scout;

#[tokio::main]
async fn main() {
    // Initialize logging, defaulting to `info` level for our crate and `warn` for `neo4rs`
    // if the `RUST_LOG` environment variable is not set.
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,neo4rs=warn"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    info!("Starting eve-graph");
    let client = Client::new();
    let graph = get_graph_client_with_retry(10).await.unwrap();

    let systems_refresh = warp::path!("systems" / "refresh");
    let systems_risk = warp::path!("systems" / "risk");
    let systems_routes = systems_risk
        .and(warp::post())
        .and(with_client(client.clone()))
        .and(with_graph(graph.clone()))
        .and_then(systems_risk_handler)
        .or(systems_refresh
            .and(warp::post())
            .and(with_client(client.clone()))
            .and(with_graph(graph.clone()))
            .and_then(systems_refresh_handler));

    let stargates_refresh = warp::path!("stargates" / "refresh");
    let stargates_routes = stargates_refresh
        .and(warp::post())
        .and(with_client(client.clone()))
        .and(with_graph(graph.clone()))
        .and_then(stargates_refresh_handler);

    let wormholes_refresh = warp::path!("wormholes" / "refresh");
    let wormholes_routes = wormholes_refresh
        .and(warp::post())
        .and(with_client(client.clone()))
        .and(with_graph(graph.clone()))
        .and_then(wormholes_refresh_handler);

    let shortest_route_to = warp::path!("shortest-route" / String / "to" / String);
    let safest_route_to = warp::path!("safest-route" / String / "to" / String);
    let routes_routes = shortest_route_to
        .and(warp::get())
        .and(with_graph(graph.clone()))
        .and_then(shortest_route_to_handler)
        .or(safest_route_to
            .and(warp::get())
            .and(with_graph(graph.clone()))
            .and_then(safest_route_to_handler));

    let service_routes = routes_routes
        .or(wormholes_routes)
        .or(systems_routes)
        .or(stargates_routes)
        .recover(handle_rejection);

    // Build or refresh all data
    match synchronize_esi_systems(client.clone(), graph.clone()).await {
        Ok(_) => {
            // Stargate sync relies on systems being saved
            match synchronize_esi_stargates(client.clone(), graph.clone()).await {
                Ok(_) => {
                    // Jump risk sync relies on connections existing from stargate sync
                    refresh_jump_risks(client.clone(), graph.clone())
                        .await
                        .unwrap();
                    refresh_jump_risk_graph(graph.clone()).await.unwrap();
                }
                Err(err) => error!("Stargate synchronization failed {}", err),
            }

            refresh_eve_scout_system_relations(client.clone(), graph.clone())
                .await
                .unwrap();
            refresh_jump_cost_graph(graph).await.unwrap();
        }
        Err(err) => error!("System synchronization failed {}", err),
    }

    info!("Serving routes on 8008");
    warp::serve(service_routes).run(([0, 0, 0, 0], 8008)).await;
}

fn with_client(client: Client) -> impl Filter<Extract = (Client,), Error = Infallible> + Clone {
    warp::any().map(move || client.clone())
}

fn with_graph(
    graph: Arc<Graph>,
) -> impl Filter<Extract = (Arc<Graph>,), Error = Infallible> + Clone {
    warp::any().map(move || graph.clone())
}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    if err.is_not_found() {
        return Ok(reply::with_status("NOT_FOUND", StatusCode::NOT_FOUND));
    }

    if err.find::<ReplicationError>().is_some() {
        return Ok(reply::with_status(
            "INTERNAL_SERVER_ERROR",
            StatusCode::INTERNAL_SERVER_ERROR,
        ));
    }

    if err.find::<neo4rs::Error>().is_some() {
        return Ok(reply::with_status(
            "INTERNAL_SERVER_ERROR",
            StatusCode::INTERNAL_SERVER_ERROR,
        ));
    }

    error!("unhandled rejection: {:?}", err);
    Ok(reply::with_status(
        "INTERNAL_SERVER_ERROR",
        StatusCode::INTERNAL_SERVER_ERROR,
    ))
}

async fn shortest_route_to_handler(
    from_system_name: String,
    to_system_name: String,
    graph: Arc<Graph>,
) -> Result<impl Reply, Rejection> {
    match find_shortest_route(graph, from_system_name, to_system_name)
        .await
        .unwrap()
    {
        None => Ok(json::<String>(&String::from(""))),
        Some(route) => Ok(json::<Vec<_>>(&route)),
    }
}

async fn safest_route_to_handler(
    from_system_name: String,
    to_system_name: String,
    graph: Arc<Graph>,
) -> Result<impl Reply, Rejection> {
    let exists = graph_exists(&graph, String::from("jump-risk"))
        .await
        .map_err(Target);
    if !exists.unwrap() {
        let _ = build_jump_risk_graph(graph.clone()).await.map_err(Target);
    }

    match find_safest_route(graph, from_system_name, to_system_name)
        .await
        .unwrap()
    {
        None => Ok(json::<String>(&String::from(""))),
        Some(route) => Ok(json::<Vec<_>>(&route)),
    }
}

async fn wormholes_refresh_handler(
    client: Client,
    graph: Arc<Graph>,
) -> Result<impl Reply, Rejection> {
    refresh_eve_scout_system_relations(client, graph.clone())
        .await
        .unwrap();
    refresh_jump_cost_graph(graph).await.unwrap();
    Ok(reply())
}

async fn systems_risk_handler(client: Client, graph: Arc<Graph>) -> Result<impl Reply, Rejection> {
    refresh_jump_risks(client, graph.clone()).await?;
    refresh_jump_risk_graph(graph).await.unwrap();
    Ok(reply())
}

async fn systems_refresh_handler(
    client: Client,
    graph: Arc<Graph>,
) -> Result<impl Reply, Rejection> {
    synchronize_esi_systems(client, graph).await?;
    Ok(reply())
}

async fn stargates_refresh_handler(
    client: Client,
    graph: Arc<Graph>,
) -> Result<impl Reply, Rejection> {
    pull_all_stargates(client, graph.clone()).await?;
    refresh_jump_cost_graph(graph).await.unwrap();
    Ok(reply())
}

async fn refresh_eve_scout_system_relations(
    client: Client,
    graph: Arc<Graph>,
) -> Result<(), ReplicationError> {
    info!("Refreshing EVE Scout Public Connections");
    drop_system_connections(&graph, "Thera").await?;
    drop_system_connections(&graph, "Turnur").await?;

    let mut set = JoinSet::new();

    get_public_signatures(client.clone())
        .await?
        .iter()
        .filter(|sig| sig.signature_type == "wormhole")
        .for_each(|wormhole| {
            set.spawn(save_wormhole(graph.clone(), wormhole.clone()));
        });

    error_if_any_member_has_error(&mut set)
        .await
        .unwrap()
        .map_err(Target)
}

async fn pull_all_stargates(client: Client, graph: Arc<Graph>) -> Result<(), ReplicationError> {
    info!("Pulling all stargates from ESI");
    let mut set = JoinSet::new();

    let systems = get_all_systems(graph.clone()).await?;
    let all_stargate_ids: Vec<i64> = systems.into_iter().flat_map(|s| s.stargates).collect();
    info!("Found {} stargates to process.", all_stargate_ids.len());

    let semaphore = Arc::new(Semaphore::new(50));

    for stargate_id in all_stargate_ids {
        let client = client.clone();
        let graph = graph.clone();
        let semaphore = semaphore.clone();
        set.spawn(async move {
            if !stargate_id_exists(graph.clone(), stargate_id).await? {
                let _permit = semaphore.acquire().await.unwrap();
                pull_stargate(client, graph, stargate_id).await
            } else {
                Ok(())
            }
        });
    }

    error_if_any_member_has_error(&mut set).await.unwrap()
}

async fn synchronize_esi_systems(
    client: Client,
    graph: Arc<Graph>,
) -> Result<(), ReplicationError> {
    info!("Synchronizing systems with ESI");

    // 1. Get all system IDs from ESI (source of truth)
    let esi_system_ids = get_system_ids(&client)
        .await
        .map_err(|e| ReplicationError::Source(RequestError::HttpError(e)))?;
    let esi_system_ids_set: std::collections::HashSet<i64> = esi_system_ids.into_iter().collect();

    // 2. Get all system IDs from our DB
    let db_system_ids = get_all_system_ids(graph.clone()).await?;
    let db_system_ids_set: std::collections::HashSet<i64> = db_system_ids.into_iter().collect();

    // 3. Find systems to remove (in DB but not in ESI)
    let to_remove: Vec<i64> = db_system_ids_set
        .difference(&esi_system_ids_set)
        .cloned()
        .collect();
    if !to_remove.is_empty() {
        info!(
            "Removing {} stale systems from the database.",
            to_remove.len()
        );
        remove_systems_by_id(graph.clone(), to_remove).await?;
    }

    // 4. Find systems to add (in ESI but not in DB)
    let to_add: Vec<i64> = esi_system_ids_set
        .difference(&db_system_ids_set)
        .cloned()
        .collect();
    if !to_add.is_empty() {
        info!("Adding {} new systems to the database.", to_add.len());
        pull_systems(client.clone(), graph.clone(), to_add).await?;
    }

    // 5. Clean up any duplicates that might have crept in
    info!("Checking for and removing any duplicate systems.");
    remove_duplicate_systems(graph.clone()).await?;

    let final_count = get_saved_system_count(&graph).await?;
    info!(
        "System synchronization complete. Total systems: {}",
        final_count
    );

    Ok(())
}

const EXPECTED_ESI_STARGATE_COUNT: i64 = 13776;
async fn synchronize_esi_stargates(
    client: Client,
    graph: Arc<Graph>,
) -> Result<(), ReplicationError> {
    info!("Synchronizing stargates with ESI");
    let mut saved_count = get_saved_stargate_count(&graph).await?;
    let max_attempts = 5;

    for _attempt in 1..=max_attempts {
        match saved_count.cmp(&EXPECTED_ESI_STARGATE_COUNT) {
            Ordering::Less => {
                pull_all_stargates(client.clone(), graph.clone()).await?;
                saved_count = get_saved_stargate_count(&graph).await?;
            }
            Ordering::Equal => {
                info!("Stargates synchronized");
                return Ok(());
            }
            Ordering::Greater => {
                info!("Database has more stargates than expected, removing any duplicates");
                remove_duplicate_stargates(graph.clone()).await?;
                saved_count = get_saved_stargate_count(&graph).await?;
            }
        }
    }

    info!(
        "Failed to synchronize saved stargate count {} to expected count {}",
        saved_count, EXPECTED_ESI_STARGATE_COUNT
    );
    Ok(())
}

async fn pull_systems(
    client: Client,
    graph: Arc<Graph>,
    system_ids: Vec<i64>,
) -> Result<(), ReplicationError> {
    let mut set = JoinSet::new();
    info!("Pulling details for {} systems from ESI", system_ids.len());
    for system_id in system_ids {
        set.spawn(pull_system(client.clone(), graph.clone(), system_id));
    }
    error_if_any_member_has_error(&mut set).await.unwrap()
}
impl From<SystemEsiResponse> for System {
    fn from(s: SystemEsiResponse) -> Self {
        Self {
            constellation_id: s.constellation_id.unwrap_or(-1),
            name: s.name.unwrap_or(String::from("undefined")),
            planets: s
                .planets
                .unwrap_or_default()
                .iter()
                .map(|planet| planet.planet_id)
                .collect(),
            x: s.position.x,
            y: s.position.y,
            z: s.position.z,
            security_class: s.security_class.unwrap_or(String::from("undefined")),
            security_status: s.security_status,
            star_id: s.star_id.unwrap_or(-1),
            stargates: s.stargates.unwrap_or_default(),
            system_id: s.system_id,
            kills: 0,
            jumps: 0,
        }
    }
}

#[derive(Error, Debug)]
enum ReplicationError {
    #[error("failed to retrieve the data")]
    Source(#[from] RequestError),
    #[error("failed to process the data")]
    Process(#[from] JoinError),
    #[error("failed to persist data to the target")]
    Target(#[from] neo4rs::Error),
}

impl Reject for ReplicationError {}

async fn pull_system(
    client: Client,
    graph: Arc<Graph>,
    system_id: i64,
) -> Result<(), ReplicationError> {
    let system_response = get_system_details(&client, system_id).await?;
    let system = System::from(system_response);
    save_system(&graph, &system).await.map_err(Target)
}

impl From<StargateEsiResponse> for Stargate {
    fn from(value: StargateEsiResponse) -> Self {
        Self {
            destination_stargate_id: value.destination.stargate_id,
            destination_system_id: value.destination.system_id,
            name: value.name,
            x: value.position.x,
            y: value.position.y,
            z: value.position.z,
            stargate_id: value.stargate_id,
            system_id: value.system_id,
            type_id: value.type_id,
        }
    }
}

async fn error_if_any_member_has_error<T: 'static>(
    set: &mut JoinSet<Result<(), T>>,
) -> Option<Result<(), T>> {
    while let Some(res) = set.join_next().await {
        if let Err(e) = res.unwrap() {
            return Some(Err(e));
        }
    }
    Some(Ok(()))
}

async fn pull_system_kills(client: Client, graph: Arc<Graph>) -> Result<i32, ReplicationError> {
    let response = get_system_kills(&client).await?;
    let galaxy_kills: i32 = response.system_kills.iter().map(|s| s.ship_kills).sum();

    let mut set = JoinSet::new();

    response.system_kills.iter().for_each(|system_kill| {
        set.spawn(set_last_hour_system_kills(
            graph.clone(),
            system_kill.system_id,
            system_kill.ship_kills,
        ));
    });

    error_if_any_member_has_error(&mut set)
        .await
        .unwrap()
        .map_err(Target)
        .map(|_| galaxy_kills)
}

async fn pull_last_hour_of_jumps(
    client: Client,
    graph: Arc<Graph>,
) -> Result<i32, ReplicationError> {
    let response = get_system_jumps(&client).await?;
    let galaxy_jumps: i32 = response.system_jumps.iter().map(|s| s.ship_jumps).sum();

    let mut set = JoinSet::new();

    response.system_jumps.iter().for_each(|system_jump| {
        set.spawn(set_last_hour_system_jumps(
            graph.clone(),
            system_jump.system_id,
            system_jump.ship_jumps,
        ));
    });

    error_if_any_member_has_error(&mut set)
        .await
        .unwrap()
        .map_err(Target)
        .map(|_| galaxy_jumps)
}

async fn refresh_jump_risks(client: Client, graph: Arc<Graph>) -> Result<(), ReplicationError> {
    info!("Refreshing system jump risks");
    let galaxy_kills = pull_system_kills(client.clone(), graph.clone()).await?;
    let galaxy_jumps = pull_last_hour_of_jumps(client.clone(), graph.clone()).await?;
    let system_ids = get_all_system_ids(graph.clone()).await?;
    let mut set = JoinSet::new();

    system_ids.iter().for_each(|&system_id| {
        set.spawn(set_system_jump_risk(
            graph.clone(),
            system_id,
            galaxy_jumps,
            galaxy_kills,
        ));
    });

    error_if_any_member_has_error(&mut set)
        .await
        .unwrap()
        .map_err(Target)
}

#[instrument(skip(client, graph), fields(stargate_id = %stargate_id))]
async fn pull_stargate(
    client: Client,
    graph: Arc<Graph>,
    stargate_id: i64,
) -> Result<(), ReplicationError> {
    match get_stargate_details(&client, stargate_id).await {
        Ok(response) => {
            let stargate = Stargate::from(response);
            save_stargate(graph.clone(), &stargate)
                .await
                .map_err(Target)
        }
        Err(err) => {
            error!(error = %err, "Failed to pull stargate details. Skipping.");
            Ok(()) // Temporarily allow this to not error so that other stargate pulls can succeed.
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::database::{get_graph_client_with_retry, save_system, System};
    use crate::esi::get_system_details;
    use crate::{pull_all_stargates, pull_last_hour_of_jumps, pull_system_kills};
    use reqwest::Client;
    use tracing::error;

    #[tokio::test]
    #[ignore]
    async fn can_save_system_to_database() {
        let client = Client::new();
        let graph = get_graph_client_with_retry(1).await.unwrap();
        let system_id = 30000201;
        let system_response = get_system_details(&client, system_id).await.unwrap();

        match save_system(&graph, &System::from(system_response)).await {
            Ok(_) => {
                //TODO: Delete the record created
            }
            Err(_) => panic!("Could not save system"),
        }
    }

    #[tokio::test]
    #[ignore]
    async fn should_pull_all_stargates() {
        match pull_all_stargates(Client::new(), get_graph_client_with_retry(1).await.unwrap()).await
        {
            Ok(_) => {}
            Err(_) => {
                error!("failed to pull all stargates")
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn should_pull_system_jumps() {
        let client = Client::new();
        let graph = get_graph_client_with_retry(1).await.unwrap();

        let total_jumps = pull_last_hour_of_jumps(client, graph).await.unwrap();

        assert!(total_jumps > 0)
    }

    #[tokio::test]
    #[ignore]
    async fn should_pull_system_kills() {
        let client = Client::new();
        let graph = get_graph_client_with_retry(1).await.unwrap();

        let total_kills = pull_system_kills(client, graph).await.unwrap();

        assert!(total_kills > 0)
    }
}
