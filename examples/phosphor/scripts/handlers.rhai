// Handlers for the Phosphor icon browser.
//
// The icon catalog is hydrated at startup from icons.txt (one Phosphor icon
// name per line, generated from the nemo-phosphor asset set) into
// `data.icons` — an array of `#{name, size, color, weight}` maps. The grid is
// a runtime `n:for` container over that array.
//
// Every header control mutates the shared display parameters and then calls
// `render_icons`, which rebuilds `data.icons` with a name filter; the runtime
// differ add/removes grid chips (identity is the icon name via `n:key`, so a
// narrower filter only removes chips) and stamps the new size/color/weight
// into every retained chip via the template placeholders.
//
// Rhai functions are pure: no module state, everything lives in the data store.

// ── Startup seed ─────────────────────────────────────────────────────────────

fn on_load(component_id, event_data) {
    set_data("weight", "light");
    set_data("query", "");
    set_data("size", 32);
    set_data("color", "#0F0F0F");
    let all = read_catalog();
    render_icons("");
}

// ── Header handlers ──────────────────────────────────────────────────────────

fn on_weight_change(component_id, event_data) {
    // event_data: the selected option value (capitalized display name).
    let weight = to_lower(event_data);
    set_data("weight", weight);
    set_component_property("size_label", "text", label_size_text());
    render_icons(get_data("query"));
}

// The dropdown-button dispatches each <menu-item>'s on-click with that item's
// id and a constant "click" event_data, so each weight needs its own handler.
fn on_click_thin(component_id, event_data) {
    on_weight_select("thin");
}

fn on_click_light(component_id, event_data) {
    on_weight_select("light");
}

fn on_click_regular(component_id, event_data) {
    on_weight_select("regular");
}

fn on_click_bold(component_id, event_data) {
    on_weight_select("bold");
}

fn on_click_fill(component_id, event_data) {
    on_weight_select("fill");
}

fn on_click_duotone(component_id, event_data) {
    on_weight_select("duotone");
}

fn on_weight_select(weight) {
    set_data("weight", weight);
    set_component_property("weight_select", "label", to_upper(weight));
    set_component_property("size_label", "text", label_size_text());
    render_icons(get_data("query"));
}

fn on_search_change(component_id, event_data) {
    set_data("query", event_data);
    render_icons(event_data);
}

// The slider reports whole numbers here (step = 1).
fn on_size_change(component_id, event_data) {
    let size = parse_int(event_data);
    set_data("size", size);
    set_component_property("size_label", "text", size + "px");
    render_icons(get_data("query"));
}

fn on_color_change(component_id, event_data) {
    let color = trim(event_data);
    if color == "" {
        // Empty means "theme foreground" (currentColor behavior).
        color = "currentColor";
    }
    set_data("color", color);
    render_icons(get_data("query"));
}

// ── Grid render ──────────────────────────────────────────────────────────────

// Rebuilds data.col_0..col_6 from the catalog with the active query applied:
// the (already-filtered) icon list is dealt out round-robin into seven column
// arrays, one per grid column.
fn render_icons(query) {
    let q = to_lower(trim(query));
    let all = read_catalog();
    let filtered = build_icons(all, q);
    deal_columns(filtered);
}

// Deals icon chips into the seven column arrays (data.col_0..col_6).
fn deal_columns(icons) {
    for c in range(0, 7) {
        set_data("col_" + to_string(c), column_slice(icons, c));
    }
}

fn column_slice(icons, col) {
    let slice = [];
    let i = col;
    while i < len(icons) {
        slice.push(icons[i]);
        i += 7;
    }
    slice
}

// Reads examples/phosphor/icons.txt (relative to the directory the app was
// launched from; the task-list example uses the same convention).
fn read_catalog() {
    let file = open_file("examples/phosphor/icons.txt", "r");
    let contents = file.read_string();
    split(contents, "\n")
}

// Builds chip maps (name/size/color/weight) for names containing the query.
fn build_icons(names, q) {
    let weight = get_data("weight");
    let size = get_data("size");
    let color = get_data("color");
    let icons = [];
    let i = 0;
    while i < len(names) {
        let name = trim(names[i]);
        if name != "" && (q == "" || contains(name, q)) {
            icons.push(#{ name: name, size: size, color: color, weight: weight });
        }
        i += 1;
    }
    icons
}

fn label_size_text() {
    let size = get_data("size");
    size + "px"
}