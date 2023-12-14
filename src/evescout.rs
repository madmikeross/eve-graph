use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct EveScoutSignature {
    id: String,
    created_at: String,
    created_by_id: i64,
    created_by_name: String,
    updated_at: String,
    updated_by_id: i64,
    updated_by_name: String,
    completed_at: String,
    completed_by_id: i64,
    completed_by_name: String,
    completed: bool,
    wh_exits_outward: bool,
    wh_type: String,
    max_ship_size: String,
    expires_at: String,
    remaining_hours: i64,
    pub signature_type: String,
    pub out_system_id: i64,
    out_system_name: String,
    out_signature: String,
    pub in_system_id: i64,
    in_system_class: String,
    in_system_name: String,
    in_region_id: i64,
    in_region_name: String,
    in_signature: String,
    comment: Option<String>,
}

pub(crate) async fn get_public_signatures(
    client: Client,
) -> Result<Vec<EveScoutSignature>, reqwest::Error> {
    let get_public_signatures = format!("https://api.eve-scout.com/v2/public/signatures");
    let response = client.get(&get_public_signatures).send().await?;
    response.json().await
}

#[cfg(test)]
mod tests {
    use crate::evescout::get_public_signatures;
    use reqwest::Client;

    #[tokio::test]
    async fn should_get_public_signatures() {
        let client = Client::new();
        let signatures = get_public_signatures(client).await.unwrap();

        assert!(signatures.len() > 0);
        println!("{:?}", signatures);
    }
}
