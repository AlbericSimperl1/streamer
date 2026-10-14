//! mDNS advertisement of `_nvstream._tcp` so Moonlight finds this host
//! automatically.

use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;

const SERVICE_TYPE: &str = "_nvstream._tcp.local.";

pub struct MdnsAdvertiser {
    daemon: ServiceDaemon,
    fullname: String,
}

impl MdnsAdvertiser {
    pub fn start(hostname: &str, port: u16) -> Result<Self, String> {
        let daemon = ServiceDaemon::new().map_err(|e| format!("mdns daemon: {e}"))?;

        let instance = if hostname.trim().is_empty() {
            "hyprstream"
        } else {
            hostname.trim()
        };
        let host_label: String = instance
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let host_fqdn = format!("{host_label}.local.");

        let info = ServiceInfo::new(
            SERVICE_TYPE,
            instance,
            &host_fqdn,
            "",
            port,
            HashMap::<String, String>::new(),
        )
        .map_err(|e| format!("mdns service info: {e}"))?
        .enable_addr_auto();

        let fullname = info.get_fullname().to_string();
        daemon
            .register(info)
            .map_err(|e| format!("mdns register: {e}"))?;

        log::info!("mdns: advertising '{instance}' as {SERVICE_TYPE} on port {port}");
        Ok(Self { daemon, fullname })
    }
}

impl Drop for MdnsAdvertiser {
    fn drop(&mut self) {
        let _ = self.daemon.unregister(&self.fullname);
        let _ = self.daemon.shutdown();
    }
}
