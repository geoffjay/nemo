// Click handler for the embedded SVG.
//
// The SVG rasterizes to a single image, so its individual child elements are
// not separately addressable. To "change a color or rotation of the svg
// elements" we rewrite the whole SVG markup and store it back on the
// component's `content` property; the component re-rasterizes on the next
// render. A custom `rotated` property tracks the toggle state.
fn on_svg_click(component_id, event_data) {
    let rotated = get_component_property(component_id, "rotated");
    let now_rotated = if rotated == true { false } else { true };
    set_component_property(component_id, "rotated", now_rotated);

    let fill = if now_rotated { "#a3be8c" } else { "url(#grad)" };
    let angle = if now_rotated { 45 } else { 0 };

    let svg = `
        <svg width="120" height="120" viewBox="0 0 100 100">
          <defs>
            <linearGradient id="grad" x1="0" y1="0" x2="1" y2="1">
              <stop offset="0%" stop-color="#bf616a" />
              <stop offset="100%" stop-color="#ebcb8b" />
            </linearGradient>
          </defs>
          <rect x="5" y="5" width="90" height="90" rx="16" fill="${fill}" transform="rotate(${angle} 50 50)" />
          <circle cx="50" cy="50" r="26" fill="none" stroke="#2e3440" stroke-width="6" />
          <text x="50" y="58" font-size="24" text-anchor="middle" fill="#2e3440">N</text>
        </svg>
    `;

    set_component_property(component_id, "content", svg);
}
