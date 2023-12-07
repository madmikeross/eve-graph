use std::sync::Arc;

use neo4rs::Graph;
use reqwest::Client;

use crate::database::{get_graph_client, get_system_details, get_system_ids, save_system, system_id_exists};

mod models;
mod database;

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

async fn pull_system(
    client: Client,
    graph: Arc<Graph>,
    system_id: i64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match get_system_details(&client, system_id).await {
        Ok(system_details) => {
            if let Err(err) = save_system(&graph, &system_details).await {
                println!("Error saving system {}: {}", system_details.system_id, err);
            } else {
                print!("{:?}, ", system_details.system_id);
            }
        }
        Err(err) => {
            println!("Error getting system details for system {}: {}", system_id, err);
        }
    }

    Ok(())
}

