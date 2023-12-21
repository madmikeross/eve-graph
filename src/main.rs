use std::convert::Infallible;
use std::sync::Arc;

use neo4rs::{Error, Graph};
use reqwest::Client;
use thiserror::Error;
use tokio::task::{JoinError, JoinSet};
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
use crate::ReplicationError::TargetError;

mod database;
mod esi;
mod eve_scout;

#[tokio::main]
async fn main() {
    println!("Starting eve-graph");
    let client = Client::new();
    let graph = get_graph_client_with_retry(5).await.unwrap();

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

    let routes_to = warp::path!("routes" / String / "to" / String);
    let routes_routes = routes_to
        .and(warp::get())
        .and(with_graph(graph.clone()))
        .and_then(routes_to_handler);

    let service_routes = routes_routes
        .or(wormholes_routes)
        .or(systems_routes)
        .or(stargates_routes)
        .recover(handle_rejection);

    println!("Serving routes on 8008");
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

    if let Some(_) = err.find::<ReplicationError>() {
        return Ok(reply::with_status(
            "INTERNAL_SERVER_ERROR",
            StatusCode::INTERNAL_SERVER_ERROR,
        ));
    }

    if let Some(_) = err.find::<neo4rs::Error>() {
        return Ok(reply::with_status(
            "INTERNAL_SERVER_ERROR",
            StatusCode::INTERNAL_SERVER_ERROR,
        ));
    }

    eprintln!("unhandled rejection: {:?}", err);
    Ok(reply::with_status(
        "INTERNAL_SERVER_ERROR",
        StatusCode::INTERNAL_SERVER_ERROR,
    ))
}

async fn routes_to_handler(
    from_system_name: String,
    to_system_name: String,
    graph: Arc<Graph>,
) -> Result<impl Reply, Rejection> {
    let route = find_shortest_route(graph, from_system_name, to_system_name)
        .await
        .unwrap()
        .unwrap();
    Ok(json::<Vec<_>>(&route))
}

async fn wormholes_refresh_handler(
    client: Client,
    graph: Arc<Graph>,
) -> Result<impl Reply, Rejection> {
    refresh_eve_scout_system_relations(client, graph.clone())
        .await
        .unwrap();
    rebuild_jump_cost_graph(graph).await.unwrap();
    Ok(reply())
}

async fn systems_risk_handler(client: Client, graph: Arc<Graph>) -> Result<impl Reply, Rejection> {
    pull_system_risk(client, graph.clone()).await.unwrap();
    rebuild_jump_risk_graph(graph).await.unwrap();
    Ok(reply())
}

async fn systems_refresh_handler(
    client: Client,
    graph: Arc<Graph>,
) -> Result<impl Reply, Rejection> {
    pull_all_systems(client, graph).await?;
    Ok(reply())
}

async fn stargates_refresh_handler(
    client: Client,
    graph: Arc<Graph>,
) -> Result<impl Reply, Rejection> {
    pull_all_stargates(client, graph).await?;
    Ok(reply())
}

async fn refresh_eve_scout_system_relations(
    client: Client,
    graph: Arc<Graph>,
) -> Result<(), ReplicationError> {
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
        .map_err(TargetError)
}

async fn pull_all_stargates(client: Client, graph: Arc<Graph>) -> Result<(), ReplicationError> {
    let mut set = JoinSet::new();

    get_all_system_ids(graph.clone())
        .await?
        .iter()
        .for_each(|&system_id| {
            set.spawn(pull_system_stargates(
                client.clone(),
                graph.clone(),
                system_id,
            ));
        });

    error_if_any_member_has_error(&mut set).await.unwrap()
}

async fn pull_all_systems(client: Client, graph: Arc<Graph>) -> Result<(), ReplicationError> {
    let mut set = JoinSet::new();

    get_system_ids(&client)
        .await
        .unwrap()
        .iter()
        .for_each(|&system_id| {
            println!("Spawning task to pull {} system if its missing", system_id);
            set.spawn(pull_system_if_missing(
                client.clone(),
                graph.clone(),
                system_id,
            ));
        });

    error_if_any_member_has_error(&mut set).await.unwrap()
}

async fn pull_system_if_missing(
    client: Client,
    graph: Arc<Graph>,
    system_id: i64,
) -> Result<(), ReplicationError> {
    println!("Checking if system_id {} exists in the database", system_id);

    match system_id_exists(graph.clone(), system_id).await {
        Ok(exists) => {
            if !exists {
                println!(
                    "System {} does not already exist in the database",
                    system_id
                );
                pull_system(client, graph.clone(), system_id).await?;
            }
            Ok(())
        }
        Err(_) => {
            println!("Error checking if system_id {} exists", system_id);
            Err(TargetError(Error::ConnectionError))
        }
    }
}

impl From<SystemEsiResponse> for System {
    fn from(s: SystemEsiResponse) -> Self {
        Self {
            constellation_id: s.constellation_id.unwrap_or(-1),
            name: s.name.unwrap_or(String::from("undefined")),
            planets: s
                .planets
                .unwrap_or(Vec::new())
                .iter()
                .map(|planet| planet.planet_id)
                .collect(),
            x: s.position.x,
            y: s.position.y,
            z: s.position.z,
            security_class: s.security_class.unwrap_or(String::from("undefined")),
            security_status: s.security_status,
            star_id: s.star_id.unwrap_or(-1),
            stargates: s.stargates.unwrap_or(Vec::new()),
            system_id: s.system_id,
            kills: 0,
            jumps: 0,
        }
    }
}

#[derive(Error, Debug)]
enum ReplicationError {
    #[error("failed to retrieve the data")]
    SourceError(#[from] RequestError),
    #[error("failed to process the data")]
    ProcessError(#[from] JoinError),
    #[error("failed to persist data to the target")]
    TargetError(#[from] neo4rs::Error),
}

impl Reject for ReplicationError {}

async fn pull_system(
    client: Client,
    graph: Arc<Graph>,
    system_id: i64,
) -> Result<(), ReplicationError> {
    println!("Getting system {} from ESI", system_id);
    let system_response = get_system_details(&client, system_id).await?;
    let system = System::from(system_response);
    println!("Saving system {} to database", system_id);
    save_system(&graph, &system).await.map_err(TargetError)
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

async fn pull_system_stargates(
    client: Client,
    graph: Arc<Graph>,
    system_id: i64,
) -> Result<(), ReplicationError> {
    let mut set = JoinSet::new();

    get_system(graph.clone(), system_id)
        .await?
        .unwrap()
        .stargates
        .iter()
        .for_each(|&stargate_id| {
            set.spawn(pull_stargate_if_missing(
                client.clone(),
                graph.clone(),
                stargate_id,
            ));
        });

    error_if_any_member_has_error(&mut set).await.unwrap()
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
        .map_err(TargetError)
        .map(|_| galaxy_kills)
}

async fn pull_system_jumps(client: Client, graph: Arc<Graph>) -> Result<i32, ReplicationError> {
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
        .map_err(TargetError)
        .map(|_| galaxy_jumps)
}

async fn pull_system_risk(client: Client, graph: Arc<Graph>) -> Result<(), ReplicationError> {
    let galaxy_kills = pull_system_kills(client.clone(), graph.clone()).await?;
    let galaxy_jumps = pull_system_jumps(client.clone(), graph.clone()).await?;
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
        .map_err(TargetError)
}

async fn pull_stargate_if_missing(
    client: Client,
    graph: Arc<Graph>,
    stargate_id: i64,
) -> Result<(), ReplicationError> {
    if !stargate_id_exists(graph.clone(), stargate_id).await? {
        pull_stargate(client, graph, stargate_id).await?;
    }
    Ok(())
}

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
                .map_err(TargetError)
        }
        Err(_) => {
            println!("Failed to pull stargate {}", stargate_id);
            Ok(()) // Temporarily allow this to not error so that other stargate pulls can succeed.
        }
    }
}

#[cfg(test)]
mod tests {
    use reqwest::Client;

    use crate::database::{get_graph_client_with_retry, save_system, System};
    use crate::esi::get_system_details;
    use crate::{pull_all_stargates, pull_system_jumps, pull_system_kills, pull_system_stargates};

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
                println!("failed to pull all stargates")
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn should_pull_system_stargates() {
        let client = Client::new();
        let graph = get_graph_client_with_retry(1).await.unwrap();
        let system_id = 30000193;

        match pull_system_stargates(client.clone(), graph.clone(), system_id).await {
            Ok(_) => {}
            Err(_) => {
                println!("Failed to pull system stargates");
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn should_pull_system_jumps() {
        let client = Client::new();
        let graph = get_graph_client_with_retry(1).await.unwrap();

        let total_jumps = pull_system_jumps(client, graph).await.unwrap();

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
