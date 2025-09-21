use axum::extract::ws::{Message, WebSocket};
use futures::stream::{SplitStream, StreamExt};
use std::{mem, net::IpAddr};
use tokio::{
    sync::{broadcast, mpsc},
    task::JoinHandle,
};
use tracing::{debug, error, info, warn};

use crate::incoming::ws_axum::protocol::WSMessage;
use crate::shared::app_state::AppState;
use domain::{events::TileVersionEvent, tile::TileVersion};

use super::{buffer::BufferedMessageHandler, connection::Connection};

enum MessageSource {
    Direct(broadcast::Receiver<TileVersionEvent>),
    Buffered(mpsc::UnboundedReceiver<WSMessage>, JoinHandle<()>),
}

pub struct ConnectionCounterGuard {
    state: AppState,
}

impl ConnectionCounterGuard {
    pub fn new(state: AppState) -> Self {
        state.increment_websocket_connections();
        Self { state }
    }
}

impl Drop for ConnectionCounterGuard {
    fn drop(&mut self) {
        self.state.decrement_websocket_connections();
    }
}

pub struct ConnectionHandler {
    connection: Connection,
    message_receiver: SplitStream<WebSocket>,
    message_source: MessageSource,
    client_ip: IpAddr,
    _connection_counter_guard: ConnectionCounterGuard,
}

impl ConnectionHandler {
    pub fn new(socket: WebSocket, state: &AppState, client_ip: IpAddr) -> Self {
        let (connection, message_receiver) = Connection::new(socket, client_ip);
        let broadcast_receiver = state.ws_broadcast.subscribe();

        Self {
            connection,
            message_receiver,
            message_source: MessageSource::Direct(broadcast_receiver),
            client_ip,
            _connection_counter_guard: ConnectionCounterGuard::new(state.clone()),
        }
    }

    pub fn new_with_buffering(socket: WebSocket, state: &AppState, client_ip: IpAddr) -> Self {
        let (connection, message_receiver) = Connection::new(socket, client_ip);
        let broadcast_receiver = state.ws_broadcast.subscribe();

        let (outgoing_sender, outgoing_receiver) = mpsc::unbounded_channel();

        let buffer_handler = BufferedMessageHandler::new(
            broadcast_receiver,
            outgoing_sender,
            state.config.websocket.connection_buffer_size,
            state.config.websocket.drop_newest_on_full_buffer,
        );

        let handle = tokio::spawn(async move {
            buffer_handler.run().await;
        });

        Self {
            connection,
            message_receiver,
            message_source: MessageSource::Buffered(outgoing_receiver, handle),
            client_ip,
            _connection_counter_guard: ConnectionCounterGuard::new(state.clone()),
        }
    }

    pub async fn run(mut self, state: AppState) {
        info!(
            "New WebSocket connection established for IP: {}",
            self.client_ip
        );

        let client_ip = self.client_ip;

        let message_source = mem::replace(
            &mut self.message_source,
            MessageSource::Direct(state.ws_broadcast.subscribe()),
        );

        match message_source {
            MessageSource::Direct(broadcast_receiver) => {
                info!("WebSocket connection using direct broadcast mode");
                self.run_direct(broadcast_receiver, state).await;
            }
            MessageSource::Buffered(buffered_receiver, handle) => {
                info!("WebSocket connection using buffered broadcast mode");
                self.run_buffered(buffered_receiver, state).await;
                handle.abort();
            }
        }

        info!("WebSocket connection closed for IP: {}", client_ip);
    }

    async fn run_direct(
        &mut self,
        mut broadcast_receiver: broadcast::Receiver<TileVersionEvent>,
        state: AppState,
    ) {
        loop {
            tokio::select! {
                client_msg = self.message_receiver.next() => {
                    if !self.handle_client_message(client_msg, &state).await {
                        break;
                    }
                }

                broadcast_msg = broadcast_receiver.recv() => {
                    if !self.handle_broadcast_message(broadcast_msg).await {
                        break;
                    }
                }

                () = self.connection.heartbeat_tick() => {
                    if let Err(e) = self.connection.refresh_subscriptions(&state).await {
                        error!("Failed to refresh subscriptions: {}", e);
                        break;
                    }
                }
            }
        }
    }

    async fn run_buffered(
        &mut self,
        mut buffered_receiver: mpsc::UnboundedReceiver<WSMessage>,
        state: AppState,
    ) {
        loop {
            tokio::select! {
                client_msg = self.message_receiver.next() => {
                    if !self.handle_client_message(client_msg, &state).await {
                        break;
                    }
                }

                buffered_msg = buffered_receiver.recv() => {
                    if let Some(msg) = buffered_msg {
                        if !self.handle_buffered_message(msg).await {
                            break;
                        }
                    } else {
                        debug!("Buffered message channel closed");
                        break;
                    }
                }

                () = self.connection.heartbeat_tick() => {
                    if let Err(e) = self.connection.refresh_subscriptions(&state).await {
                        error!("Failed to refresh subscriptions: {}", e);
                        break;
                    }
                }
            }
        }
    }

    async fn handle_client_message(
        &mut self,
        msg_result: Option<Result<Message, axum::Error>>,
        state: &AppState,
    ) -> bool {
        match msg_result {
            Some(Ok(msg)) => {
                if let Err(e) = self.connection.handle_client_message(msg, state).await {
                    error!("Error handling client message: {}", e);
                    return false;
                }
            }
            Some(Err(e)) => {
                warn!("WebSocket error: {}", e);
                return false;
            }
            None => {
                debug!("WebSocket connection closed by client");
                return false;
            }
        }
        true
    }

    async fn handle_buffered_message(&mut self, msg: WSMessage) -> bool {
        debug!("Received buffered message: {:?}", msg);
        if self.connection.should_receive_broadcast(&msg) {
            info!("Sending buffered message to WebSocket client: {:?}", msg);
            if let Err(e) = self.connection.send_ws_message(&msg).await {
                error!("Error sending buffered message: {}", e);
                return false;
            }
        } else {
            debug!("Buffered message not sent");
        }
        true
    }

    async fn handle_broadcast_message(
        &mut self,
        msg_result: Result<TileVersionEvent, broadcast::error::RecvError>,
    ) -> bool {
        use broadcast::error::RecvError;

        match msg_result {
            Ok(event) => {
                debug!("Received broadcast event: {:?}", event);
                let ws_msg =
                    WSMessage::tile_version(event.coord, TileVersion::from_u64(event.version));
                if self.connection.should_receive_broadcast(&ws_msg) {
                    info!("Sending message to WebSocket client: {:?}", ws_msg);
                    if let Err(e) = self.connection.send_ws_message(&ws_msg).await {
                        error!("Error sending broadcast message: {}", e);
                        return false;
                    }
                } else {
                    debug!("Message not sent");
                }
            }
            Err(RecvError::Lagged(skipped)) => {
                warn!(
                    "WebSocket connection lagged behind broadcast channel, skipped {} messages. \
                     Connection will continue receiving new messages.",
                    skipped
                );
            }
            Err(RecvError::Closed) => {
                info!("Broadcast channel closed, terminating WebSocket connection");
                return false;
            }
        }
        true
    }
}
