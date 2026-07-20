//! `nemo screenshot` — render an app config to a PNG and exit.
//!
//! macOS-first. Uses gpui's offscreen `Window::render_to_image` (Metal drawable
//! texture readback), which is only compiled in when the `screenshot` build
//! feature enables `gpui_platform/test-support`. The capture works while the
//! window is invisible and needs no screen-recording permission.
//!
//! The window is built via the shared `crate::build_app_window`, so the captured
//! frame is identical to what the normal run path renders. After launch we wait
//! `--settle-ms` on the real dispatcher (so async data bindings and first paint
//! land), capture, write the PNG, and quit.
//!
//! See docs/knowledgebase/decisions/screenshot-via-test-support-feature.md.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{bail, Context as _, Result};

use crate::args::ScreenshotArgs;
use crate::config::NemoConfig;
use crate::workspace::WorkspaceArgs;
use crate::{build_app_window, theme, BootstrapParams};

pub fn run(args: ScreenshotArgs) -> Result<()> {
    // Fail fast with a clear message before launching the GPUI app.
    if !args.app_config.exists() {
        bail!("config file not found: {}", args.app_config.display());
    }

    let size_override = args
        .size
        .as_deref()
        .map(parse_size)
        .transpose()
        .context("invalid --size")?;

    // A `--theme` alone applies with the default mode; `--mode` only takes
    // effect alongside a theme (there is no standalone mode override here).
    let theme_override = args.theme.clone().map(|name| {
        (
            name,
            args.mode.clone().unwrap_or_else(|| "system".to_string()),
        )
    });

    let nemo_config = NemoConfig::load_from(None);
    let out = args.out.clone();
    let settle = Duration::from_millis(args.settle_ms);

    let params = BootstrapParams {
        nemo_config,
        app_config_path: Some(args.app_config.clone()),
        ws_args: WorkspaceArgs {
            extension_dirs: Vec::new(),
            initial_route: args.route.clone(),
        },
        watch: None,
        size_override,
    };

    // Capture failures inside the async task surface here so the process exits
    // non-zero (the run loop itself always returns Ok once quit).
    let capture_err: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    let gpui_app = gpui_platform::application().with_assets(gpui_component_assets::Assets);
    let capture_err_run = capture_err.clone();
    gpui_app.run(move |cx| {
        let window = build_app_window(cx, params);

        // A `--theme` override must win over the app's own XML-configured theme,
        // which was applied while building the window above.
        if let Some((name, mode)) = &theme_override {
            theme::apply_configured_theme(name, mode, None, cx);
            let _ = window.update(cx, |_, w, _| w.refresh());
        }

        let out = out.clone();
        let capture_err_task = capture_err_run.clone();
        cx.spawn(async move |cx| {
            cx.background_executor().timer(settle).await;

            let result: Result<()> = cx.update(|cx| {
                let img = window
                    .update(cx, |_, w, _| w.render_to_image())
                    .context("window closed before capture")?
                    .context("failed to render window to image")?;
                img.save(&out)
                    .with_context(|| format!("failed to write {}", out.display()))?;
                Ok(())
            });

            if let Err(e) = result {
                *capture_err_task.lock().unwrap() = Some(format!("{e:#}"));
            }
            cx.update(|cx| cx.quit());
        })
        .detach();
    });

    if let Some(e) = capture_err.lock().unwrap().take() {
        bail!("{e}");
    }

    println!("Wrote {}", args.out.display());
    Ok(())
}

/// Parses a `WxH` size string (case-insensitive `x`) into logical pixels.
fn parse_size(s: &str) -> Result<(u32, u32)> {
    let (w, h) = s
        .split_once(['x', 'X'])
        .with_context(|| format!("expected WxH like 1200x800, got {s:?}"))?;
    let w = w
        .trim()
        .parse()
        .with_context(|| format!("invalid width in {s:?}"))?;
    let h = h
        .trim()
        .parse()
        .with_context(|| format!("invalid height in {s:?}"))?;
    Ok((w, h))
}

#[cfg(test)]
mod tests {
    use super::parse_size;

    #[test]
    fn parses_valid_sizes() {
        assert_eq!(parse_size("1200x800").unwrap(), (1200, 800));
        assert_eq!(parse_size("640X480").unwrap(), (640, 480));
        assert_eq!(parse_size(" 100 x 200 ").unwrap(), (100, 200));
    }

    #[test]
    fn rejects_bad_sizes() {
        assert!(parse_size("1200").is_err());
        assert!(parse_size("axb").is_err());
        assert!(parse_size("100x").is_err());
    }
}
