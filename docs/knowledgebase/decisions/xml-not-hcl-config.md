---
type: Decision
title: XML is the configuration format (not HCL)
description: Applications are configured in XML; HCL is not implemented.
tags: [config, xml, decision]
timestamp: 2026-07-11T00:00:00Z
---

# Decision

> **Superseded for the application entry** by [The application entry is a
> `.nemo` SFC](app-nemo-sfc-entry.md): the entry is now an `app.nemo` SFC and
> `app.xml` entries are rejected. XML lives on only inside `<include>` fragments
> and the machine-written `overrides.xml` settings overlay. The core of this
> decision — there is **no HCL** parser or loader — still holds.

# Context

Early design notes and some archived agent docs referenced HCL, and a few code
comments describe XML syntax by analogy to a hypothetical HCL form (e.g.
`crates/nemo-config/src/xml_parser.rs`). This left conflicting signals about the
config format. In the actual implementation, the loader
(`crates/nemo-config/src/loader.rs`) and parser
(`crates/nemo-config/src/xml_parser.rs`, quick-xml) handle XML only, and every
example under `examples/*/app.xml` is XML.

# Consequences

* Treat XML as the single config format. Do not add or document HCL without an
  explicit new decision superseding this one.
* Config authoring, schema validation, and the `<include>`/`<templates>`/
  `<variable>` mechanisms all live in the XML pipeline. See
  [Configuration](../concepts/configuration.md).
