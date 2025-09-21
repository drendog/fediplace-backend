use std::collections::VecDeque;
use tokio::sync::{
    broadcast::{self, error::RecvError},
    mpsc,
};
use tracing::{debug, warn};

use crate::incoming::ws_axum::protocol::WSMessage;
use domain::{events::TileVersionEvent, tile::TileVersion};

pub struct ConnectionBuffer {
    buffer: VecDeque<WSMessage>,
    max_size: usize,
    drop_newest_on_full: bool,
    dropped_count: u64,
}

impl ConnectionBuffer {
    pub fn new(max_size: usize, drop_newest_on_full: bool) -> Self {
        Self {
            buffer: VecDeque::with_capacity(max_size),
            max_size,
            drop_newest_on_full,
            dropped_count: 0,
        }
    }

    pub fn push_with_drop_policy(&mut self, message: WSMessage) -> bool {
        if self.buffer.len() >= self.max_size {
            if self.drop_newest_on_full {
                self.dropped_count += 1;
                warn!(
                    "Connection buffer full ({}/{}), dropping newest message. Total dropped: {}",
                    self.buffer.len(),
                    self.max_size,
                    self.dropped_count
                );
                return false;
            } else if let Some(dropped) = self.buffer.pop_front() {
                self.dropped_count += 1;
                debug!(
                    "Connection buffer full, dropping oldest message: {:?}. Total dropped: {}",
                    dropped, self.dropped_count
                );
            }
        }

        self.buffer.push_back(message);
        true
    }

    pub fn pop(&mut self) -> Option<WSMessage> {
        self.buffer.pop_front()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn dropped_count(&self) -> u64 {
        self.dropped_count
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn utilization(&self) -> f64 {
        (self.len() as f64 / self.max_size as f64) * 100.0
    }
}

pub struct BufferedMessageHandler {
    broadcast_receiver: broadcast::Receiver<TileVersionEvent>,
    outgoing_sender: mpsc::UnboundedSender<WSMessage>,
    buffer: ConnectionBuffer,
}

impl BufferedMessageHandler {
    pub fn new(
        broadcast_receiver: broadcast::Receiver<TileVersionEvent>,
        outgoing_sender: mpsc::UnboundedSender<WSMessage>,
        buffer_size: usize,
        drop_newest_on_full: bool,
    ) -> Self {
        Self {
            broadcast_receiver,
            outgoing_sender,
            buffer: ConnectionBuffer::new(buffer_size, drop_newest_on_full),
        }
    }

    pub async fn run(mut self) {
        loop {
            match self.broadcast_receiver.recv().await {
                Ok(event) => {
                    let ws_message =
                        WSMessage::tile_version(event.coord, TileVersion::from_u64(event.version));
                    if self.buffer.push_with_drop_policy(ws_message) && !self.flush_buffer() {
                        debug!("Outgoing channel closed, exiting buffer handler");
                        break;
                    }
                }
                Err(RecvError::Lagged(skipped)) => {
                    warn!(
                        "Buffered message handler lagged behind broadcast channel, \
                         skipped {} messages. Buffer stats: {}/{} ({}% full), {} total dropped",
                        skipped,
                        self.buffer.len(),
                        self.buffer.max_size,
                        self.buffer.utilization(),
                        self.buffer.dropped_count()
                    );
                }
                Err(RecvError::Closed) => {
                    debug!("Broadcast channel closed, flushing remaining buffer and exiting");
                    self.flush_buffer();
                    break;
                }
            }
        }
    }

    fn flush_buffer(&mut self) -> bool {
        while let Some(message) = self.buffer.pop() {
            if self.outgoing_sender.send(message).is_err() {
                debug!("Outgoing channel closed, stopping buffer flush");
                return false;
            }
        }
        true
    }
}
