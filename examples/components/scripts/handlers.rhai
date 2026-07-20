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

// Sidenav toggle handler: toggles collapsed state of the center demo sidenav
fn on_toggle_sidenav(component_id, event_data) {
    let current = get_component_property("sidenav_center_expanded", "collapsed");
    if current == true {
        set_component_property("sidenav_center_expanded", "collapsed", false);
    } else {
        set_component_property("sidenav_center_expanded", "collapsed", true);
    }
}
