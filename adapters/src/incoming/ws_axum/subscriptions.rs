use std::collections::HashSet;
use tracing::{debug, warn};

use domain::coords::TileCoord;

pub struct SubscriptionManager {
    subscribed_tiles: HashSet<TileCoord>,
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscribed_tiles: HashSet::new(),
        }
    }

    pub fn add_tiles(&mut self, tiles: Vec<TileCoord>) -> Vec<TileCoord> {
        tiles
            .into_iter()
            .filter(|tile_coord| {
                debug!("Client attempting to subscribe to tile: {}", tile_coord);
                if tile_coord.validate_bounds().is_ok() {
                    self.subscribed_tiles.insert(*tile_coord);
                    debug!("Successfully subscribed to tile: {}", tile_coord);
                    true
                } else {
                    warn!("Invalid tile coordinates for subscription: {}", tile_coord);
                    false
                }
            })
            .collect()
    }

    pub fn remove_tiles(&mut self, tiles: Vec<TileCoord>) -> Vec<TileCoord> {
        tiles
            .into_iter()
            .filter(|tile_coord| self.subscribed_tiles.remove(tile_coord))
            .collect()
    }

    pub fn is_subscribed_to(&self, tile_coord: TileCoord) -> bool {
        self.subscribed_tiles.contains(&tile_coord)
    }

    pub fn get_subscribed_tiles(&self) -> &HashSet<TileCoord> {
        &self.subscribed_tiles
    }
}
