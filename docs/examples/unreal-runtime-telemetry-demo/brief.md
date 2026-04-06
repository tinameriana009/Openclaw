# Brief: Unreal runtime telemetry plugin

## Scenario

You are helping plan and extend a small Unreal Engine plugin called `RuntimeTelemetry`.

The plugin should collect lightweight gameplay/runtime events and expose a conservative API for recording them to a local log/export path during development.

## Constraints

- target Unreal Engine 5.x
- runtime-first; editor tooling is optional and should stay out of the first slice
- prefer one runtime module unless there is a strong reason to split
- keep public API surface small
- do not assume the harness can compile, launch, or hot-reload Unreal for the operator
- explicitly separate static code review from real build/editor validation

## Good first request

```text
Use the attached Unreal demo corpus to explain the plugin layout, the intended runtime telemetry workflow, the likely Build.cs dependency choices, and the next implementation slice. Distinguish facts from inference.
```

## Good follow-up requests

- `List the first files I should edit to add a new telemetry event type.`
- `Review whether this plugin should stay Runtime-only or split out an Editor module later.`
- `Summarize the reflection and module-load risks I should validate in the engine.`
- `Turn this compile error into a probable root-cause checklist.`
