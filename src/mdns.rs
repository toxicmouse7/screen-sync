use std::{
    collections::BTreeSet,
    net::{IpAddr, Ipv4Addr},
};

use mdns_sd::{HostnameResolutionEvent, Receiver, ServiceDaemon};

const HOST: &str = "screen-sync.local.";

pub struct Search {
    session: Option<(ServiceDaemon, Receiver<HostnameResolutionEvent>)>,
    pub addresses: BTreeSet<Ipv4Addr>,
}

impl Search {
    pub fn start() -> Result<Self, mdns_sd::Error> {
        let daemon = ServiceDaemon::new()?;
        let events = match daemon.resolve_hostname(HOST, Some(3_000)) {
            Ok(events) => events,
            Err(error) => {
                let _ = daemon.shutdown();
                return Err(error);
            }
        };
        Ok(Self {
            session: Some((daemon, events)),
            addresses: BTreeSet::new(),
        })
    }

    pub fn poll(&mut self) -> bool {
        let Some((_, events)) = &self.session else {
            return false;
        };
        while let Ok(event) = events.try_recv() {
            match event {
                HostnameResolutionEvent::AddressesFound(_, addresses) => {
                    for address in addresses {
                        if let IpAddr::V4(ip) = address.to_ip_addr() {
                            self.addresses.insert(ip);
                        }
                    }
                }
                HostnameResolutionEvent::AddressesRemoved(_, addresses) => {
                    for address in addresses {
                        if let IpAddr::V4(ip) = address.to_ip_addr() {
                            self.addresses.remove(&ip);
                        }
                    }
                }
                HostnameResolutionEvent::SearchTimeout(_)
                | HostnameResolutionEvent::SearchStopped(_) => {
                    self.stop();
                    return false;
                }
                _ => {}
            }
        }
        if events.is_disconnected() {
            self.stop();
            return false;
        }
        true
    }

    fn stop(&mut self) {
        if let Some((daemon, _)) = self.session.take() {
            let _ = daemon.shutdown();
        }
    }
}

impl Drop for Search {
    fn drop(&mut self) {
        self.stop();
    }
}
