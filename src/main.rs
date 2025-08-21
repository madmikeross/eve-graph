use std::convert::Infallible;
use std::sync::Arc;

use eve_graph::database::*;
use eve_graph::sync;
use eve_graph::sync::{
    refresh_eve_scout_system_relations, refresh_jump_risks, synchronize_esi_stargates,
    synchronize_esi_systems,
};
use neo4rs::Graph;
use reqwest::Client;
use tracing::{error, info};
use warp::hyper::StatusCode;
use warp::reject::Reject;
use warp::{reply, Filter, Rejection, Reply};

#[tokio::main]
async fn main() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,neo4rs=warn"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    info!("Starting eve-graph");
    let client = Client::new();
    let graph = get_graph_client_with_retry(10, None).await.unwrap();

    // Bootstrap application data. If this fails, we log the error and exit.
    if let Err(e) = bootstrap(client.clone(), graph.clone()).await {
        error!(
            "Failed to bootstrap application data: {}. Shutting down.",
            e
        );
        return;
    }

    // --- Define API Routes ---
    let shortest_route = warp::path!("shortest-route" / String / "to" / String)
        .and(warp::get())
        .and(with_graph(graph.clone()))
        .and_then(shortest_route_to_handler);

    let safest_route = warp::path!("safest-route" / String / "to" / String)
        .and(warp::get())
        .and(with_graph(graph.clone()))
        .and_then(safest_route_to_handler);

    let systems_refresh = warp::path!("systems" / "refresh")
        .and(warp::post())
        .and(with_client(client.clone()))
        .and(with_graph(graph.clone()))
        .and_then(systems_refresh_handler);

    let systems_risk = warp::path!("systems" / "risk")
        .and(warp::post())
        .and(with_client(client.clone()))
        .and(with_graph(graph.clone()))
        .and_then(systems_risk_handler);

    let stargates_refresh = warp::path!("stargates" / "refresh")
        .and(warp::post())
        .and(with_client(client.clone()))
        .and(with_graph(graph.clone()))
        .and_then(stargates_refresh_handler);

    let wormholes_refresh = warp::path!("wormholes" / "refresh")
        .and(warp::post())
        .and(with_client(client.clone()))
        .and(with_graph(graph.clone()))
        .and_then(wormholes_refresh_handler);

    let routes = shortest_route
        .or(safest_route)
        .or(wormholes_refresh)
        .or(systems_refresh)
        .or(systems_risk)
        .or(stargates_refresh)
        .recover(handle_rejection);

    info!("Serving routes on 8008");
    warp::serve(routes).run(([0, 0, 0, 0], 8008)).await;
}

/// Runs the initial data synchronization tasks required for the application to function.
async fn bootstrap(client: Client, graph: Arc<Graph>) -> Result<(), sync::Error> {
    info!("Bootstrapping application data...");

    synchronize_esi_systems(client.clone(), graph.clone()).await?;
    info!("System synchronization complete.");

    synchronize_esi_stargates(client.clone(), graph.clone()).await?;
    info!("Stargate synchronization complete.");

    refresh_jump_risks(client.clone(), graph.clone()).await?;
    info!("Jump risk calculation complete.");

    refresh_jump_risk_graph(graph.clone()).await?;
    info!("Jump risk graph projection complete.");

    refresh_eve_scout_system_relations(client, graph.clone()).await?;
    info!("EVE Scout data refreshed.");

    refresh_jump_cost_graph(graph).await?;
    info!("Jump cost graph projection complete.");

    info!("Bootstrap complete.");
    Ok(())
}

fn with_client(client: Client) -> impl Filter<Extract = (Client,), Error = Infallible> + Clone {
    warp::any().map(move || client.clone())
}

fn with_graph(
    graph: Arc<Graph>,
) -> impl Filter<Extract = (Arc<Graph>,), Error = Infallible> + Clone {
    warp::any().map(move || graph.clone())
}

/// A newtype wrapper for our library's error to implement `warp::reject::Reject`.
#[derive(Debug)]
struct ApiError(sync::Error);

impl Reject for ApiError {}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    if err.is_not_found() {
        return Ok(reply::with_status("NOT_FOUND", StatusCode::NOT_FOUND));
    }

    if let Some(api_error) = err.find::<ApiError>() {
        error!("API Error: {:?}", api_error.0);
        return Ok(reply::with_status(
            "INTERNAL_SERVER_ERROR",
            StatusCode::INTERNAL_SERVER_ERROR,
        ));
    }

    if let Some(db_error) = err.find::<neo4rs::Error>() {
        error!("Database Error: {:?}", db_error);
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
        .map_err(|e| warp::reject::custom(ApiError(e.into())))?
    {
        None => {
            // Return a 404 Not Found, which is more idiomatic for a missing resource (the path).
            let mut res = warp::reply::json(&serde_json::json!({ "error": "route not found" }))
                .into_response();
            *res.status_mut() = StatusCode::NOT_FOUND;
            Ok(res)
        }
        Some(route) => Ok(warp::reply::json(&route).into_response()),
    }
}

async fn safest_route_to_handler(
    from_system_name: String,
    to_system_name: String,
    graph: Arc<Graph>,
) -> Result<impl Reply, Rejection> {
    let exists = graph_exists(&graph, String::from("jump-risk"))
        .await
        .map_err(|e| warp::reject::custom(ApiError(e.into())))?;
    if !exists {
        build_jump_risk_graph(graph.clone())
            .await
            .map_err(|e| warp::reject::custom(ApiError(e.into())))?;
    }

    match find_safest_route(graph, from_system_name, to_system_name)
        .await
        .map_err(|e| warp::reject::custom(ApiError(e.into())))?
    {
        None => {
            let mut res = warp::reply::json(&serde_json::json!({ "error": "route not found" }))
                .into_response();
            *res.status_mut() = StatusCode::NOT_FOUND;
            Ok(res)
        }
        Some(route) => Ok(warp::reply::json(&route).into_response()),
    }
}

async fn wormholes_refresh_handler(
    client: Client,
    graph: Arc<Graph>,
) -> Result<impl Reply, Rejection> {
    refresh_eve_scout_system_relations(client, graph.clone())
        .await
        .map_err(|e| warp::reject::custom(ApiError(e)))?;
    refresh_jump_cost_graph(graph)
        .await
        .map_err(|e| warp::reject::custom(ApiError(e.into())))?;
    Ok(reply())
}

async fn systems_risk_handler(client: Client, graph: Arc<Graph>) -> Result<impl Reply, Rejection> {
    refresh_jump_risks(client, graph.clone())
        .await
        .map_err(|e| warp::reject::custom(ApiError(e)))?;
    refresh_jump_risk_graph(graph)
        .await
        .map_err(|e| warp::reject::custom(ApiError(e.into())))?;
    Ok(reply())
}

async fn systems_refresh_handler(
    client: Client,
    graph: Arc<Graph>,
) -> Result<impl Reply, Rejection> {
    synchronize_esi_systems(client, graph)
        .await
        .map_err(|e| warp::reject::custom(ApiError(e)))?;
    Ok(reply())
}

async fn stargates_refresh_handler(
    client: Client,
    graph: Arc<Graph>,
) -> Result<impl Reply, Rejection> {
    synchronize_esi_stargates(client, graph.clone())
        .await
        .map_err(|e| warp::reject::custom(ApiError(e)))?;
    refresh_jump_cost_graph(graph)
        .await
        .map_err(|e| warp::reject::custom(ApiError(e.into())))?;
    Ok(reply())
}
