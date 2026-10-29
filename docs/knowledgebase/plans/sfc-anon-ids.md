# SFC templates require explicit child ids (`__anon_N` collision)

## Symptom
An SFC page template (`component-page`) whose children have **no `id`**
attributes renders mangled once the page is instantiated with slotted
children: chrome (title/description/panels) dropped or reordered, the first
default-slot child escaping its slot, later default-slot children missing,
and named-slot content (e.g. a properties `<table>`) rendering as an empty
sliver.

## Mechanism
`XmlParser` gives id-less component children auto ids `__anon_1`, `__anon_2`,
… (parser-global counter, `crates/nemo-config/src/xml_parser.rs:339`). The
counter is per-*parse*, so every instance of the same template carries the
same `__anon_N` keys. Expansion flattens components into an id-keyed map, and
nested SFC composition (`component-page-button` → `component-page` →
`component-example`) re-emits the same keys — the map collapses (last insert
wins) and chrome/slot layout breaks. Template instances with ids everywhere
work because `scope_template_children` prefixes each instance's own ids
(`instance_id` → `page_button.page_title` style keys).

`examples/sfc/card.nemo` dodged this because its stacks carry ids
(`head`/`body`); `labeled-button.nemo` (ids, no slots) also fine. The
combination "no ids + slots + multi-level composition" is what breaks.

## Rule
**Every child of an SFC `<template>` body must declare `id="..."`** — root
stack, labels, tabs, tab-items, slot containers? no: the `<slot>` marker is
stripped before flattening, but its *parent stack* needs the id. Prefer stable,
meaningful ids (`page_title`, `example_preview`) since they become visible
per-instance scoped ids used for lookups in scripts/styles.

## Status
- Workaround applied (2026-10-29): `examples/components/templates/component.nemo`
  and `component-example.nemo` got ids; gallery `/button` page verified
  rendering (title, description inside layout, all three examples, properties
  table, callbacks).
- Engine-level fix (deduping/generating anon ids per template *instance* at
  expansion time, instead of per parse) is a potential future improvement;
  tracked here. Do not author id-less SFC templates until then.