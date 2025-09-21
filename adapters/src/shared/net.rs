use std::net::IpAddr;

pub fn ip_key(ip: IpAddr) -> String {
    format!("ws:subs:ip:{}", ip)
}
