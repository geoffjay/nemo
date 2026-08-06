//! Settings-overlay persistence.
//!
//! Project-level user preferences (today: the theme `name` and `mode` chosen
//! in the settings UI) are written to an `overrides.xml` file sitting next to
//! the entry (`app.nemo` or `app.xml`), not edited into the entry itself. This
//! keeps the source entry immutable regardless of format — important for
//! `.nemo` SFCs, where a text edit into raw-text `<style>`/`<script>` blocks
//! would be fragile. See the
//! [settings-overlay decision](../../docs/knowledgebase/decisions/settings-overrides-xml.md).
//!
//! The overlay is a tiny plain-XML document (`<nemo><app><theme …/></app></nemo>`)
//! parsed by the existing `load_xml_string` path; the runtime merges its `app`
//! key over the entry's at load time.

use std::io;
use std::path::Path;

/// The overlay filename, looked for next to the entry file.
pub const OVERRIDES_FILE: &str = "overrides.xml";

/// Persist the theme `name` and `mode` into the `overrides.xml` next to the
/// given entry path (`app.nemo` or `app.xml`).
///
/// - If `overrides.xml` already exists, only its `<theme>` element's `name`/
///   `mode` attributes are updated (other attributes/children preserved).
/// - Otherwise a new `overrides.xml` is created with a self-closing
///   `<theme name mode />` under `<app>`.
///
/// The entry file itself is never mutated.
pub fn set_app_theme(entry_path: &Path, name: &str, mode: &str) -> io::Result<()> {
    let overlay_path = entry_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(OVERRIDES_FILE);
    let existing = std::fs::read_to_string(&overlay_path).unwrap_or_default();
    let updated = set_theme_in_xml(&existing, name, mode)?;
    std::fs::write(&overlay_path, updated)
}

/// Pure string transform behind [`set_app_theme`], separated out for testing.
pub fn set_theme_in_xml(content: &str, name: &str, mode: &str) -> io::Result<String> {
    if let Some((start, gt)) = find_opening_tag(content, "theme") {
        let tag = &content[start..=gt];
        let self_closing = tag.trim_end().ends_with("/>");

        // Slice out just the attributes portion of the opening tag.
        let attrs_start = start + "<theme".len();
        let attrs_end = if self_closing {
            // Trim back over the trailing "/>".
            content[..gt].rfind('/').unwrap_or(gt)
        } else {
            gt
        };
        let attrs = &content[attrs_start..attrs_end];

        let mut pairs = parse_attributes(attrs);
        set_attr(&mut pairs, "name", name);
        set_attr(&mut pairs, "mode", mode);

        let mut rebuilt = String::from("<theme");
        for (k, v) in &pairs {
            rebuilt.push_str(&format!(" {}=\"{}\"", k, v));
        }
        rebuilt.push_str(if self_closing { " />" } else { ">" });

        let mut out = String::with_capacity(content.len() + rebuilt.len());
        out.push_str(&content[..start]);
        out.push_str(&rebuilt);
        out.push_str(&content[gt + 1..]);
        return Ok(out);
    }
    // No existing <theme>: insert one after the <app ...> opening tag, or
    // synthesize a fresh overlay document when the content is empty/has no
    // <app> element (e.g. first-ever settings write).
    let insertion = format!("\n    <theme name=\"{}\" mode=\"{}\" />", name, mode);
    if let Some((_app_start, app_gt)) = find_opening_tag(content, "app") {
        let mut out = String::with_capacity(content.len() + insertion.len());
        out.push_str(&content[..=app_gt]);
        out.push_str(&insertion);
        out.push_str(&content[app_gt + 1..]);
        return Ok(out);
    }

    // No <app> at all (empty or malformed content): create a minimal overlay.
    Ok(format!("<nemo>\n  <app>\n{insertion}\n  </app>\n</nemo>\n"))
}

/// Find the opening tag `<{tag}` for an element, returning the byte index of the
/// `<` and the byte index of the closing `>` of that opening tag.
///
/// Matches only when `<{tag}` is followed by whitespace, `/`, or `>` so that
/// e.g. searching for `theme` does not match `<theme-picker`.
fn find_opening_tag(content: &str, tag: &str) -> Option<(usize, usize)> {
    let needle = format!("<{}", tag);
    let mut from = 0;
    while let Some(rel) = content[from..].find(&needle) {
        let start = from + rel;
        let after = start + needle.len();
        let next = content[after..].chars().next();
        match next {
            Some(c) if c.is_whitespace() || c == '/' || c == '>' => {
                if let Some(gt_rel) = content[start..].find('>') {
                    return Some((start, start + gt_rel));
                }
                return None;
            }
            _ => {
                from = after;
            }
        }
    }
    None
}

/// Parse `key="value"` pairs from an attribute string, preserving order.
fn parse_attributes(attrs: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let bytes = attrs.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Skip whitespace.
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        // Read key up to '='.
        let key_start = i;
        while i < bytes.len() && bytes[i] != b'=' && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if key_start == i {
            break;
        }
        let key = attrs[key_start..i].to_string();
        // Skip whitespace and '='.
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'=') {
            i += 1;
        }
        // Expect a quote.
        if i >= bytes.len() || (bytes[i] != b'"' && bytes[i] != b'\'') {
            // Attribute without a value; record empty and continue.
            pairs.push((key, String::new()));
            continue;
        }
        let quote = bytes[i];
        i += 1;
        let val_start = i;
        while i < bytes.len() && bytes[i] != quote {
            i += 1;
        }
        let value = attrs[val_start..i].to_string();
        if i < bytes.len() {
            i += 1; // consume closing quote
        }
        pairs.push((key, value));
    }
    pairs
}

/// Update an attribute's value in place, or append it if absent.
fn set_attr(pairs: &mut Vec<(String, String)>, key: &str, value: &str) {
    if let Some(pair) = pairs.iter_mut().find(|(k, _)| k == key) {
        pair.1 = value.to_string();
    } else {
        pairs.push((key.to_string(), value.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_existing_theme_attrs() {
        let xml = r#"<nemo>
  <app title="Demo">
    <theme name="kanagawa" mode="dark" />
    <header-bar />
  </app>
</nemo>"#;
        let out = set_theme_in_xml(xml, "nord", "light").unwrap();
        assert!(out.contains(r#"<theme name="nord" mode="light" />"#));
        assert!(!out.contains("kanagawa"));
        // Rest of the file is untouched.
        assert!(out.contains(r#"<app title="Demo">"#));
        assert!(out.contains("<header-bar />"));
    }

    #[test]
    fn preserves_other_theme_attrs_and_order() {
        let xml = r#"<app><theme mode="dark" name="nord" extra="x" /></app>"#;
        let out = set_theme_in_xml(xml, "gruvbox", "light").unwrap();
        // Original ordering (mode, name, extra) is preserved; values updated.
        assert_eq!(
            out,
            r#"<app><theme mode="light" name="gruvbox" extra="x" /></app>"#
        );
    }

    #[test]
    fn inserts_theme_when_missing() {
        let xml = r#"<nemo>
  <app title="Demo">
    <header-bar />
  </app>
</nemo>"#;
        let out = set_theme_in_xml(xml, "tokyo night", "system").unwrap();
        assert!(out.contains(r#"<theme name="tokyo night" mode="system" />"#));
        // The theme is inserted inside <app>, before the existing child.
        let theme_idx = out.find("<theme").unwrap();
        let header_idx = out.find("<header-bar").unwrap();
        assert!(theme_idx < header_idx);
        assert!(out.contains(r#"<app title="Demo">"#));
    }

    #[test]
    fn does_not_match_similarly_named_elements() {
        let xml = r#"<app><theme-picker name="x" /><theme name="a" mode="dark" /></app>"#;
        let out = set_theme_in_xml(xml, "nord", "light").unwrap();
        assert!(out.contains(r#"<theme-picker name="x" />"#));
        assert!(out.contains(r#"<theme name="nord" mode="light" />"#));
    }

    #[test]
    fn synthesizes_overlay_when_no_app_element() {
        // Empty content (first-ever settings write): synthesize a fresh overlay.
        let out = set_theme_in_xml("", "nord", "dark").unwrap();
        assert!(out.contains(r#"<theme name="nord" mode="dark" />"#));
        assert!(out.contains("<app>"));
        assert!(out.contains("</nemo>"));

        // Content with no <app> element: also synthesizes a fresh overlay.
        let xml = r#"<nemo><layout /></nemo>"#;
        let out = set_theme_in_xml(xml, "nord", "dark").unwrap();
        assert!(out.contains(r#"<theme name="nord" mode="dark" />"#));
        assert!(out.contains("<app>"));
    }

    #[test]
    fn handles_open_theme_tag_with_children() {
        let xml = r##"<app>
    <theme name="nord" mode="dark">
      <extend background="#000000" />
    </theme>
  </app>"##;
        let out = set_theme_in_xml(xml, "gruvbox", "light").unwrap();
        // Opening tag updated, still non-self-closing, children preserved.
        assert!(out.contains(r#"<theme name="gruvbox" mode="light">"#));
        assert!(out.contains(r##"<extend background="#000000" />"##));
        assert!(out.contains("</theme>"));
    }
}
