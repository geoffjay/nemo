use indexmap::IndexMap;
use nemo_plugin_api::*;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;

const WINDOW_SECS: f64 = 60.0;

struct ChannelWindow {
    samples: VecDeque<(Instant, f64)>,
}

impl ChannelWindow {
    fn new() -> Self {
        Self {
            samples: VecDeque::new(),
        }
    }

    fn push(&mut self, now: Instant, value: f64) {
        self.samples.push_back((now, value));
        self.evict(now);
    }

    fn evict(&mut self, now: Instant) {
        while let Some(&(t, _)) = self.samples.front() {
            if now.duration_since(t).as_secs_f64() > WINDOW_SECS {
                self.samples.pop_front();
            } else {
                break;
            }
        }
    }

    fn count(&self) -> usize {
        self.samples.len()
    }

    fn mean(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.samples.iter().map(|(_, v)| v).sum();
        sum / self.samples.len() as f64
    }

    fn min(&self) -> f64 {
        self.samples
            .iter()
            .map(|(_, v)| *v)
            .fold(f64::INFINITY, f64::min)
    }

    fn max(&self) -> f64 {
        self.samples
            .iter()
            .map(|(_, v)| *v)
            .fold(f64::NEG_INFINITY, f64::max)
    }

    fn stddev(&self) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }
        let mean = self.mean();
        let variance: f64 = self
            .samples
            .iter()
            .map(|(_, v)| (v - mean).powi(2))
            .sum::<f64>()
            / (self.samples.len() - 1) as f64;
        variance.sqrt()
    }
}

/// Extract all channel names and values from a NATS batch message.
///
/// The NATS source stores data as `{ subject: string, payload: object }`.
/// The payload contains `{ channels: { channel_N: { value: f64, ... }, ... }, timestamp: ... }`.
fn extract_metrics(value: &PluginValue) -> Vec<(String, f64)> {
    let mut results = Vec::new();
    if let PluginValue::Object(obj) = value {
        let payload = match obj.get("payload") {
            Some(p) => p,
            None => return results,
        };
        if let PluginValue::Object(payload_obj) = payload {
            if let Some(PluginValue::Object(channels)) = payload_obj.get("channels") {
                for (channel_name, channel_data) in channels {
                    if let PluginValue::Object(ch_obj) = channel_data {
                        let val = match ch_obj.get("value") {
                            Some(PluginValue::Float(f)) => *f,
                            Some(PluginValue::Integer(i)) => *i as f64,
                            _ => continue,
                        };
                        results.push((channel_name.clone(), val));
                    }
                }
            }
        }
    }
    results
}

fn init(registrar: &mut dyn PluginRegistrar) {
    let ctx = registrar.context_arc();

    // Set initial empty summary
    let _ = ctx.set_data("stats.summary", PluginValue::Array(vec![]));

    ctx.log(LogLevel::Info, "Streaming stats plugin initialized");

    spawn_stats_thread(ctx);
}

fn spawn_stats_thread(ctx: Arc<dyn PluginContext>) {
    std::thread::spawn(move || {
        let mut windows: HashMap<String, ChannelWindow> = HashMap::new();

        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));

            let now = Instant::now();

            // Read the latest batch from the NATS data source.
            // get_data() prepends "data." internally, so just use the source name.
            if let Some(value) = ctx.get_data("metrics") {
                for (channel, val) in extract_metrics(&value) {
                    let window = windows.entry(channel).or_insert_with(ChannelWindow::new);
                    window.push(now, val);
                }
            }

            // Evict old samples from all windows
            for window in windows.values_mut() {
                window.evict(now);
            }

            // Write per-channel stats
            let mut summary_rows: Vec<PluginValue> = Vec::new();

            let mut channels: Vec<&String> = windows.keys().collect();
            channels.sort();

            for channel in channels {
                let window = &windows[channel];
                if window.count() == 0 {
                    continue;
                }

                let mean = (window.mean() * 1000.0).round() / 1000.0;
                let min = (window.min() * 1000.0).round() / 1000.0;
                let max = (window.max() * 1000.0).round() / 1000.0;
                let stddev = (window.stddev() * 1000.0).round() / 1000.0;
                let count = window.count() as i64;

                let prefix = format!("stats.{channel}");
                let _ = ctx.set_data(&format!("{prefix}.mean"), PluginValue::Float(mean));
                let _ = ctx.set_data(&format!("{prefix}.min"), PluginValue::Float(min));
                let _ = ctx.set_data(&format!("{prefix}.max"), PluginValue::Float(max));
                let _ = ctx.set_data(&format!("{prefix}.stddev"), PluginValue::Float(stddev));
                let _ = ctx.set_data(&format!("{prefix}.count"), PluginValue::Integer(count));

                // Build summary row for table display
                let mut row = IndexMap::new();
                row.insert("channel".to_string(), PluginValue::String(channel.clone()));
                row.insert("mean".to_string(), PluginValue::Float(mean));
                row.insert("min".to_string(), PluginValue::Float(min));
                row.insert("max".to_string(), PluginValue::Float(max));
                row.insert("stddev".to_string(), PluginValue::Float(stddev));
                row.insert("count".to_string(), PluginValue::Integer(count));
                summary_rows.push(PluginValue::Object(row));
            }

            let _ = ctx.set_data("stats.summary", PluginValue::Array(summary_rows));
        }
    });
}

declare_plugin!(
    PluginManifest::new(
        "streaming-stats",
        "Streaming Statistics",
        semver::Version::new(0, 1, 0)
    )
    .with_description(
        "Computes rolling statistics (mean, min, max, stddev) over a 60-second sliding window"
    )
    .with_capability(Capability::DataSource("stats".to_string())),
    init
);
