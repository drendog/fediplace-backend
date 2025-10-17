use axum::extract::ws::{Message, WebSocket};
use futures::{
    sink::SinkExt,
    stream::{SplitSink, SplitStream, StreamExt},
};
use std::{
    future,
    net::IpAddr,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use tokio::time::{Interval, interval};
use tracing::{debug, error, info, warn};

use crate::incoming::ws_axum::WsAdapterPolicy;
use crate::incoming::ws_axum::protocol::{ClientMessage, RejectedTile, WSMessage};
use crate::shared::app_state::AppState;
use domain::coords::TileCoord;
use domain::world::WorldId;
use fedi_wplace_application::{
    contracts::subscriptions::SubscriptionResult, error::AppError,
    ports::incoming::tiles::TilesQueryUseCase,
};

use super::subscriptions::SubscriptionManager;

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] axum::Error),

    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Application error: {0}")]
    Application(#[from] AppError),

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

pub type ConnectionResult<T> = Result<T, ConnectionError>;

pub struct Connection {
    socket_sender: SplitSink<WebSocket, Message>,
    subscriptions: SubscriptionManager,
    client_ip: IpAddr,
    heartbeat_interval: Option<Interval>,
    world_id: WorldId,
}

impl Connection {
    pub fn new(socket: WebSocket, client_ip: IpAddr, world_id: WorldId) -> (Self, SplitStream<WebSocket>) {
        let (sender, receiver) = socket.split();
        let connection = Self {
            socket_sender: sender,
            subscriptions: SubscriptionManager::new(),
            client_ip,
            heartbeat_interval: None,
            world_id,
        };
        (connection, receiver)
    }

    pub fn start_heartbeat(&mut self, policy: &WsAdapterPolicy) {
        if self.heartbeat_interval.is_some() {
            return;
        }

        let interval_duration = Duration::from_millis(policy.heartbeat_refresh_secs * 1000);
        self.heartbeat_interval = Some(interval(interval_duration));
    }

    pub async fn handle_client_message(
        &mut self,
        msg: Message,
        state: &AppState,
    ) -> ConnectionResult<()> {
        if let Some(ref rate_limiter) = state.websocket_rate_limiter {
            match rate_limiter.check_rate_limit(self.client_ip) {
                super::super::http_axum::middleware::rate_limit::RateLimitResult::Allowed(_) => {}
                super::super::http_axum::middleware::rate_limit::RateLimitResult::Denied(
                    rate_info,
                ) => {
                    warn!("WebSocket rate limit exceeded for IP: {}", self.client_ip);

                    let now_instant = Instant::now();
                    let now_system = SystemTime::now();
                    let time_until_reset =
                        rate_info.reset_time.saturating_duration_since(now_instant);
                    let reset_system_time = now_system + time_until_reset;
                    let reset_timestamp = reset_system_time
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or(Duration::from_secs(0))
                        .as_secs();

                    let error_msg = WSMessage::error(format!(
                        "Rate limit exceeded. Limit: {}, Reset at: {}, Retry after: {}s",
                        rate_info.limit,
                        reset_timestamp,
                        rate_info.retry_after_seconds.unwrap_or(60)
                    ));
                    self.send_ws_message(&error_msg).await?;
                    return Err(ConnectionError::RateLimited);
                }
            }
        }

        match msg {
            Message::Text(text) => {
                self.handle_text_message(text.to_string(), state).await?;
            }
            Message::Ping(data) => {
                debug!("Received ping, sending pong");
                self.socket_sender.send(Message::Pong(data)).await?;
            }
            Message::Pong(_) => {
                debug!("Received pong");
            }
            Message::Close(_) => {
                debug!("Received close message");
                self.cleanup_subscriptions(state).await;
            }
            Message::Binary(_) => {
                warn!("Received unexpected binary message");
                let error_msg = WSMessage::error("Binary messages not supported".to_string());
                self.send_ws_message(&error_msg).await?;
            }
        }
        Ok(())
    }

    async fn handle_text_message(
        &mut self,
        text: String,
        state: &AppState,
    ) -> ConnectionResult<()> {
        debug!("Received client message: {}", text);

        match serde_json::from_str::<ClientMessage>(&text) {
            Ok(client_msg) => {
                self.handle_parsed_message(client_msg, state).await?;
            }
            Err(e) => {
                warn!("Invalid client message format: {}", e);
                let error_msg = WSMessage::error("Invalid message format".to_string());
                self.send_ws_message(&error_msg).await?;
                return Err(ConnectionError::InvalidFormat(e.to_string()));
            }
        }
        Ok(())
    }

    async fn handle_parsed_message(
        &mut self,
        client_msg: ClientMessage,
        state: &AppState,
    ) -> ConnectionResult<()> {
        match client_msg {
            ClientMessage::Subscribe { tiles } => {
                self.process_subscription_request_with_fifo_eviction(tiles, state)
                    .await?;
            }
            ClientMessage::Unsubscribe { tiles } => {
                self.handle_unsubscription(tiles, state).await?;
            }
            ClientMessage::Ping => {
                debug!("Received application-level ping");
                self.socket_sender
                    .send(Message::Pong(vec![].into()))
                    .await?;
            }
        }
        Ok(())
    }

    async fn process_subscription_request_with_fifo_eviction(
        &mut self,
        requested_tile_coordinates: Vec<TileCoord>,
        app_state: &AppState,
    ) -> ConnectionResult<()> {
        if self.heartbeat_interval.is_none() {
            self.start_heartbeat(&app_state.ws_policy);
        }

        match app_state
            .subscription_service
            .subscribe(self.client_ip, &self.world_id, &requested_tile_coordinates)
            .await
        {
            Ok(subscription_result) => {
                self.handle_subscription_result(subscription_result, app_state)
                    .await?;
            }
            Err(e) => {
                let error_msg = WSMessage::error("Server temporarily unavailable".to_string());
                self.send_ws_message(&error_msg).await?;
                error!(
                    "Subscription service error for IP {}: {}",
                    self.client_ip, e
                );
                return Err(ConnectionError::Application(e));
            }
        }

        Ok(())
    }

    async fn handle_subscription_result(
        &mut self,
        subscription_result: SubscriptionResult,
        app_state: &AppState,
    ) -> ConnectionResult<()> {
        if !subscription_result.accepted.is_empty() {
            let added_tiles = self
                .subscriptions
                .add_tiles(subscription_result.accepted.clone());
            debug!(
                "Added {} tiles to local subscription tracking",
                added_tiles.len()
            );
        }

        self.build_and_send_subscription_acknowledgment(&subscription_result, app_state)
            .await?;

        if !subscription_result.accepted.is_empty() {
            self.send_current_versions_for_newly_subscribed_tiles(
                &subscription_result.accepted,
                app_state,
            )
            .await?;
            info!(
                "Client {} subscribed to {} tiles",
                self.client_ip,
                subscription_result.accepted.len()
            );
        }

        Ok(())
    }

    async fn build_and_send_subscription_acknowledgment(
        &mut self,
        subscription_result: &SubscriptionResult,
        app_state: &AppState,
    ) -> ConnectionResult<()> {
        let max_tiles = app_state.ws_policy.max_tiles_per_ip;
        let current_count = self.subscriptions.get_subscribed_tiles().len();
        let remaining_budget = max_tiles.saturating_sub(current_count) as u32;

        let rejected_tiles: Vec<RejectedTile> = subscription_result
            .rejected
            .iter()
            .map(|tile| tile.clone().into())
            .collect();

        let subscription_acknowledgment = WSMessage::subscribe_ack(
            subscription_result.accepted.clone(),
            rejected_tiles,
            remaining_budget,
        );

        self.send_ws_message(&subscription_acknowledgment).await?;
        Ok(())
    }

    async fn send_current_versions_for_newly_subscribed_tiles(
        &mut self,
        newly_subscribed_tiles: &[TileCoord],
        app_state: &AppState,
    ) -> ConnectionResult<()> {
        let tile_query_uc: &dyn TilesQueryUseCase = &*app_state.tiles_query_service;
        for tile_coordinate in newly_subscribed_tiles {
            match tile_query_uc
                .get_tile_version(&self.world_id, *tile_coordinate)
                .await
            {
                Ok(current_version) => {
                    let version_message =
                        WSMessage::tile_version(*tile_coordinate, current_version);
                    if let Err(send_error) = self.send_ws_message(&version_message).await {
                        error!(
                            "Failed to send tile version for {}: {}",
                            tile_coordinate, send_error
                        );
                    }
                }
                Err(version_lookup_error) => {
                    error!(
                        "Failed to get tile version for {}: {}",
                        tile_coordinate, version_lookup_error
                    );
                }
            }
        }
        Ok(())
    }

    async fn handle_unsubscription(
        &mut self,
        tiles: Vec<TileCoord>,
        state: &AppState,
    ) -> ConnectionResult<()> {
        match state
            .subscription_service
            .unsubscribe(self.client_ip, &self.world_id, &tiles)
            .await
        {
            Ok(_redis_removed_tiles) => {
                let local_removed = self.subscriptions.remove_tiles(tiles.clone());

                if !local_removed.is_empty() {
                    let confirmation = WSMessage::unsubscription_confirmed(local_removed.clone());
                    self.send_ws_message(&confirmation).await?;

                    info!(
                        "Client {} unsubscribed from {} tiles",
                        self.client_ip,
                        local_removed.len()
                    );
                }
            }
            Err(e) => {
                error!("Failed to unsubscribe for IP {}: {}", self.client_ip, e);
                let error_msg = WSMessage::error("Failed to unsubscribe".to_string());
                self.send_ws_message(&error_msg).await?;
                return Err(ConnectionError::Application(e));
            }
        }

        Ok(())
    }

    pub async fn heartbeat_tick(&mut self) {
        if let Some(ref mut interval) = self.heartbeat_interval {
            interval.tick().await;
        } else {
            future::pending::<()>().await;
        }
    }

    pub async fn refresh_subscriptions(&self, state: &AppState) -> ConnectionResult<()> {
        if self.subscriptions.get_subscribed_tiles().is_empty() {
            return Ok(());
        }

        let tiles: Vec<TileCoord> = self
            .subscriptions
            .get_subscribed_tiles()
            .iter()
            .copied()
            .collect();

        match state
            .subscription_service
            .refresh_subscriptions(self.client_ip, &self.world_id, &tiles)
            .await
        {
            Ok(()) => {
                debug!(
                    "Refreshed {} subscriptions for IP {}",
                    tiles.len(),
                    self.client_ip
                );
            }
            Err(e) => {
                warn!(
                    "Failed to refresh subscriptions for IP {}: {}",
                    self.client_ip, e
                );
            }
        }

        Ok(())
    }

    async fn cleanup_subscriptions(&mut self, state: &AppState) {
        let subscribed_tiles: Vec<TileCoord> = self
            .subscriptions
            .get_subscribed_tiles()
            .iter()
            .copied()
            .collect();
        if subscribed_tiles.is_empty() {
            return;
        }

        if let Err(e) = state
            .subscription_service
            .unsubscribe(self.client_ip, &self.world_id, &subscribed_tiles)
            .await
        {
            warn!(
                "Failed to cleanup subscriptions for IP {}: {}",
                self.client_ip, e
            );
        }

        self.subscriptions.remove_tiles(subscribed_tiles);
        info!("Cleaned up subscriptions for IP {}", self.client_ip);
    }

    pub fn should_receive_broadcast(&self, msg: &WSMessage) -> bool {
        match msg {
            WSMessage::TileVersion { x, y, .. } => {
                let tc = TileCoord::new(*x, *y);
                self.subscriptions.is_subscribed_to(tc)
            }
            WSMessage::Error { .. }
            | WSMessage::SubscriptionConfirmed { .. }
            | WSMessage::SubscribeAck { .. }
            | WSMessage::UnsubscriptionConfirmed { .. } => false,
        }
    }

    pub async fn send_ws_message(&mut self, msg: &WSMessage) -> ConnectionResult<()> {
        let json = serde_json::to_string(msg)?;
        self.socket_sender.send(Message::Text(json.into())).await?;
        Ok(())
    }
}
