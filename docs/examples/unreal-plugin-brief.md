# Example brief: Unreal runtime telemetry plugin

## Goal

Create an Unreal plugin that records lightweight runtime telemetry events and writes them to a local export path for debugging.

## Constraints

- target Unreal 5.x
- runtime-first, editor support optional
- keep module count low unless there is a strong reason to split
- assume manual compile/editor validation outside the harness

## Good first request

```text
Plan an Unreal runtime telemetry plugin. Propose the `.uplugin`, module layout, Build.cs starting point, first source files, and the main risks around engine integration or dependencies.
```
