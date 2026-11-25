use std::{
    net::{SocketAddr, UdpSocket},
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, RecvTimeoutError},
    },
    time::{Duration, Instant},
};

use crate::i18n::Key;
use crate::settings::{ScreenSyncCaptureSettings, ScreenSyncLayout};
use anyhow::Context as _;
use windows_capture::{
    capture::{CaptureControl, Context, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor,
    settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
    },
};

use super::{CaptureMonitor, sampler::ScreenSyncSampler};

pub fn monitors() -> anyhow::Result<Vec<CaptureMonitor>> {
    let primary = Monitor::primary()
        .and_then(|monitor| monitor.device_name())
        .context(Key::MonitorListFailed)?;
    Monitor::enumerate()
        .context(Key::MonitorListFailed)?
        .into_iter()
        .map(|monitor| {
            let name = monitor.device_name()?;
            Ok(CaptureMonitor {
                primary: name == primary,
                name,
                width: u64::from(monitor.width()?),
                height: u64::from(monitor.height()?),
            })
        })
        .collect()
}

struct LatestFrame {
    rgb: Vec<u8>,
    ready: bool,
}

struct CaptureHandler {
    layout: ScreenSyncLayout,
    band_percent: u32,
    interval: Duration,
    next_frame: Instant,
    sampler: Option<ScreenSyncSampler>,
    rgb: Vec<u8>,
    latest: Arc<Mutex<LatestFrame>>,
}

impl GraphicsCaptureApiHandler for CaptureHandler {
    type Flags = Self;
    type Error = anyhow::Error;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(ctx.flags)
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _: InternalCaptureControl,
    ) -> anyhow::Result<()> {
        let now = Instant::now();
        if now < self.next_frame {
            return Ok(());
        }
        let dimensions = (frame.width() as usize, frame.height() as usize);
        if self.sampler.as_ref().map(ScreenSyncSampler::dimensions) != Some(dimensions) {
            self.sampler = Some(ScreenSyncSampler::new(
                &self.layout,
                dimensions.0,
                dimensions.1,
                self.band_percent,
            )?);
        }
        let mut buffer = frame.buffer()?;
        let row_pitch = buffer.row_pitch() as usize;
        self.sampler
            .as_ref()
            .expect("Sampler was initialized")
            .sample_bgra(buffer.as_raw_buffer(), row_pitch, &mut self.rgb)?;
        let mut latest = self.latest.lock().map_err(|_| Key::CaptureThreadFailed)?;
        latest.rgb.copy_from_slice(&self.rgb);
        latest.ready = true;
        self.next_frame += self.interval;
        if self.next_frame <= now {
            self.next_frame = now + self.interval;
        }
        Ok(())
    }
}

pub fn run(
    settings: &ScreenSyncCaptureSettings,
    layout: &ScreenSyncLayout,
    address: SocketAddr,
    stop: Receiver<()>,
) -> anyhow::Result<()> {
    let monitor = Monitor::enumerate()
        .context(Key::MonitorListFailed)?
        .into_iter()
        .find(|monitor| {
            monitor
                .device_name()
                .is_ok_and(|name| name == settings.monitor)
        })
        .ok_or(Key::MonitorUnavailable)?;
    let socket = UdpSocket::bind("0.0.0.0:0").context(Key::SocketFailed)?;
    socket.connect(address).context(Key::TargetFailed)?;
    socket.set_write_timeout(Some(Duration::from_millis(100)))?;
    let mut rgb = vec![
        0;
        layout
            .segments
            .iter()
            .map(|segment| segment.led_count as usize)
            .sum::<usize>()
            * 3
    ];
    let latest = Arc::new(Mutex::new(LatestFrame {
        rgb: rgb.clone(),
        ready: false,
    }));
    let interval = Duration::from_secs_f64(1.0 / f64::from(settings.fps));
    let options = Settings::new(
        monitor,
        CursorCaptureSettings::WithoutCursor,
        DrawBorderSettings::WithoutBorder,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Bgra8,
        CaptureHandler {
            layout: layout.clone(),
            band_percent: settings.band_percent,
            interval,
            next_frame: Instant::now(),
            sampler: None,
            rgb: rgb.clone(),
            latest: latest.clone(),
        },
    );
    let capture = CaptureHandler::start_free_threaded(options).context(Key::CaptureStartFailed)?;
    let result = frame_loop(settings, &latest, &capture, &stop, &socket, &mut rgb);
    let stopped = capture.stop().context(Key::CaptureStopFailed);
    rgb.fill(0);
    let _ = socket.send(&rgb);
    result.and(stopped)
}

fn frame_loop(
    settings: &ScreenSyncCaptureSettings,
    latest: &Mutex<LatestFrame>,
    capture: &CaptureControl<CaptureHandler, anyhow::Error>,
    stop: &Receiver<()>,
    socket: &UdpSocket,
    rgb: &mut [u8],
) -> anyhow::Result<()> {
    let interval = Duration::from_secs_f64(1.0 / f64::from(settings.fps));
    let mut next_tick = Instant::now();
    let mut next_monitor_check = Instant::now() + Duration::from_secs(1);
    let first_frame_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let now = Instant::now();
        let wait = next_tick
            .saturating_duration_since(now)
            .min(Duration::from_millis(50));
        match stop.recv_timeout(wait) {
            Ok(()) | Err(RecvTimeoutError::Disconnected) => return Ok(()),
            Err(RecvTimeoutError::Timeout) => {}
        }
        // stop() joins the finished capture thread and returns its detailed error.
        if capture.is_finished() {
            return Err(Key::CaptureEnded.into());
        }
        let now = Instant::now();
        if now >= next_monitor_check {
            if !monitors()?
                .iter()
                .any(|monitor| monitor.name == settings.monitor)
            {
                return Err(Key::MonitorDisconnected.into());
            }
            next_monitor_check = now + Duration::from_secs(1);
        }
        if now < next_tick {
            continue;
        }
        let ready = {
            let latest = latest.lock().map_err(|_| Key::CaptureThreadFailed)?;
            rgb.copy_from_slice(&latest.rgb);
            latest.ready
        };
        // Windows may deliver no new frames for a static screen; keep the LEDs alive.
        if ready {
            let sent = socket.send(rgb).context(Key::FrameSendFailed)?;
            if sent != rgb.len() {
                return Err(Key::PartialFrame.into());
            }
        } else if now >= first_frame_deadline {
            return Err(Key::FirstFrameMissing.into());
        }
        next_tick += interval;
        if next_tick <= Instant::now() {
            next_tick = Instant::now() + interval;
        }
    }
}
