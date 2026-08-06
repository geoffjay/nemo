# Patterns

Recurring implementation patterns used in the project.

* [Four-file component creation workflow](four-file-component-workflow.md) - the four files touched when adding a built-in component.
* [Stateful widget Entity persistence](stateful-widget-entity-persistence.md) - persist widget state in ComponentStates keyed by ID, with data-change detection.
* [Definite height for uniform_list widgets](definite-height-for-lists.md) - Table/Tree collapse to 0px without a definite parent height.
* [Collection properties as JSON-string attributes](json-string-collection-properties.md) - which components take arrays/objects as a JSON-string attribute, and how coerce_value handles them.
* [Parent-rendered child components](parent-rendered-child-components.md) - how a parent reads and renders its typed child components, vs. generic render_children.
* [Layout sizing and centering](layout-sizing-and-centering.md) - the minimal style surface: `flex` ignores its value, no percent/align, and how to center a fixed-width panel.
* [Single-file components](single-file-components.md) - authoring reusable `.nemo` components (template + scoped script + `${prop}` interpolation + default slot), imported and used as a custom tag.
* [Runtime component creation](runtime-component-creation.md) - create/remove built-in component instances at runtime via Rhai and PluginContext (create_component, update_component, remove_component).
