# Plans

Forward-looking plans for the project.

* [Roadmap](roadmap.md) - current capabilities, phase-2 status, remaining roadmap items, and pointers to full planning docs.
* [Declarative children over JSON-string properties](declarative-children-migration.md) - migrate collection components from JSON-string attributes to nested child elements, piloted on accordion.
* [Headless renderer and screenshots](headless-screenshots.md) - implemented on macOS via gpui's offscreen `Window::render_to_image` (`nemo screenshot`); Linux capture remains open.
* [Devtools inspector](devtools-inspector.md) - what a nemo-devtools crate would take; the introspection surfaces already exist, in-process panel recommended over an external client.
* [Design tokens and active redesign](design-tokens.md) - centralized spacing/radius/typography/semantic-color tokens in `theme/tokens.rs`; incremental component migration with screenshot verification.
