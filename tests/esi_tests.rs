use reqwest::Client;

use eve_graph::esi::{get_stargate_details, get_system_jumps, get_system_kills};

#[tokio::test]
async fn should_get_system_kills() {
    let client = Client::new();
    let system_kills_response = get_system_kills(&client).await.unwrap();

    assert!(&system_kills_response.system_kills.len() > &0);
}

#[tokio::test]
async fn should_get_system_jumps() {
    let client = Client::new();
    let system_jumps_response = get_system_jumps(&client).await.unwrap();

    assert!(&system_jumps_response.system_jumps.len() > &0);
}

#[tokio::test]
async fn should_get_stargate_details() {
    let client = Client::new();
    let stargate_id = 50011905;

    let stargate = get_stargate_details(&client, stargate_id).await.unwrap();

    assert_eq!(stargate.stargate_id, stargate_id);
    assert_eq!(stargate.name, "Stargate (Vouskiaho)");
}
