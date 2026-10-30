//! Frame timer and frame rate for the title bar (Settings › Appearance): the CPU time
//! the frames take and how many are drawn per second, averaged over half a second so
//! that the title stays readable. The app draws only when something changes, so an idle
//! window shows few frames per second.

/// How long the values are averaged before the title shows them (s).
const EVERY: f64 = 0.5;

#[derive(Debug, Default)]
pub struct FrameStats {
    /// Start of the current interval, its frames and their CPU time (s).
    since: Option<f64>,
    frames: u32,
    cpu: f32,
    /// Shown: frame time (ms) and frames per second.
    shown: Option<(f32, f32)>,
}

impl FrameStats {
    /// Counts a frame drawn at `time` (s) and the CPU time of the frame before (s; none
    /// before the first).
    pub fn record(&mut self, time: f64, cpu: Option<f32>) {
        let Some(since) = self.since else {
            self.since = Some(time);
            return;
        };
        self.frames += 1;
        self.cpu += cpu.unwrap_or(0.0);
        let span = time - since;
        if span >= EVERY {
            self.shown = Some((self.cpu / self.frames as f32 * 1000.0, (f64::from(self.frames) / span) as f32));
            (self.since, self.frames, self.cpu) = (Some(time), 0, 0.0);
        }
    }

    /// The text for the title bar, e.g. `4.2 ms · 60 FPS`, once there is one.
    pub fn text(&self) -> Option<String> {
        self.shown.map(|(ms, fps)| format!("{ms:.1} ms · {fps:.0} FPS"))
    }
}

#[cfg(test)]
mod tests;
