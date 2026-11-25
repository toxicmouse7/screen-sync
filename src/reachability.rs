use std::{
    collections::BTreeMap,
    net::Ipv4Addr,
    ptr,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use eframe::egui;
use windows_sys::Win32::{
    Foundation::INVALID_HANDLE_VALUE,
    NetworkManagement::IpHelper::{IP_SUCCESS, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho},
};

const INTERVAL: Duration = Duration::from_secs(2);
const MISSES: u8 = 3;

#[derive(Clone, Copy)]
pub enum Status {
    Checking,
    Online,
    NoReply,
}

#[derive(Default)]
pub struct Reachability {
    samples: BTreeMap<Ipv4Addr, (Status, u8)>,
    pending: Option<Receiver<(Ipv4Addr, bool)>>,
    next_check: Option<Instant>,
}

impl Reachability {
    pub fn status(&self, ip: Ipv4Addr) -> Status {
        self.samples
            .get(&ip)
            .map_or(Status::Checking, |&(status, _)| status)
    }

    pub fn forget(&mut self, ip: Ipv4Addr) {
        self.samples.remove(&ip);
    }

    pub fn poll(
        &mut self,
        ctx: &egui::Context,
        addresses: impl Iterator<Item = Ipv4Addr>,
        contains: impl Fn(Ipv4Addr) -> bool,
    ) {
        if let Some(pending) = &self.pending {
            loop {
                match pending.try_recv() {
                    Ok((ip, online)) if contains(ip) => {
                        let (status, misses) =
                            self.samples.entry(ip).or_insert((Status::Checking, 0));
                        if online {
                            *status = Status::Online;
                            *misses = 0;
                        } else {
                            *misses = (*misses + 1).min(MISSES);
                            if *misses == MISSES {
                                *status = Status::NoReply;
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(TryRecvError::Empty) => return,
                    Err(TryRecvError::Disconnected) => {
                        self.pending = None;
                        break;
                    }
                }
            }
        }

        let now = Instant::now();
        if let Some(next) = self.next_check.filter(|&next| next > now) {
            ctx.request_repaint_after(next - now);
            return;
        }

        let addresses: Vec<_> = addresses.collect();
        if addresses.is_empty() {
            return;
        }

        self.next_check = Some(now + INTERVAL);
        let (sender, receiver) = mpsc::channel();
        let repaint = ctx.clone();
        if thread::Builder::new()
            .name("icmp-check".into())
            .spawn(move || {
                for ip in addresses {
                    if sender.send((ip, ping(ip))).is_err() {
                        return;
                    }
                    repaint.request_repaint();
                }
                drop(sender);
                repaint.request_repaint();
            })
            .is_ok()
        {
            self.pending = Some(receiver);
        } else {
            ctx.request_repaint_after(INTERVAL);
        }
    }
}

fn ping(ip: Ipv4Addr) -> bool {
    let destination = u32::from_ne_bytes(ip.octets());
    let request = [0u8; 8];
    let mut reply = [0u64; 8];
    // The aligned buffer holds a reply header, the payload, and space for ICMP error data.
    unsafe {
        let handle = IcmpCreateFile();
        if handle == INVALID_HANDLE_VALUE {
            return false;
        }
        let count = IcmpSendEcho(
            handle,
            destination,
            request.as_ptr().cast(),
            request.len() as u16,
            ptr::null(),
            reply.as_mut_ptr().cast(),
            size_of_val(&reply) as u32,
            700,
        );
        IcmpCloseHandle(handle);

        // Both reply layouts start with the source address and the IP status as DWORDs.
        let header = reply[0].to_ne_bytes();
        count > 0
            && header[..4] == ip.octets()
            && u32::from_ne_bytes([header[4], header[5], header[6], header[7]]) == IP_SUCCESS
    }
}
