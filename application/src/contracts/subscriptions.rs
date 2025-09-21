use domain::coords::TileCoord;

#[derive(Debug, Clone)]
pub struct SubscriptionRejection {
    pub tile: TileCoord,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct SubscriptionResult {
    pub accepted: Vec<TileCoord>,
    pub rejected: Vec<SubscriptionRejection>,
}
