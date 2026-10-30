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

// Header-bar sidenav collapse toggle: flips the app-shell's `collapsed`
// property; the UI re-renders because property mutations wake the App loop.
fn on_toggle_nav(component_id, event_data) {
    let current = get_component_property("app_shell", "collapsed");
    set_component_property("app_shell", "collapsed", current == false);
    log_info("Toggled nav collapse: " + (current == false));
}
