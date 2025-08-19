use eve_graph::eve_scout::get_public_signatures;
use reqwest::Client;

#[tokio::test]
async fn should_get_public_signatures() {
    let client = Client::new();
    let signatures = get_public_signatures(client).await.unwrap();

    assert!(!signatures.is_empty());
    println!("{signatures:?}");
}
