use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct System {
    pub constellation_id: Option<i64>,
    pub name: Option<String>,
    pub planets: Option<Vec<Planet>>,
    pub position: Position,
    pub security_class: Option<String>,
    pub security_status: f64,
    pub star_id: Option<i64>,
    pub stargates: Option<Vec<i64>>,
    pub system_id: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Planet {
    planet_id: i64,
    asteroid_belts: Option<Vec<i64>>,
    moons: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Stargate {
    pub destination: Destination,
    pub name: String,
    pub position: Position,
    pub stargate_id: i64,
    pub system_id: i64,
    pub type_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Destination {
    stargate_id: i64,
    system_id: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}