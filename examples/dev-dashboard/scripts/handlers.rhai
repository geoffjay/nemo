// Dev Dashboard event handlers
//
// Demonstrates every Rhai extension package available to nemo scripts:
//
//   rhai-env      — read environment variables (env("HOME"), env("SHELL"), …)
//   rhai-process  — run shell commands (cmd(["uname", "-s"]).build().run())
//   rhai-sci      — compute statistics (mean, std, median, min, max)
//   rhai-chrono   — timestamp the clock (datetime_local().format(…))
//   built-in HTTP — fetch a public API (http_get("https://httpbin.org/uuid"))
//
// The app XML opts in via:
//   <script src="./scripts" features="file-io, system, science" />
//
// and the binary must be built with:
//   cargo build --features nemo-extension/pkg-env,nemo-extension/pkg-sci,nemo-extension/pkg-process

// ── State ────────────────────────────────────────────────────────────────────
// Response-time samples (ms) for the stats panel.
let samples = [];
let initialized = false;

// ── Initialization ───────────────────────────────────────────────────────────

fn ensure_init() {
    if initialized { return; }
    initialized = true;
    refresh_clock();
    refresh_env();
    refresh_sys();
}

// ── Clock (rhai-chrono) ───────────────────────────────────────────────────────

fn refresh_clock() {
    let now = datetime_local();
    let label = now.format("%H:%M:%S");
    set_component_text("clock", label);
}

// ── Environment Variables (rhai-env) ─────────────────────────────────────────

fn refresh_env() {
    let home = env("HOME");
    let shell = env("SHELL");
    let user = env("USER");
    let path = env("PATH");

    set_component_text("env_home", home);
    set_component_text("env_shell", shell);
    set_component_text("env_user", user);

    // Truncate PATH for display
    if len(path) > 80 {
        set_component_text("env_path", substr(path, 0, 80) + "...");
    } else {
        set_component_text("env_path", path);
    }
}

// ── System Info (rhai-process) ───────────────────────────────────────────────

fn refresh_sys() {
    // OS name (e.g. "Darwin" or "Linux")
    let os_result = cmd(["uname", "-s"]).build().run();
    if os_result.success {
        set_component_text("sys_os", trim(os_result.stdout));
    } else {
        set_component_text("sys_os", "uname failed: " + os_result.stderr);
    }

    // Hostname
    let host_result = cmd(["hostname"]).build().run();
    if host_result.success {
        set_component_text("sys_host", trim(host_result.stdout));
    } else {
        set_component_text("sys_host", "hostname failed");
    }

    // Disk usage (head -3 of df -h)
    let df_result = cmd(["df", "-h"]).build().run();
    if df_result.success {
        let lines = split_lines(df_result.stdout);
        let summary = "";
        let i = 0;
        while i < min(3, len(lines)) {
            if i > 0 { summary += " | "; }
            summary += trim(lines[i]);
            i += 1;
        }
        set_component_text("sys_disk", summary);
    } else {
        set_component_text("sys_disk", "df failed");
    }
}

// ── HTTP Fetch (built-in http_get) ───────────────────────────────────────────

fn fetch_uuid(component_id, event_data) {
    if !initialized { ensure_init(); }

    set_component_text("http_uuid", "Fetching...");
    set_component_text("http_status", "--");

    let response = http_get("https://httpbin.org/uuid");

    if response.ok {
        let status = response.status;
        set_component_text("http_status", "HTTP " + status);

        // body is a parsed JSON map: { "uuid": "..." }
        let body = response.body;
        if body != () && body.uuid != () {
            set_component_text("http_uuid", body.uuid);
        } else {
            set_component_text("http_uuid", "No uuid in response");
        }
    } else {
        let err = response.error;
        if err == () { err = "unknown error"; }
        set_component_text("http_uuid", "Failed: " + err);
        set_component_text("http_status", "Error");
    }
}

// ── Stats (rhai-sci) ──────────────────────────────────────────────────────────

fn take_sample(component_id, event_data) {
    if !initialized { ensure_init(); }

    // Simulate a response-time measurement (ms) with some randomness.
    // Uses the built-in http_get timing as a real-world sample.
    let start = datetime_utc().timestamp_millis();
    let response = http_get("https://httpbin.org/delay/1");
    let end = datetime_utc().timestamp_millis();
    let elapsed = to_float(end - start);

    samples.push(elapsed);
    update_stats();
    log_info("Sample " + len(samples) + ": " + elapsed + "ms");
}

fn update_stats() {
    let n = len(samples);
    set_component_text("stats_samples", "Samples: " + n);

    if n == 0 {
        set_component_text("stats_mean", "--");
        set_component_text("stats_std", "--");
        set_component_text("stats_minmax", "--");
        set_component_text("stats_median", "--");
        return;
    }

    // rhai-sci: mean, std, min, max, median
    let avg = mean(samples);
    let sd = std(samples);
    let lo = min(samples);
    let hi = max(samples);
    let med = median(samples);

    set_component_text("stats_mean", format_float(avg) + " ms");
    set_component_text("stats_std", format_float(sd) + " ms");
    set_component_text("stats_minmax", format_float(lo) + " / " + format_float(hi) + " ms");
    set_component_text("stats_median", format_float(med) + " ms");
}

fn reset_stats(component_id, event_data) {
    samples = [];
    update_stats();
    log_info("Stats reset");
}

// ── Refresh All ───────────────────────────────────────────────────────────────

fn refresh_all(component_id, event_data) {
    ensure_init();
    refresh_clock();
    refresh_env();
    refresh_sys();
    update_stats();
    log_info("Dashboard refreshed");
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn format_float(x) {
    // Round to 1 decimal place
    let rounded = round(x * 10.0) / 10.0;
    to_string(rounded)
}

fn split_lines(s) {
    // Rhai doesn't have a built-in split, so we split on newlines manually.
    // This is a simple implementation that handles \n.
    let result = [];
    let current = "";
    let i = 0;
    while i < len(s) {
        let ch = s[i..i+1];
        if ch == "\n" {
            result.push(current);
            current = "";
        } else if ch != "\r" {
            current += ch;
        }
        i += 1;
    }
    if current != "" {
        result.push(current);
    }
    result
}

fn substr(s, start, end) {
    // Slice a string from start (inclusive) to end (exclusive).
    let result = "";
    let i = start;
    while i < end && i < len(s) {
        result += s[i..i+1];
        i += 1;
    }
    result
}