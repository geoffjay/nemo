//! [Phosphor](https://phosphoricons.com) icons for nemo.
//!
//! A port of the `gpui-kit-phosphor` crate (0.1.0) compiled against the
//! workspace's exact `gpui-pre` snapshot (`=0.3.10`); the upstream crate pins
//! `=0.3.9`, so it cannot be a direct dependency of this workspace without forking
//! the `gpui` graph. The vendor script copies the upstream `assets/` and
//! `build.rs` verbatim (see `script/`); the catalog codegen therefore matches
//! upstream exactly: all 1,512 icons in six weights, embedded as
//! `include_bytes!` constants.
//!
//! ```no_run
//! use nemo_phosphor::{Phosphor, PhosphorIcon, Weight};
//!
//! let icon = PhosphorIcon::from_name("acorn").unwrap();
//! let phosphor = icon.weight(Weight::Light);
//! // Asset path served by `PhosphorAssets`: `phosphor/light/acorn.svg`.
//! assert_eq!(phosphor.path().as_ref(), "phosphor/light/acorn.svg");
//! ```
//!
//! Register the asset source ahead of GPUI Kit's own so `svg().path(..)` finds
//! these:
//!
//! ```no_run
//! // gpui_platform::application()
//! //     .with_assets(nemo_phosphor::PhosphorAssets.with(gpui_kit::assets::Assets));
//! ```
//!
//! Depending on this crate does not put every SVG into the binary: only icons a
//! code path actually references are emitted, because the generated tables are
//! `const` items consulted through `svg().path(..)` lookups at runtime.

use std::borrow::Cow;

use gpui::{AssetSource, SharedString};

pub const PATH_PREFIX: &str = "phosphor/";

include!(concat!(env!("OUT_DIR"), "/phosphor.rs"));

/// A Phosphor icon weight (style).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Weight {
    Thin,
    Light,
    #[default]
    Regular,
    Bold,
    Fill,
    Duotone,
}

impl Weight {
    /// Every weight, lightest first.
    pub const ALL: [Weight; 6] = [
        Weight::Thin,
        Weight::Light,
        Weight::Regular,
        Weight::Bold,
        Weight::Fill,
        Weight::Duotone,
    ];

    /// The lowercase weight name used in asset paths, e.g. `"bold"`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Weight::Thin => "thin",
            Weight::Light => "light",
            Weight::Regular => "regular",
            Weight::Bold => "bold",
            Weight::Fill => "fill",
            Weight::Duotone => "duotone",
        }
    }

    /// Parse a lowercase weight name.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|weight| weight.as_str() == name)
    }

    /// Whether [`PhosphorAssets`] embeds this weight in the current build.
    pub const fn is_embedded(self) -> bool {
        match self {
            Weight::Thin => cfg!(feature = "thin"),
            Weight::Light => cfg!(feature = "light"),
            Weight::Regular => cfg!(feature = "regular"),
            Weight::Bold => cfg!(feature = "bold"),
            Weight::Fill => cfg!(feature = "fill"),
            Weight::Duotone => cfg!(feature = "duotone"),
        }
    }
}

impl std::fmt::Display for Weight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Weight {
    type Err = ParseError;

    /// Parse a lowercase weight name, e.g. `"bold"`.
    fn from_str(name: &str) -> Result<Self, ParseError> {
        Self::from_name(name).ok_or(ParseError(()))
    }
}

/// Returned when parsing an unknown icon or weight name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseError(());

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("unknown Phosphor icon or weight name")
    }
}

impl std::error::Error for ParseError {}

impl std::fmt::Display for PhosphorIcon {
    /// The Phosphor name, e.g. `arrow-up-right`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl std::str::FromStr for PhosphorIcon {
    type Err = ParseError;

    /// Parse a Phosphor name, e.g. `"arrow-up-right"`.
    fn from_str(name: &str) -> Result<Self, ParseError> {
        Self::from_name(name).ok_or(ParseError(()))
    }
}

impl std::fmt::Display for Phosphor {
    /// The name and weight, e.g. `gear (bold)`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.icon, self.weight)
    }
}

impl PhosphorIcon {
    /// Look an icon up by its Phosphor name, e.g. `"address-book"`.
    pub fn from_name(name: &str) -> Option<Self> {
        BY_NAME
            .binary_search_by(|(candidate, _)| (*candidate).cmp(name))
            .ok()
            .map(|index| BY_NAME[index].1)
    }

    /// Every icon name, in Phosphor's canonical order.
    ///
    /// Used by apps that want to enumerate the catalog (e.g. an icon grid);
    /// 1,512 names.
    pub fn all_names() -> impl Iterator<Item = &'static str> {
        BY_NAME.iter().map(|(name, _)| *name)
    }

    /// This icon in the given weight.
    pub const fn weight(self, weight: Weight) -> Phosphor {
        Phosphor { icon: self, weight }
    }

    /// This icon in the thin weight.
    pub const fn thin(self) -> Phosphor {
        self.weight(Weight::Thin)
    }

    /// This icon in the light weight.
    pub const fn light(self) -> Phosphor {
        self.weight(Weight::Light)
    }

    /// This icon in the regular weight (the same as the bare icon).
    pub const fn regular(self) -> Phosphor {
        self.weight(Weight::Regular)
    }

    /// This icon in the bold weight.
    pub const fn bold(self) -> Phosphor {
        self.weight(Weight::Bold)
    }

    /// This icon in the fill weight.
    pub const fn fill(self) -> Phosphor {
        self.weight(Weight::Fill)
    }

    /// This icon in the duotone weight.
    pub const fn duotone(self) -> Phosphor {
        self.weight(Weight::Duotone)
    }

    /// The asset path of the regular weight, e.g. `phosphor/regular/gear.svg`.
    pub fn path(self) -> SharedString {
        self.regular().path()
    }
}

/// A Phosphor icon in a specific [`Weight`].
///
/// Build one with [`PhosphorIcon::weight`] or a shorthand such as
/// [`PhosphorIcon::bold`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Phosphor {
    pub icon: PhosphorIcon,
    pub weight: Weight,
}

impl Phosphor {
    pub const fn new(icon: PhosphorIcon, weight: Weight) -> Self {
        Self { icon, weight }
    }

    /// The asset path, e.g. `phosphor/bold/gear.svg`.
    pub fn path(self) -> SharedString {
        format!(
            "{PATH_PREFIX}{}/{}.svg",
            self.weight.as_str(),
            self.icon.name()
        )
        .into()
    }

    /// Parse an asset path produced by [`Phosphor::path`].
    pub fn from_path(path: &str) -> Option<Self> {
        let rest = path.strip_prefix(PATH_PREFIX)?.strip_suffix(".svg")?;
        let (weight, name) = rest.split_once('/')?;
        Some(Self::new(
            PhosphorIcon::from_name(name)?,
            Weight::from_name(weight)?,
        ))
    }

    /// The SVG bytes, if this icon's weight is enabled as a Cargo feature.
    ///
    /// Calling this refers to every icon of the enabled weights: use
    /// `Phosphor::from_path(..)`/`svg()` only inside the running app, never in a
    /// path you expect the linker to drop.
    pub fn svg(self) -> Option<&'static [u8]> {
        embedded_entry(self).map(|(_, bytes)| bytes)
    }
}

impl From<PhosphorIcon> for Phosphor {
    fn from(icon: PhosphorIcon) -> Self {
        icon.regular()
    }
}

#[allow(unused_variables, unreachable_patterns)]
fn embedded_entry(icon: Phosphor) -> Option<(&'static str, &'static [u8])> {
    match icon.weight {
        #[cfg(feature = "thin")]
        Weight::Thin => Some(thin(icon.icon)),
        #[cfg(feature = "light")]
        Weight::Light => Some(light(icon.icon)),
        #[cfg(feature = "regular")]
        Weight::Regular => Some(regular(icon.icon)),
        #[cfg(feature = "bold")]
        Weight::Bold => Some(bold(icon.icon)),
        #[cfg(feature = "fill")]
        Weight::Fill => Some(fill(icon.icon)),
        #[cfg(feature = "duotone")]
        Weight::Duotone => Some(duotone(icon.icon)),
        _ => None,
    }
}

/// Serves every icon of the weights enabled as Cargo features.
///
/// Combine it with GPUI Kit's own `Assets` (or any other source) using
/// [`PhosphorAssets::with`]:
///
/// ```ignore
/// gpui_platform::application()
///     .with_assets(nemo_phosphor::PhosphorAssets.with(gpui_kit::assets::Assets));
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct PhosphorAssets;

impl PhosphorAssets {
    /// Serve Phosphor icons first and fall back to `fallback` for other paths.
    pub fn with<B: AssetSource>(self, fallback: B) -> Layered<Self, B> {
        Layered(self, fallback)
    }
}

impl AssetSource for PhosphorAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok(Phosphor::from_path(path)
            .and_then(Phosphor::svg)
            .map(Cow::Borrowed))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        // Cheap rejection for the common case of listing another source's paths.
        if !(path.starts_with(PATH_PREFIX) || PATH_PREFIX.starts_with(path)) {
            return Ok(Vec::new());
        }
        Ok(Weight::ALL
            .into_iter()
            .filter(|weight| weight.is_embedded())
            .flat_map(|weight| {
                PhosphorIcon::ALL
                    .iter()
                    .map(move |icon| icon.weight(weight))
            })
            .map(Phosphor::path)
            .filter(|candidate| candidate.starts_with(path))
            .collect())
    }
}

/// Two asset sources layered together: `load` tries the first source, then the
/// second; `list` merges both.
#[derive(Clone, Copy, Debug, Default)]
pub struct Layered<A, B>(pub A, pub B);

impl<A, B> Layered<A, B> {
    /// Add another fallback below the existing ones.
    pub fn with<C: AssetSource>(self, fallback: C) -> Layered<Self, C> {
        Layered(self, fallback)
    }
}

impl<A: AssetSource, B: AssetSource> AssetSource for Layered<A, B> {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        match self.0.load(path)? {
            Some(bytes) => Ok(Some(bytes)),
            None => self.1.load(path),
        }
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let mut paths = self.0.list(path)?;
        paths.extend(self.1.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_complete_and_named() {
        assert_eq!(PhosphorIcon::ALL.len(), 1512);
        assert_eq!(PhosphorIcon::AddressBook.name(), "address-book");
        for &icon in PhosphorIcon::ALL {
            assert_eq!(PhosphorIcon::from_name(icon.name()), Some(icon));
        }
        assert_eq!(PhosphorIcon::from_name("not-an-icon"), None);
    }

    #[test]
    fn paths_round_trip() {
        assert_eq!(
            PhosphorIcon::Acorn.path().to_string(),
            "phosphor/regular/acorn.svg"
        );
        assert_eq!(
            PhosphorIcon::ArrowUpRight.weight(Weight::Bold).path(),
            PhosphorIcon::ArrowUpRight.bold().path()
        );
        assert_eq!(
            Phosphor::from_path("phosphor/duotone/gear.svg"),
            Some(PhosphorIcon::Gear.duotone())
        );
    }

    #[test]
    fn weight_parsing() {
        for weight in Weight::ALL {
            assert_eq!(Weight::from_name(weight.as_str()), Some(weight));
        }
        assert_eq!(Weight::from_name("heavy"), None);
    }

    #[test]
    fn svg_bytes_available_when_embedded() {
        // The default feature set embeds every weight.
        assert!(PhosphorIcon::Gear.fill().svg().is_some());
        assert!(PhosphorIcon::Gear.thin().svg().is_some());
        // Unknown weight combos cannot exist by construction.
        assert_eq!(
            Phosphor::from_path("phosphor/thin/acorn.svg"),
            Some(PhosphorIcon::Acorn.thin())
        );
    }
}
