# Manual validation checklist: Unreal runtime telemetry demo

Use this after the harness helps you plan or edit the plugin. The point is to close the loop with **real engine validation**, not to trust a plausible answer blindly.

## Before opening Unreal

- [ ] Run `python3 tests/validate_unreal_demo.py` so the demo kit itself is intact.
- [ ] Read `expected-findings.md` so you know the baseline plugin structure.
- [ ] Copy `plugin/RuntimeTelemetry/` into a disposable Unreal 5.x project or compare it against an existing plugin repo.
- [ ] Decide whether you are validating with IDE build, UnrealBuildTool, or editor compile flow.

## Compile / project validation

- [ ] Generate project files if your setup requires it.
- [ ] Build the project or plugin.
- [ ] Confirm there are no missing module dependency errors from `Build.cs`.
- [ ] Confirm Unreal Header Tool does not report reflection or macro issues.
- [ ] If compilation fails, capture the exact error text for the next prompt instead of paraphrasing it.

## Editor validation

- [ ] Enable the plugin in the project if it is not already enabled.
- [ ] Restart the editor if Unreal requests it after enabling the plugin.
- [ ] Confirm the module loads without startup errors.
- [ ] Confirm the `URuntimeTelemetrySubsystem` class appears as expected through normal engine discovery paths.
- [ ] Confirm Blueprint nodes from `UTelemetryBlueprintLibrary` are visible if the library is meant to be Blueprint-facing.

## Runtime behavior validation

- [ ] Create a minimal map or test actor that records at least one telemetry event.
- [ ] Trigger the event in PIE or standalone play.
- [ ] Confirm the subsystem buffer changes as expected.
- [ ] Confirm flush/log behavior matches the implementation comments and does not silently claim unsupported export behavior.
- [ ] Check that repeated event recording does not obviously break lifecycle assumptions.

## Operator review questions

- [ ] Did the model distinguish facts from guesses about Unreal internals?
- [ ] Did it keep the first slice Runtime-only unless there was evidence for an Editor split?
- [ ] Did it call out uncertainty around file output paths, packaging, or reflection edge cases?
- [ ] Did it avoid pretending the CLI can run Unreal or validate the plugin automatically?

## Good evidence to feed back into the next prompt

- full compiler or Unreal Header Tool errors
- screenshot text or copied log lines from plugin enable/load failures
- the exact Blueprint node that is missing or misnamed
- the specific lifecycle step where subsystem behavior diverged from expectations
