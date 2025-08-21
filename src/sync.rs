use std::sync::Arc;

use neo4rs::{Error as Neo4rsError, Graph};
use reqwest::Client;
use thiserror::Error;
use tokio::sync::Semaphore;
use tokio::task::{JoinError, JoinSet};
use tracing::{error, info, instrument};
use warp::reject::Reject;

use crate::database::*;
use crate::esi::{
    get_stargate_details, get_system_details, get_system_ids, get_system_jumps, get_system_kills,
    RequestError, StargateEsiResponse, SystemEsiResponse,
};
use crate::eve_scout::get_public_signatures;
use crate::sync::ReplicationError::Target;

#[derive(Error, Debug)]
pub enum ReplicationError {
    #[error("failed to retrieve the data")]
    Source(#[from] RequestError),
    #[error("failed to process the data")]
    Process(#[from] JoinError),
    #[error("failed to persist data to the target")]
    Target(#[from] Neo4rsError),
}

impl Reject for ReplicationError {}

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

pub async fn refresh_eve_scout_system_relations(
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

async fn pull_stargates(
    client: Client,
    graph: Arc<Graph>,
    stargate_ids: Vec<i64>,
) -> Result<(), ReplicationError> {
    info!(
        "Pulling details for {} stargates from ESI",
        stargate_ids.len()
    );
    let mut set = JoinSet::new();
    let semaphore = Arc::new(Semaphore::new(50));

    for stargate_id in stargate_ids {
        let client = client.clone();
        let graph = graph.clone();
        let semaphore = semaphore.clone();
        set.spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            pull_stargate(client, graph, stargate_id).await
        });
    }

    error_if_any_member_has_error(&mut set).await.unwrap()
}

pub async fn synchronize_esi_systems(
    client: Client,
    graph: Arc<Graph>,
) -> Result<(), ReplicationError> {
    info!("Synchronizing systems with ESI");

    // 1. Get all system IDs from ESI (source of truth)
    let esi_system_ids = get_system_ids(&client).await?;
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

pub async fn synchronize_esi_stargates(
    client: Client,
    graph: Arc<Graph>,
) -> Result<(), ReplicationError> {
    info!("Synchronizing stargates with ESI");

    // 1. Get all stargate IDs from our DB's systems (source of truth for what *should* exist)
    let systems = get_all_systems(graph.clone()).await?;
    let esi_stargate_ids: std::collections::HashSet<i64> =
        systems.into_iter().flat_map(|s| s.stargates).collect();

    // 2. Get all stargate IDs from our DB
    let db_stargate_ids = get_all_stargate_ids(graph.clone()).await?;
    let db_stargate_ids_set: std::collections::HashSet<i64> = db_stargate_ids.into_iter().collect();

    // 3. Find stargates to remove (in DB but not in ESI list)
    let to_remove: Vec<i64> = db_stargate_ids_set
        .difference(&esi_stargate_ids)
        .cloned()
        .collect();
    if !to_remove.is_empty() {
        info!(
            "Removing {} stale stargates from the database.",
            to_remove.len()
        );
        remove_stargates_by_id(graph.clone(), to_remove).await?;
    }

    // 4. Find stargates to add (in ESI list but not in DB)
    let to_add: Vec<i64> = esi_stargate_ids
        .difference(&db_stargate_ids_set)
        .cloned()
        .collect();
    if !to_add.is_empty() {
        info!("Adding {} new stargates to the database.", to_add.len());
        pull_stargates(client.clone(), graph.clone(), to_add).await?;
    }

    // 5. Clean up any duplicates that might have crept in
    info!("Checking for and removing any duplicate stargates.");
    remove_duplicate_stargates(graph.clone()).await?;

    let final_count = get_saved_stargate_count(&graph).await?;
    info!(
        "Stargate synchronization complete. Total stargates: {}",
        final_count
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

async fn pull_system(
    client: Client,
    graph: Arc<Graph>,
    system_id: i64,
) -> Result<(), ReplicationError> {
    let system_response = get_system_details(&client, system_id).await?;
    let system = System::from(system_response);
    save_system(&graph, &system).await.map_err(Target)
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

pub async fn pull_system_kills(client: Client, graph: Arc<Graph>) -> Result<i32, ReplicationError> {
    let system_kills = get_system_kills(&client).await?;
    let galaxy_kills: i32 = system_kills.iter().map(|s| s.ship_kills).sum();

    let mut set = JoinSet::new();

    system_kills.iter().for_each(|system_kill| {
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

pub async fn pull_last_hour_of_jumps(
    client: Client,
    graph: Arc<Graph>,
) -> Result<i32, ReplicationError> {
    let system_jumps = get_system_jumps(&client).await?;
    let galaxy_jumps: i32 = system_jumps.iter().map(|s| s.ship_jumps).sum();

    let mut set = JoinSet::new();

    system_jumps.iter().for_each(|system_jump| {
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

pub async fn refresh_jump_risks(client: Client, graph: Arc<Graph>) -> Result<(), ReplicationError> {
    info!("Refreshing system jump risks");
    let galaxy_kills = pull_system_kills(client.clone(), graph.clone()).await?;
    let galaxy_jumps = pull_last_hour_of_jumps(client.clone(), graph.clone()).await?;
    let system_ids = get_all_system_ids(graph.clone()).await?;
    let mut set = JoinSet::new();

    let baseline_jump_risk = if galaxy_jumps > 0 {
        galaxy_kills as f64 / galaxy_jumps as f64
    } else {
        0.01 // galaxy jumps should never be zero, but just in case
    };

    system_ids.iter().for_each(|&system_id| {
        set.spawn(set_system_jump_risk(
            graph.clone(),
            system_id,
            baseline_jump_risk,
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
        Err(err) => match err {
            RequestError::NotFound { .. } => {
                // This can happen if a stargate was removed from ESI. It's safe to ignore.
                info!(error = %err, "Stargate not found, likely removed from ESI. Skipping.");
                Ok(())
            }
            RequestError::RateLimited { .. } => {
                // This is a critical error. We should stop the entire process.
                // Propagate the error up.
                error!(error = %err, "Rate limited by ESI. Aborting stargate pull.");
                Err(ReplicationError::Source(err))
            }
            _ => {
                // For other errors (server errors, unexpected issues), log it and skip this one.
                error!(error = %err, "Failed to pull stargate details. Skipping.");
                Ok(())
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::esi::{Planet, Position};

    #[test]
    fn test_system_from_esi_response() {
        let esi_response = SystemEsiResponse {
            system_id: 30000142,
            name: Some("Jita".to_string()),
            constellation_id: Some(20000020),
            security_status: 0.9,
            star_id: Some(40000849),
            security_class: Some("A".to_string()),
            position: Position {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            planets: Some(vec![Planet {
                planet_id: 40000855,
                asteroid_belts: None,
                moons: None,
            }]),
            stargates: Some(vec![50000056]),
        };

        let system = System::from(esi_response);

        assert_eq!(system.system_id, 30000142);
        assert_eq!(system.name, "Jita");
        assert_eq!(system.stargates, vec![50000056]);
        assert_eq!(system.planets, vec![40000855]);
        assert_eq!(system.kills, 0); // Default value
    }
}
