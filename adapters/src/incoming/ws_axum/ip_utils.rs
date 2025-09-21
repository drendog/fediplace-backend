use axum::http::Request;
use std::net::{IpAddr, SocketAddr};

pub fn extract_client_ip<B>(
    req: &Request<B>,
    socket: Option<SocketAddr>,
    trust_xff: bool,
) -> IpAddr {
    if trust_xff {
        if let Some(forwarded_for) = req.headers().get("x-forwarded-for") {
            if let Ok(header_value) = forwarded_for.to_str() {
                if let Some(first_ip) = header_value.split(',').next() {
                    if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                        return ip;
                    }
                }
            }
        }

        if let Some(real_ip) = req.headers().get("x-real-ip") {
            if let Ok(header_value) = real_ip.to_str() {
                if let Ok(ip) = header_value.parse::<IpAddr>() {
                    return ip;
                }
            }
        }
    }

    socket.map_or_else(|| IpAddr::from([127, 0, 0, 1]), |addr| addr.ip())
}
