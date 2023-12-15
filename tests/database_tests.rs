use eve_graph::database::{
    build_jump_risk_graph, build_system_jump_graph, drop_jump_risk_graph, drop_system_jump_graph,
    get_all_system_ids, get_graph_client, get_system,
};

#[tokio::test]
#[ignore]
async fn should_get_system() {
    let system_id = 30000276;
    let system = get_system(get_graph_client().await, system_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(system.system_id, system_id)
}

#[tokio::test]
#[ignore]
async fn should_get_all_system_ids() {
    let system_ids = get_all_system_ids(get_graph_client().await).await;

    assert_eq!(system_ids.unwrap().len(), 8436)
}

#[tokio::test]
#[ignore]
async fn should_drop_system_jump_graph() {
    let graph = get_graph_client().await;
    let dropped_graph_name = drop_system_jump_graph(&graph).await.unwrap();
    assert_eq!(dropped_graph_name, "system-map")
}

#[tokio::test]
#[ignore]
async fn should_create_system_jump_graph() {
    let graph = get_graph_client().await;
    let new_graph_name = build_system_jump_graph(graph).await.unwrap();
    assert_eq!(new_graph_name, "system-map")
}

#[tokio::test]
#[ignore]
async fn should_drop_jump_risk_graph() {
    let graph = get_graph_client().await;
    let dropped_graph_name = drop_jump_risk_graph(graph).await.unwrap();
    assert_eq!(dropped_graph_name, "jump-risk")
}

#[tokio::test]
#[ignore]
async fn should_create_jump_risk_graph() {
    let graph = get_graph_client().await;
    let new_graph_name = build_jump_risk_graph(graph).await.unwrap();
    assert_eq!(new_graph_name, "jump-risk")
}
