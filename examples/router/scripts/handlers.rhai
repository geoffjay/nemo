// Router demo handlers.
//
// These show the Rhai navigation API driving the same router as the declarative
// <nav-link>s: `navigate(path)` targets the primary router, `back()` walks
// history, and a route `on-enter` hook reads the projected params.

// on-click on the Home page button: navigate the primary router to /users/7.
fn go_user_7(component_id, event_data) {
    navigate("/users/7");
}

// on-click on the User Detail page button: go back one history entry.
fn nav_back(component_id, event_data) {
    back();
}

// on-enter for /users/:id — fires when the route becomes active (component_id is
// the router id, "main"). Reads the freshly projected param.
fn on_user_enter(router_id, event_data) {
    let uid = get_data("route.main.params.id");
    log_info("entered user route for id=" + uid);
}
