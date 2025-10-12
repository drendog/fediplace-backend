use std::net::IpAddr;

use crate::{contracts::subscriptions::SubscriptionResult, error::AppResult};
use domain::{coords::TileCoord, world::WorldId};

#[async_trait::async_trait]
pub trait SubscriptionUseCase: Send + Sync {
    async fn subscribe(
        &self,
        ip: IpAddr,
        world_id: &WorldId,
        tiles: &[TileCoord],
    ) -> AppResult<SubscriptionResult>;

    async fn unsubscribe(
        &self,
        ip: IpAddr,
        world_id: &WorldId,
        tiles: &[TileCoord],
    ) -> AppResult<Vec<TileCoord>>;

    async fn refresh_subscriptions(
        &self,
        ip: IpAddr,
        world_id: &WorldId,
        tiles: &[TileCoord],
    ) -> AppResult<()>;
}
