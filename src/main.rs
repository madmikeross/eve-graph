use std::sync::Arc;

use neo4rs::Graph;
use reqwest::Client;

use crate::database::{get_graph_client, save_system, System, system_id_exists};
use crate::esi::{get_system_details, get_system_ids, SystemEsiResponse};

mod database;
mod esi;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>>{
    let client = Client::new();
    let system_ids = get_system_ids(&client).await.unwrap();

    let graph = get_graph_client().await;

    let system_pulls: Vec<_> = system_ids
        .iter()
        .map(|&system_id| tokio::spawn(pull_system_if_missing(client.clone(), graph.clone(), system_id)))
        .collect();

    futures::future::try_join_all(system_pulls).await?;

    Ok(())
}

async fn pull_system_if_missing(
    client: Client,
    graph: Arc<Graph>,
    system_id: i64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !system_id_exists(&graph, system_id).await? {
        pull_system(client, graph, system_id).await?;
    } else {
        println!("System {} already exists in the database.", system_id);
    }

    Ok(())
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

async fn pull_system(
    client: Client,
    graph: Arc<Graph>,
    system_id: i64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match get_system_details(&client, system_id).await {
        Ok(system_response) => {
            let system = System::from(system_response);
            if let Err(err) = save_system(&graph, &system).await {
                println!("Error saving system {}: {}", system.system_id, err);
            } else {
                print!("{:?}, ", system.system_id);
            }
        }
        Err(err) => {
            println!("Error getting system details for system {}: {}", system_id, err);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use reqwest::Client;

    use crate::database::{get_graph_client, save_system, System};
    use crate::esi::{get_stargate, get_system_details};

    #[tokio::test]
    async fn can_save_system_to_database() {
        let client = Client::new();
        let graph = get_graph_client().await;

        let system_id = 30000201;
        let system_response = get_system_details(&client, system_id).await.unwrap();

        match save_system(&graph, &System::from(system_response)).await {
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