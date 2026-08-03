// SVG interactivity demo handlers.
//
// The SVG rasterizes to a single image, so its individual child elements are
// not separately addressable. Effects (recolor, rotate, scale) are produced by
// rewriting the whole SVG markup and storing it back on the `content` (or
// `src`) property; the component re-rasterizes on the next render.

// ── Embedded graphic: click toggles rotation + color ──────────────
fn on_svg_click(component_id, event_data) {
    let rotated = get_component_property(component_id, "rotated");
    let now_rotated = if rotated == true { false } else { true };
    set_component_property(component_id, "rotated", now_rotated);

    let fill = if now_rotated { "#a3be8c" } else { "url(#grad)" };
    let angle = if now_rotated { 45 } else { 0 };

    set_component_property(component_id, "content", svg_markup(fill, angle, false));
}

// ── Embedded graphic: hover enter highlights, hover leave restores ─
fn on_svg_hover(component_id, event_data) {
    // Highlight on enter ("hover"); restore on leave ("hover_end"). A click-
    // toggled rotation survives the hover restore because we read `rotated`.
    let highlight = event_data == "hover";
    let rotated = get_component_property(component_id, "rotated");
    let now_rotated = if rotated == true { true } else { false };
    let fill = if now_rotated { "#a3be8c" } else { "url(#grad)" };
    let angle = if now_rotated { 45 } else { 0 };
    set_component_property(component_id, "content", svg_markup(fill, angle, highlight));
}

// ── File graphic: hover swaps to a tinted variant via `src` ───────
// Demonstrates the `src` path: restyle by swapping the loaded file rather than
// rewriting markup. Hover enter loads the highlighted asset; leave restores.
fn on_file_hover(component_id, event_data) {
    let src = if event_data == "hover" {
        "assets/rust-logo-hover.svg"
    } else {
        "assets/rust-logo.svg"
    };
    set_component_property(component_id, "src", src);
}

// Builds the embedded graphic markup with a given rect fill, rotation, and an
// optional highlight stroke around the whole graphic.
fn svg_markup(fill, angle, highlight) {
    let stroke = if highlight { " stroke=\"#eceff4\" stroke-width=\"4\"" } else { "" };
    let head = "<svg width=\"120\" height=\"120\" viewBox=\"0 0 100 100\"" + stroke + ">";
    let defs =
        "<defs><linearGradient id=\"grad\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"1\">" +
        "<stop offset=\"0%\" stop-color=\"#bf616a\" />" +
        "<stop offset=\"100%\" stop-color=\"#ebcb8b\" /></linearGradient></defs>";
    let rect =
        "<rect x=\"5\" y=\"5\" width=\"90\" height=\"90\" rx=\"16\" fill=\"" + fill +
        "\" transform=\"rotate(" + angle + " 50 50)\" />";
    let circle =
        "<circle cx=\"50\" cy=\"50\" r=\"26\" fill=\"none\" stroke=\"#2e3440\" stroke-width=\"6\" />";
    let text = "<text x=\"50\" y=\"58\" font-size=\"24\" text-anchor=\"middle\" fill=\"#2e3440\">N</text>";
    head + defs + rect + circle + text + "</svg>"
}