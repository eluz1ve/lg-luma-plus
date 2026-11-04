//! On-TV frame timing, for measuring where no debugger reaches: while a `perf` file sits beside
//! the preferences, each second that drew frames appends one line to `perf.log` there. A
//! `script=` line in that file plays remote presses at launch, as `--script` does, and
//! `msaa=`/`partial=` lines change how the next launch draws, for comparing on the TV.
use crate::diagnostics::Frame;
use std::{
    fs::File,
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

/// The log stops growing here.
const LIMIT: u64 = 4 * 1024 * 1024;
/// A longer pause between frames is idle time, not a slow frame.
const IDLE: Duration = Duration::from_millis(250);
/// A longer gap between frames missed at least one 60 Hz vsync.
const LATE_MS: f64 = 25.;

struct Sample {
    gap: f64,
    compose: f64,
    render: f64,
    swap: f64,
    image: f64,
    redrawn: f64,
}

pub struct PerfLog {
    file: File,
    written: u64,
    started: Instant,
    window: Instant,
    label: String,
    last: Option<Instant>,
    samples: Vec<Sample>,
    gpu: Vec<f64>,
}

impl PerfLog {
    pub fn open(directory: &Path) -> Option<Self> {
        if !directory.join("perf").exists() {
            return None;
        }
        let mut options = std::fs::OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o664);
        let file = options.open(directory.join("perf.log")).ok()?;
        let written = file.metadata().map_or(0, |metadata| metadata.len());
        let now = Instant::now();
        Some(Self {
            file,
            written,
            started: now,
            window: now,
            label: String::new(),
            last: None,
            samples: Vec::new(),
            gpu: Vec::new(),
        })
    }

    /// A line about the launch (how it draws), ahead of its frames.
    pub fn note(&mut self, text: &str) {
        let line = format!("t={:.1} {text}\n", self.started.elapsed().as_secs_f64());
        if self.written <= LIMIT && self.file.write_all(line.as_bytes()).is_ok() {
            self.written += line.len() as u64;
        }
    }

    /// GPU milliseconds of a frame drawn recently, counted in the current second.
    pub fn gpu(&mut self, milliseconds: f64) {
        self.gpu.push(milliseconds);
    }

    /// One presented frame; a new second or screen writes out the frames before it.
    pub fn record(&mut self, now: Instant, label: &str, frame: &Frame) {
        if now.duration_since(self.window) >= Duration::from_secs(1) || label != self.label {
            self.flush(now);
            self.label = label.to_owned();
        }
        let gap = self
            .last
            .map(|last| now.duration_since(last))
            .filter(|gap| *gap < IDLE)
            .map(|gap| gap.as_secs_f64() * 1000.);
        self.last = Some(now);
        if let Some(gap) = gap {
            self.samples.push(Sample {
                gap,
                compose: frame.compose_ms,
                render: frame.render_ms,
                swap: frame.swap_ms,
                image: frame.image_ms,
                redrawn: frame.redrawn,
            });
        }
    }

    fn flush(&mut self, now: Instant) {
        self.window = now;
        if self.samples.is_empty() || self.written > LIMIT {
            self.samples.clear();
            self.gpu.clear();
            return;
        }
        let line = line(
            now.duration_since(self.started).as_secs_f64(),
            &self.label,
            &self.samples,
            &self.gpu,
        );
        self.samples.clear();
        self.gpu.clear();
        if self.file.write_all(line.as_bytes()).is_ok() {
            self.written += line.len() as u64;
        }
    }
}

/// The presses a `script=a,b,c` line of the switch file asks for.
pub fn script(directory: &Path) -> Vec<String> {
    list(directory, "script=")
}

/// The draw kinds a `skip=glass,shadow` line leaves out, as LUMA_PROFILE_SKIP does.
pub fn skip(directory: &Path) -> Vec<String> {
    list(directory, "skip=")
}

/// The last `key` line's value (`msaa=0` → "0"), if the switch file has one.
pub fn value(directory: &Path, key: &str) -> Option<String> {
    let switch = std::fs::read_to_string(directory.join("perf")).ok()?;
    switch
        .lines()
        .rev()
        .filter_map(|line| line.strip_prefix(key))
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(String::from)
}

fn list(directory: &Path, key: &str) -> Vec<String> {
    let Ok(switch) = std::fs::read_to_string(directory.join("perf")) else {
        return Vec::new();
    };
    switch
        .lines()
        .filter_map(|line| line.strip_prefix(key))
        .flat_map(|items| items.split(','))
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(String::from)
        .collect()
}

fn line(seconds: f64, label: &str, samples: &[Sample], gpu: &[f64]) -> String {
    let stat = |value: fn(&Sample) -> f64| {
        let mean = samples.iter().map(value).sum::<f64>() / samples.len() as f64;
        let peak = samples.iter().map(value).fold(0., f64::max);
        format!("{mean:.1}/{peak:.1}")
    };
    let gpu = if gpu.is_empty() {
        String::new()
    } else {
        let mean = gpu.iter().sum::<f64>() / gpu.len() as f64;
        let peak = gpu.iter().copied().fold(0., f64::max);
        format!(" gpu {mean:.1}/{peak:.1}")
    };
    format!(
        "t={seconds:.1} {label} n={} gap {} compose {} render {} swap {} image {:.1} redrawn {:.2} late {}{gpu}\n",
        samples.len(),
        stat(|s| s.gap),
        stat(|s| s.compose),
        stat(|s| s.render),
        stat(|s| s.swap),
        samples.iter().map(|s| s.image).fold(0., f64::max),
        samples.iter().map(|s| s.redrawn).sum::<f64>() / samples.len() as f64,
        samples.iter().filter(|s| s.gap > LATE_MS).count(),
    )
}

#[cfg(test)]
#[path = "perf-log-tests.rs"]
mod tests;
