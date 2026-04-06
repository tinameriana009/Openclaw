# Expected findings for the Unreal runtime telemetry demo

A grounded answer against this demo corpus should usually recover most of these facts:

## Core plugin shape

- the demo plugin is named `RuntimeTelemetry`
- the plugin descriptor is `plugin/RuntimeTelemetry/RuntimeTelemetry.uplugin`
- it currently defines a single `Runtime` module named `RuntimeTelemetry`
- the module loading phase is `Default`

## Important source files

- `plugin/RuntimeTelemetry/Source/RuntimeTelemetry/RuntimeTelemetry.Build.cs`
- `plugin/RuntimeTelemetry/Source/RuntimeTelemetry/Public/RuntimeTelemetrySubsystem.h`
- `plugin/RuntimeTelemetry/Source/RuntimeTelemetry/Private/RuntimeTelemetrySubsystem.cpp`
- `plugin/RuntimeTelemetry/Source/RuntimeTelemetry/Public/TelemetryBlueprintLibrary.h`
- `plugin/RuntimeTelemetry/Source/RuntimeTelemetry/Private/TelemetryBlueprintLibrary.cpp`
- `plugin/RuntimeTelemetry/Source/RuntimeTelemetry/Private/RuntimeTelemetryModule.cpp`

## Architecture facts

- the plugin uses a `UGameInstanceSubsystem` as the main runtime anchor
- the subsystem owns a small in-memory event buffer and a flush helper
- the Blueprint-facing API delegates to the subsystem instead of owning storage itself
- the Build.cs starts conservatively with `Core`, `CoreUObject`, `Engine`, and `Projects`
- the example intentionally avoids claiming file I/O or editor integration is production-complete

## Operator validation expectations

- real validation still requires UnrealBuildTool or IDE compilation
- operators should validate plugin enablement in a disposable project
- operators should confirm module load, Blueprint node visibility, and runtime log/output behavior manually
- any answer that promises editor launch, hot reload, packaging, or reflection correctness without manual checks is overstating current capability
