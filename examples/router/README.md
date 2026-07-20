# Router

Demonstrates the chrome-free page router: `<router>`, `<route>`, and
`<nav-link>`.

Run it:

```sh
cargo run -p nemo -- dev examples/router/app.xml

# Start on a specific route instead of the default (this launch only):
cargo run -p nemo -- dev --route /users/42 examples/router/app.xml
cargo run -p nemo -- dev --route settings=/advanced examples/router/app.xml
```

## What it shows

* **Declarative navigation** — the top nav bar is a row of `<nav-link>`s. Each
  navigates its router (`router="main"`) to a `route` path and highlights when
  that path is active. No handler is involved.
* **URL-style routes with params** — `/users/:id` captures `id`, which is
  projected to `data.route.main.params.id` and shown on the page via a
  `bind-text="data.route.main.params.id"` binding.
* **Rhai navigation** — the Home page button calls `navigate("/users/7")` and
  the User Detail page button calls `back()`. Both drive the same router as the
  nav links. See `scripts/handlers.rhai`.
* **Lifecycle hooks** — the `/users/:id` route has `on-enter="on_user_enter"`,
  which fires (outside the extension lock) when the route becomes active.
* **Nested router** — the `/settings` route hosts its own `<router id="settings">`
  with `General` / `Advanced` sub-routes.
* **Not-found fallback** — `path="*"` matches anything the earlier routes did
  not, so the "Broken Link" nav item lands on a 404 page.

## How it works

Router state (history + params) is authoritative host-side in a registry on the
runtime; navigation is applied through a deferred queue so a `navigate()` call
from inside a handler never re-enters the extension lock. Only the active
route's body is mounted. See `docs/knowledgebase/patterns/routing.md`.
