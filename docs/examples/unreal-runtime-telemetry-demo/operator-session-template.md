# Unreal operator session template

Fill this in while you validate the demo plugin in a real project. This is meant to keep the Unreal workflow evidence-driven instead of memory-driven.

## Session setup

- Date/time:
- Operator:
- Unreal version:
- OS:
- Project used for validation:
- Validation path: IDE build / UBT / editor compile
- Did you modify the staged plugin before testing? If yes, which files?

## Preflight

- [ ] Ran `python3 tests/validate_unreal_demo.py`
- [ ] Read `expected-findings.md`
- [ ] Reviewed `manual-validation-checklist.md`
- [ ] Kept `error-feedback-playbook.md` ready for follow-up prompts

## Compile evidence

- Build command or path used:
- Result:
- Exact compiler/UHT errors:

## Editor evidence

- Plugin enabled successfully? yes/no
- Module load result:
- Subsystem discovery result:
- Blueprint node visibility result:
- Exact relevant log lines:

## Runtime evidence

- How was telemetry triggered?
- Buffered event count observed:
- Flush/log behavior observed:
- Unexpected behavior:

## Next prompt payload

Paste the exact evidence you plan to feed back into `claw`:

```text
[paste here]
```

## Notes

- Keep exact names (`URuntimeTelemetrySubsystem`, `UTelemetryBlueprintLibrary`, module names, filenames) intact.
- Prefer copied errors over summaries.
- If the first compile fails, do not broaden scope yet.
