use if_addrs::get_if_addrs;

pub fn detect_lan_ips() -> Vec<String> {
    let mut ips: Vec<String> = Vec::new();
    if let Ok(interfaces) = get_if_addrs() {
        for iface in interfaces {
            if iface.is_loopback() {
                continue;
            }
            if iface.ip().is_ipv4() {
                ips.push(iface.ip().to_string());
            }
        }
    }
    if ips.is_empty() {
        ips.push("127.0.0.1".to_string());
    }
    ips
}
