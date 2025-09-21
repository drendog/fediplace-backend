use std::net::IpAddr;

use crate::{contracts::subscriptions::SubscriptionResult, error::AppResult};
use domain::coords::TileCoord;

#[async_trait::async_trait]
pub trait SubscriptionUseCase: Send + Sync {
    async fn subscribe(&self, ip: IpAddr, tiles: &[TileCoord]) -> AppResult<SubscriptionResult>;

    async fn unsubscribe(&self, ip: IpAddr, tiles: &[TileCoord]) -> AppResult<Vec<TileCoord>>;

    async fn refresh_subscriptions(&self, ip: IpAddr, tiles: &[TileCoord]) -> AppResult<()>;
}
