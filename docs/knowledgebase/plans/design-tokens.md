---
type: Plan
title: Design tokens and active redesign
description: A centralized design-token layer (spacing/radius/typography/semantic colors) in crates/nemo/src/theme/tokens.rs, migrated component-by-component with screenshot verification.
tags: [design-system, tokens, theme, redesign, screenshots]
timestamp: 2026-07-18T00:00:00Z
---

# Why

Nemo had **no centralized design-token layer**: colors were semi-centralized (the
`resolve_theme_color` macro table in `components/mod.rs`), radius/shadow were
enum presets, and spacing/typography were hardcoded `px(...)` / `.text_sm()`
literals scattered across ~54 component render methods. That makes a coherent
look hard to maintain and gives the [design-system export](design-system-export.md)
nothing canonical to serialize. This is Phase 2 of the design-system initiative
(Phase 1 = [screenshots](headless-screenshots.md), which is the before/after
validation loop this redesign uses).

# The token module

`crates/nemo/src/theme/tokens.rs` (declared `pub mod tokens;` in `theme/mod.rs`)
— code-first constants, the single source of truth, and export-ready:

* **`space`** — 4px-based scale (xs 4, sm 8, md 12, lg 16, xl 24, xxl 32); `Space`
  enum + `space(Space) -> Pixels`.
* **`radius`** — sm 4, md 6, lg 8, xl 12, full 9999; matches the gpui-component
  rounded presets exactly (Tailwind `rems`: sm .25/md .375/lg .5/xl .75 × 16px),
  so adopting them is visually neutral. `radius_px(name)` maps XML preset names.
* **`FontSize`** — xs 12 / sm 14 / base 16 / lg 18 / xl 24 / xxl 30, each with a
  line height; aligns with the gpui text helpers. `font_size(FontSize) -> Pixels`.
* **`SEMANTIC_COLOR_ROLES`** — role → theme-field aliases (`surface`→background,
  `surface_raised`→secondary, `surface_overlay`→popover, `text`→foreground,
  `text_muted`→muted_foreground, `border_subtle`→border, `focus_ring`→ring).
  `resolve_role_alias(name)` translates them; roles with a direct theme name
  (accent/primary/danger/…) need no alias.

# Precedence (preserved)

Low → high: gpui-component `Theme` defaults → nemo tokens (default look for
nemo-drawn chrome) → theme JSON colors → **per-component XML style overrides
(always win)**. Token helpers set *defaults*; the `props.get(...)` override
branches in each `render` stay intact and last.

# Wiring landed

* `resolve_theme_color` (`components/mod.rs`) translates role aliases first, so
  XML can reference `theme.surface`, `theme.text_muted`, etc. Purely additive —
  existing names pass through unchanged.
* `apply_rounded` (`components/mod.rs`) now reads `radius_px` (single radius
  source); verified visually identical to the old gpui presets.
* **`TokenStyled` extension trait** (in `tokens.rs`, impl'd for all `Styled`):
  ergonomic `*_t` helpers — `.gap_t(Space::Sm)`, `.p_t/.px_t/.py_t/.pt_t/…`,
  `.text_t(FontSize::Sm)` — applied to nemo-drawn chrome instead of raw gpui
  numeric helpers. The `_t` suffix avoids colliding with `Styled`'s own methods.

# The gpui-equivalence that makes the sweep neutral

Token values are byte-identical to the gpui helpers they replace, so the sweep is
a pure refactor (confirmed by screenshots + 203 tests):

* Spacing: gpui `_1`=4 (Xs), `_2`=8 (Sm), `_3`=12 (Md), `_4`=16 (Lg), `_6`=24
  (Xl), `_8`=32 (Xxl). `_5`/`_7` have no token — left as gpui helpers.
* Radius: gpui `rounded_sm`=4, `md`=6, `lg`=8, `xl`=12 (Tailwind rems × 16px).
* Typography: gpui `text_xs`=12, `text_sm`=14, base=16, `text_lg`=18,
  `text_xl`=**20**, `text_2xl`=24. `FontSize::Xl`=20 (NOT 24) so it matches
  `text_xl` — required for the `label` component's `xl` size to stay neutral.
* `px(...)` literals in chart components are **plot geometry** (bar widths, point
  radii), not design spacing — intentionally untouched.

# Migration status

1. **Done:** token module + wiring + `TokenStyled` trait.
2. **Active redesign (visible):** `panel` draws a subtle 1px hairline border by
   default (`border="0"` opts out; explicit width wins) → clean card hierarchy.
   Verified on `examples/components` and `examples/complete`.
3. **Done — full chrome sweep:** every convertible spacing/typography literal in
   the components module (`accordion`, `collapsible`, `list`, `modal`, `tabs`,
   `notification`, `sidenav_bar`, `select`, `toggle`, `label`, plus the earlier
   `panel`/`button`/`chart_utils`) **and** the app chrome (`workspace/`:
   `settings`, `project_loader`, `header_bar`, `utils`, `mod`; `containers/app_shell`;
   `app.rs` fallback) now use tokens. Repo-wide grep confirms no convertible
   literal remains outside doc comments. Screenshots pixel-neutral; 203 tests pass.
4. **Future:** deliberate redesign passes can now adjust *token values* once and
   have the whole UI follow.

# Verification

`nemo test` (token unit tests: scale ordering, radius/font match, role aliasing) +
`nemo screenshot` in both modes. Neutral-migration changes are confirmed pixel-
identical against captured PNGs; deliberate redesign changes are eyeballed.
