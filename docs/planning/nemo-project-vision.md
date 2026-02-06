# Nemo: Project Vision and Scope

> **Project Name:** Nemo ("no one" in Latin)  
> **Status:** Initial Architecture Definition  
> **Last Updated:** 2026-02-05

## Executive Summary

Nemo is a **meta-application framework**—a Rust-based desktop application that doesn't do one specific thing, but instead provides the infrastructure to construct *any* application through configuration. Built on GPUI and gpui-component, Nemo offers:

- **Configuration-driven UI composition** using HCL schemas
- **Dynamic layout construction** from configuration files
- **Unified data flow architecture** connecting collection, storage, action, and display
- **Multi-modal extensibility** via RHAI scripting and native plugin loading
- **Third-party integration** through RPC, PubSub, and message queue systems

The result is a platform where application authors describe *what* they want rather than *how* to build it, while retaining escape hatches into code when needed.

---

## Problem Statement

Desktop application development, even with modern toolkits, requires significant boilerplate for:

1. **UI construction** — Defining views, layouts, and component trees
2. **Data wiring** — Connecting data sources to displays and user actions to mutations
3. **Extension points** — Allowing third-party or advanced users to customize behavior
4. **Integration** — Communicating with external systems, services, and processes

Current approaches force developers to either:
- Write full applications in code (high effort, full control)
- Use rigid no-code tools (low effort, limited capability)

Nemo occupies the middle ground: **high configurability with code-level escape velocity**.

---

## Core Design Principles

### 1. Configuration as the Primary Interface

The HCL configuration language serves as the main interface for application authors. Configuration files define:
- Application structure and layout
- Component instantiation and properties
- Data bindings and transformations
- Event handlers and actions
- Extension loading and initialization

### 2. Schema-Driven Everything

Every configurable aspect has a schema. Schemas provide:
- **Validation** — Catch errors before runtime
- **Documentation** — Self-describing configuration
- **Tooling support** — IDE completion, linting, visualization
- **Versioning** — Migration paths between schema versions

### 3. Loose Coupling, Strong Contracts

Components communicate through well-defined interfaces:
- Data flows through typed channels
- Events follow documented schemas  
- Extensions implement trait contracts
- External integrations use standard protocols

### 4. Progressive Complexity

Simple applications should be simple to configure. Complex applications should be *possible*:
- **Level 1:** HCL configuration only
- **Level 2:** HCL + RHAI scripts for dynamic logic
- **Level 3:** HCL + RHAI + Native plugins for performance-critical extensions

### 5. Observability by Default

All data flow, events, and state changes are observable:
- Configuration-defined logging and metrics
- Debuggable data pipelines
- Introspectable component trees

---

## Target Use Cases

### Primary Use Cases

1. **Internal Tools** — Business-specific dashboards, admin panels, data entry forms
2. **Data Visualization Applications** — Configurable displays of streaming or static data
3. **Development Tools** — IDE panels, log viewers, API explorers
4. **Kiosk Applications** — Single-purpose displays with controlled interaction

### Secondary Use Cases

1. **Prototyping** — Rapidly explore UI layouts and data flows
2. **Plugin Hosts** — Applications that primarily load third-party extensions
3. **Automation Interfaces** — Human-in-the-loop for automated systems

---

## Technology Stack

| Layer | Technology | Rationale |
|-------|------------|-----------|
| UI Framework | GPUI | High-performance, GPU-accelerated, Rust-native |
| Component Library | gpui-component | 60+ production-ready components, dock layout, theming |
| Configuration Language | HCL | Human-readable, supports expressions, better than JSON/YAML for complex config |
| Scripting Engine | RHAI | Rust-native, safe, sandboxed, good FFI |
| Native Plugins | libloading | Standard approach for dynamic library loading in Rust |
| Serialization | serde + various | HCL, JSON, MessagePack for different use cases |

---

## Success Criteria

### For Application Authors

- Can create a functional application in <100 lines of HCL
- Can extend with RHAI without touching Rust
- Can integrate external data sources without custom code
- Configuration errors are clear and actionable

### For Platform Developers

- Clear subsystem boundaries enable parallel development
- New components can be added without core changes
- Schema evolution is manageable
- Performance meets desktop application standards

### For End Users (of Nemo-built applications)

- Applications are responsive and native-feeling
- No perceptible "configuration overhead" at runtime
- Applications behave consistently and predictably

---

## Out of Scope (Initial Version)

- Web deployment (GPUI is desktop-only)
- Mobile platforms
- Visual configuration editor (future enhancement)
- Marketplace for configurations/extensions
- Multi-window applications (defer to later)
- Accessibility features beyond GPUI defaults

---

## Document History

| Date | Author | Change |
|------|--------|--------|
| 2026-02-05 | systems-designer | Initial creation |
