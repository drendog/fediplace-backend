use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::{task::spawn_blocking, time::timeout};

use fedi_wplace_application::ports::outgoing::{
    image_codec::DynImageCodecPort,
    timeout::{TimeoutError, WebPTimeoutPort},
};

pub struct TokioWebPTimeoutAdapter {
    codec_port: DynImageCodecPort,
}

impl TokioWebPTimeoutAdapter {
    pub fn new(codec_port: DynImageCodecPort) -> Self {
        Self { codec_port }
    }
}

impl WebPTimeoutPort for TokioWebPTimeoutAdapter {
    fn encode_webp_with_timeout(
        &self,
        rgba_pixels: Vec<u32>,
        duration: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, TimeoutError>> + Send + 'static>> {
        let codec = Arc::clone(&self.codec_port);

        Box::pin(async move {
            let task = spawn_blocking(move || codec.encode_lossless(&rgba_pixels));

            timeout(duration, task)
                .await
                .map_err(|_| TimeoutError)?
                .map_err(|_| TimeoutError)?
                .map_err(|_| TimeoutError)
        })
    }
}
