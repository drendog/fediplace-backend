use std::{net::IpAddr, sync::Arc};

use crate::{
    contracts::subscriptions::SubscriptionResult,
    error::AppResult,
    ports::{
        incoming::subscriptions::SubscriptionUseCase, outgoing::subscription_port::SubscriptionPort,
    },
};
use domain::coords::TileCoord;

pub struct SubscriptionService {
    subscription_port: Arc<dyn SubscriptionPort>,
}

impl SubscriptionService {
    pub fn new(subscription_port: Arc<dyn SubscriptionPort>) -> Self {
        Self { subscription_port }
    }

    pub async fn subscribe(
        &self,
        ip: IpAddr,
        tiles: &[TileCoord],
    ) -> AppResult<SubscriptionResult> {
        self.subscription_port.subscribe(ip, tiles).await
    }

    pub async fn unsubscribe(&self, ip: IpAddr, tiles: &[TileCoord]) -> AppResult<Vec<TileCoord>> {
        self.subscription_port.unsubscribe(ip, tiles).await
    }

    pub async fn refresh_subscriptions(&self, ip: IpAddr, tiles: &[TileCoord]) -> AppResult<()> {
        self.subscription_port
            .refresh_subscriptions(ip, tiles)
            .await
    }
}

#[async_trait::async_trait]
impl SubscriptionUseCase for SubscriptionService {
    async fn subscribe(&self, ip: IpAddr, tiles: &[TileCoord]) -> AppResult<SubscriptionResult> {
        self.subscription_port.subscribe(ip, tiles).await
    }

    async fn unsubscribe(&self, ip: IpAddr, tiles: &[TileCoord]) -> AppResult<Vec<TileCoord>> {
        self.subscription_port.unsubscribe(ip, tiles).await
    }

    async fn refresh_subscriptions(&self, ip: IpAddr, tiles: &[TileCoord]) -> AppResult<()> {
        self.subscription_port
            .refresh_subscriptions(ip, tiles)
            .await
    }
}
