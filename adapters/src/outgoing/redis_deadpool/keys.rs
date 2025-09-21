#[derive(Clone)]
pub struct RedisKeyBuilder {
    namespace: String,
}

impl RedisKeyBuilder {
    pub fn new(environment: &str) -> Self {
        Self {
            namespace: format!("fediplace:{}:tile:v2", environment),
        }
    }

    pub fn current_key(&self, x: i32, y: i32) -> String {
        format!("{}:{}:{}:current", self.namespace, x, y)
    }

    pub fn webp_key(&self, x: i32, y: i32, version: u64) -> String {
        format!("{}:{}:{}:webp:v{}", self.namespace, x, y, version)
    }

    pub fn rgba_key(&self, x: i32, y: i32, version: u64) -> String {
        format!("{}:{}:{}:rgba:v{}", self.namespace, x, y, version)
    }

    pub fn palette_key(&self, x: i32, y: i32, version: u64) -> String {
        format!("{}:{}:{}:palette:v{}", self.namespace, x, y, version)
    }

    pub fn missing_sentinel_key(&self, x: i32, y: i32) -> String {
        format!("{}:{}:{}:exists:false", self.namespace, x, y)
    }

    pub fn namespace_prefix(&self) -> String {
        format!("{}:*", self.namespace)
    }
}
