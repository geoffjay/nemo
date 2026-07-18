//! Design tokens for the running app — the gpui-coupled layer over the
//! gpui-free [`nemo_tokens`] data crate.
//!
//! The scales and semantic color roles live in [`nemo_tokens`] (shared with the
//! `xtask` design-system exporter so the two can't drift). This module
//! re-exports that data and adds the render-time helpers that need gpui:
//! [`space`], [`font_size`], and the [`TokenStyled`] extension trait.
//!
//! # Precedence
//!
//! At render time (low → high): gpui-component `Theme` defaults → these nemo
//! tokens (the default look for nemo-drawn chrome) → theme JSON colors →
//! per-component XML style overrides (always win). Token helpers set *defaults*;
//! they never override an explicit XML value.

// `TokenStyled` intentionally provides a complete, symmetric set of spacing
// helpers (all padding/margin sides); some are unused today but kept for a whole
// API. (Binary crate: `pub` alone does not suppress dead-code warnings.)
#![allow(dead_code)]

use gpui::{px, Pixels, Styled};

// Re-export the gpui-free token data so existing `crate::theme::tokens::*` paths
// keep working (scales, radius, `radius_px`, semantic roles, `resolve_role_alias`).
pub use nemo_tokens::*;

/// A spacing step as GPUI `Pixels`, for use in render code.
pub fn space(step: Space) -> Pixels {
    px(step.value())
}

/// Font size as GPUI `Pixels`.
pub fn font_size(size: FontSize) -> Pixels {
    px(size.size())
}

/// Ergonomic token-based styling on any GPUI `Styled` element.
///
/// These `*_t` helpers apply spacing/typography *tokens* instead of raw gpui
/// numeric helpers (`.gap_2()`, `.text_sm()`), so nemo-drawn chrome shares one
/// rhythm and a future scale change propagates everywhere. Values are identical
/// to the matching gpui helpers, so adoption is visually neutral. The `_t`
/// suffix avoids colliding with `Styled`'s own methods.
pub trait TokenStyled: Styled + Sized {
    fn gap_t(self, s: Space) -> Self {
        self.gap(space(s))
    }
    fn gap_x_t(self, s: Space) -> Self {
        self.gap_x(space(s))
    }
    fn gap_y_t(self, s: Space) -> Self {
        self.gap_y(space(s))
    }
    fn p_t(self, s: Space) -> Self {
        self.p(space(s))
    }
    fn px_t(self, s: Space) -> Self {
        self.px(space(s))
    }
    fn py_t(self, s: Space) -> Self {
        self.py(space(s))
    }
    fn pt_t(self, s: Space) -> Self {
        self.pt(space(s))
    }
    fn pb_t(self, s: Space) -> Self {
        self.pb(space(s))
    }
    fn pl_t(self, s: Space) -> Self {
        self.pl(space(s))
    }
    fn pr_t(self, s: Space) -> Self {
        self.pr(space(s))
    }
    fn m_t(self, s: Space) -> Self {
        self.m(space(s))
    }
    fn mx_t(self, s: Space) -> Self {
        self.mx(space(s))
    }
    fn my_t(self, s: Space) -> Self {
        self.my(space(s))
    }
    fn mt_t(self, s: Space) -> Self {
        self.mt(space(s))
    }
    fn mb_t(self, s: Space) -> Self {
        self.mb(space(s))
    }
    fn text_t(self, f: FontSize) -> Self {
        self.text_size(font_size(f))
    }
}

impl<T: Styled + Sized> TokenStyled for T {}
