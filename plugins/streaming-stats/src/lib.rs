use indexmap::IndexMap;
use nemo_plugin_api::*;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;

const WINDOW_SECS: f64 = 60.0;
const TIMESERIES_SIZE: usize = 30;

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

/// Extract all channel names, values, and units from a NATS batch message.
///
/// The NATS source stores data as `{ subject: string, payload: object }`.
/// The payload contains `{ channels: { channel_N: { value: f64, unit: string, ... }, ... }, timestamp: ... }`.
fn extract_metrics(value: &PluginValue) -> Vec<(String, f64, String)> {
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
                        let unit = match ch_obj.get("unit") {
                            Some(PluginValue::String(s)) => s.clone(),
                            _ => "unknown".to_string(),
                        };
                        results.push((channel_name.clone(), val, unit));
                    }
                }
            }
        }
    }
    results
}

/// Map unit strings to time-series category names.
fn unit_to_category(unit: &str) -> Option<&'static str> {
    match unit {
        "celsius" => Some("temperature"),
        "percent" => Some("humidity"),
        "psi" => Some("pressure"),
        "rpm" => Some("speed"),
        _ => None,
    }
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
        // channel_name -> unit
        let mut channel_units: HashMap<String, String> = HashMap::new();
        // Rolling time-series snapshots: each entry is (channel_name -> value)
        let mut timeseries: VecDeque<HashMap<String, f64>> = VecDeque::new();

        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));

            let now = Instant::now();

            // Read the latest batch from the NATS data source.
            let mut snapshot: HashMap<String, f64> = HashMap::new();

            if let Some(value) = ctx.get_data("metrics") {
                for (channel, val, unit) in extract_metrics(&value) {
                    let window = windows.entry(channel.clone()).or_insert_with(ChannelWindow::new);
                    window.push(now, val);
                    channel_units.insert(channel.clone(), unit);
                    snapshot.insert(channel, val);
                }
            }

            // Evict old samples from all windows
            for window in windows.values_mut() {
                window.evict(now);
            }

            // Update time-series rolling window
            if !snapshot.is_empty() {
                timeseries.push_back(snapshot);
                while timeseries.len() > TIMESERIES_SIZE {
                    timeseries.pop_front();
                }
            }

            // Write per-channel stats
            let mut summary_rows: Vec<PluginValue> = Vec::new();

            let mut channels: Vec<&String> = windows.keys().collect();
            channels.sort();

            for channel in &channels {
                let window = &windows[*channel];
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
                row.insert("channel".to_string(), PluginValue::String((*channel).clone()));
                row.insert("mean".to_string(), PluginValue::Float(mean));
                row.insert("min".to_string(), PluginValue::Float(min));
                row.insert("max".to_string(), PluginValue::Float(max));
                row.insert("stddev".to_string(), PluginValue::Float(stddev));
                row.insert("count".to_string(), PluginValue::Integer(count));
                summary_rows.push(PluginValue::Object(row));
            }

            let _ = ctx.set_data("stats.summary", PluginValue::Array(summary_rows));

            // Build time-series arrays grouped by metric category
            // Group channels by category
            let mut category_channels: HashMap<&str, Vec<&String>> = HashMap::new();
            for channel in &channels {
                if let Some(unit) = channel_units.get(*channel) {
                    if let Some(category) = unit_to_category(unit) {
                        category_channels
                            .entry(category)
                            .or_default()
                            .push(*channel);
                    }
                }
            }

            let ts_len = timeseries.len();
            for (category, cat_channels) in &category_channels {
                let mut rows: Vec<PluginValue> = Vec::with_capacity(ts_len);

                for (i, snapshot) in timeseries.iter().enumerate() {
                    let mut row = IndexMap::new();
                    // Time label: relative seconds from newest, e.g. "-29s" .. "0s"
                    let secs_ago = ts_len as i64 - 1 - i as i64;
                    let label = if secs_ago == 0 {
                        "0s".to_string()
                    } else {
                        format!("-{secs_ago}s")
                    };
                    row.insert("time".to_string(), PluginValue::String(label));

                    for ch in cat_channels {
                        let val = snapshot.get(*ch).copied().unwrap_or(0.0);
                        let val = (val * 1000.0).round() / 1000.0;
                        row.insert(ch.to_string(), PluginValue::Float(val));
                    }

                    rows.push(PluginValue::Object(row));
                }

                let _ = ctx.set_data(
                    &format!("stats.timeseries.{category}"),
                    PluginValue::Array(rows),
                );
            }
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
