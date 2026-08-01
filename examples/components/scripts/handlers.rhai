// Component Gallery interaction handlers.
//
// Navigation is handled declaratively by the <router>/<nav-link> in app.xml —
// there is no page-switching handler here anymore.

// Modal toggle handler
fn on_toggle_modal(component_id, event_data) {
    let current = get_component_property("demo_modal", "open");
    if current == true {
        set_component_property("demo_modal", "open", false);
    } else {
        set_component_property("demo_modal", "open", true);
    }
}

// Dropdown menu-item handler: proves <menu-item on-click> is wired.
// `component_id` is the clicked menu item's id, so the same handler can
// report which entry fired.
fn on_dropdown_select(component_id, event_data) {
    let action = if component_id == "dropdown_item_copy" {
        "Copy"
    } else if component_id == "dropdown_item_paste" {
        "Paste"
    } else if component_id == "dropdown_item_cut" {
        "Cut"
    } else {
        component_id
    };
    set_component_property("dropdown_status", "content", "Selected: " + action);
}

// Sidenav toggle handler: toggles collapsed state of the center demo sidenav
fn on_toggle_sidenav(component_id, event_data) {
    let current = get_component_property("sidenav_center_expanded", "collapsed");
    if current == true {
        set_component_property("sidenav_center_expanded", "collapsed", false);
    } else {
        set_component_property("sidenav_center_expanded", "collapsed", true);
    }
}

fn on_menu_roundness_square(component_id, event_data) {
    log_info("Header menu: Set roundness square");
    set_roundness("square");
}

fn on_menu_roundness_sharp(component_id, event_data) {
    log_info("Header menu: Set roundness sharp");
    set_roundness("sharp");
}

fn on_menu_roundness_default(component_id, event_data) {
    log_info("Header menu: Set roundness default");
    set_roundness("default");
}

fn on_menu_roundness_round(component_id, event_data) {
    log_info("Header menu: Set roundness round");
    set_roundness("round");
}
