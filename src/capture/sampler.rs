use crate::i18n::Key;
use crate::settings::{ScreenSyncLayout, ScreenSyncSide};
use anyhow::ensure;

struct PixelZone {
    x0: usize,
    x1: usize,
    y0: usize,
    y1: usize,
}

pub struct ScreenSyncSampler {
    width: usize,
    height: usize,
    zones: Vec<PixelZone>,
}

impl ScreenSyncSampler {
    pub fn new(
        layout: &ScreenSyncLayout,
        width: usize,
        height: usize,
        band_percent: u32,
    ) -> anyhow::Result<Self> {
        layout.validate()?;
        ensure!(width > 0 && height > 0, Key::EmptyFrame);
        ensure!((1..=50).contains(&band_percent), Key::BandDepthRange);
        ensure!(
            width
                .checked_mul(height)
                .and_then(|pixels| pixels.checked_mul(4))
                .is_some(),
            Key::FrameTooLarge
        );
        let depth_x = (width * band_percent as usize / 100).max(1);
        let depth_y = (height * band_percent as usize / 100).max(1);
        let mut zones = Vec::new();
        for segment in &layout.segments {
            let horizontal = matches!(segment.side, ScreenSyncSide::Top | ScreenSyncSide::Bottom);
            let length = if horizontal { width } else { height };
            let count = segment.led_count as usize;
            for output_index in 0..count {
                let index = if segment.reversed {
                    count - output_index - 1
                } else {
                    output_index
                };
                let start = index * length / count;
                let end = ((index + 1) * length / count).max(start + 1).min(length);
                let (x0, x1, y0, y1) = match segment.side {
                    ScreenSyncSide::Top => (start, end, 0, depth_y),
                    ScreenSyncSide::Bottom => (start, end, height - depth_y, height),
                    ScreenSyncSide::Left => (0, depth_x, start, end),
                    ScreenSyncSide::Right => (width - depth_x, width, start, end),
                };
                zones.push(PixelZone { x0, x1, y0, y1 });
            }
        }
        Ok(Self {
            width,
            height,
            zones,
        })
    }

    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    pub fn sample_bgra(
        &self,
        data: &[u8],
        row_pitch: usize,
        output: &mut [u8],
    ) -> anyhow::Result<()> {
        ensure!(
            row_pitch >= self.width * 4
                && row_pitch
                    .checked_mul(self.height)
                    .is_some_and(|size| data.len() >= size),
            Key::InvalidBgra
        );
        ensure!(output.len() == self.zones.len() * 3, Key::InvalidRgb);
        for (zone, rgb) in self.zones.iter().zip(output.chunks_exact_mut(3)) {
            let mut sums = [0_u64; 3];
            for y in zone.y0..zone.y1 {
                let row = &data[y * row_pitch + zone.x0 * 4..y * row_pitch + zone.x1 * 4];
                for bgra in row.chunks_exact(4) {
                    sums[0] += u64::from(bgra[2]);
                    sums[1] += u64::from(bgra[1]);
                    sums[2] += u64::from(bgra[0]);
                }
            }
            let count = ((zone.x1 - zone.x0) * (zone.y1 - zone.y0)) as u64;
            for channel in 0..3 {
                rgb[channel] = (sums[channel] / count) as u8;
            }
        }
        Ok(())
    }
}
