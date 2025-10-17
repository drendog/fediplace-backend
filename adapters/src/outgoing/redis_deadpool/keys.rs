use uuid::Uuid;

#[derive(Clone)]
pub struct RedisKeyBuilder {
    tile_namespace: String,
    subscription_namespace: String,
}

impl RedisKeyBuilder {
    pub fn new(environment: &str) -> Self {
        let root_namespace = "fediplace";
        Self {
            tile_namespace: format!("{}:{}:tile:v3", root_namespace, environment),
            subscription_namespace: format!("{}:{}:sub:v3", root_namespace, environment),
        }
    }

    pub fn current_key(&self, world_id: &Uuid, x: i32, y: i32) -> String {
        format!("{}:{}:{}:{}:current", self.tile_namespace, world_id, x, y)
    }

    pub fn webp_key(&self, world_id: &Uuid, x: i32, y: i32, version: u64) -> String {
        format!(
            "{}:{}:{}:{}:webp:v{}",
            self.tile_namespace, world_id, x, y, version
        )
    }

    pub fn rgba_key(&self, world_id: &Uuid, x: i32, y: i32, version: u64) -> String {
        format!(
            "{}:{}:{}:{}:rgba:v{}",
            self.tile_namespace, world_id, x, y, version
        )
    }

    pub fn palette_key(&self, world_id: &Uuid, x: i32, y: i32, version: u64) -> String {
        format!(
            "{}:{}:{}:{}:palette:v{}",
            self.tile_namespace, world_id, x, y, version
        )
    }

    pub fn missing_sentinel_key(&self, world_id: &Uuid, x: i32, y: i32) -> String {
        format!(
            "{}:{}:{}:{}:exists:false",
            self.tile_namespace, world_id, x, y
        )
    }

    pub fn namespace_prefix(&self) -> String {
        format!("{}:*", self.tile_namespace)
    }

    pub fn world_namespace_prefix(&self, world_id: &Uuid) -> String {
        format!("{}:{}:*", self.tile_namespace, world_id)
    }

    pub fn subscription_key(&self, world_id: &Uuid, ip_key: &str) -> String {
        format!("{}:{}:{}", self.subscription_namespace, world_id, ip_key)
    }
}
