use std::{
    net::Ipv4Addr,
    sync::mpsc::{self, Sender},
    thread::{self, JoinHandle},
};

use crate::i18n::Key;
use crate::settings::DeviceSettings;

#[cfg(windows)]
mod sampler;
#[cfg(windows)]
mod windows;

pub struct CaptureMonitor {
    pub name: String,
    pub primary: bool,
    pub width: u64,
    pub height: u64,
}

#[cfg(windows)]
pub use windows::monitors;

#[cfg(not(windows))]
pub fn monitors() -> anyhow::Result<Vec<CaptureMonitor>> {
    Err(Key::WindowsOnly.into())
}

pub struct Capture {
    pub monitor: String,
    stop: Option<Sender<()>>,
    thread: Option<JoinHandle<anyhow::Result<()>>>,
}

impl Capture {
    pub fn start(ip: Ipv4Addr, settings: DeviceSettings) -> anyhow::Result<Self> {
        settings.validate()?;

        let monitor = settings.capture.monitor.clone();
        let (stop, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("screen-capture".into())
            .spawn(move || {
                #[cfg(windows)]
                return windows::run(
                    &settings.capture,
                    &settings.layout,
                    (ip, 5000).into(),
                    receiver,
                );

                #[cfg(not(windows))]
                {
                    drop(receiver);
                    Err(Key::WindowsOnly.into())
                }
            })?;

        Ok(Self {
            monitor,
            stop: Some(stop),
            thread: Some(thread),
        })
    }

    pub fn request_stop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }

    pub fn is_stopping(&self) -> bool {
        self.stop.is_none()
    }

    pub fn poll(&mut self) -> Option<anyhow::Result<()>> {
        self.thread.as_ref()?.is_finished().then(|| self.join())
    }

    fn join(&mut self) -> anyhow::Result<()> {
        match self.thread.take() {
            Some(thread) => thread.join().map_err(|_| Key::CaptureThreadFailed)?,
            None => Ok(()),
        }
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.request_stop();
        let _ = self.join();
    }
}
