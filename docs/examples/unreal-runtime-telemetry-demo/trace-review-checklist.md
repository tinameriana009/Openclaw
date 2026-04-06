# Trace review checklist: Unreal runtime telemetry demo

Use this when the answer sounds confident but you want to verify whether it was grounded in the local plugin corpus.

## What to look for in the trace

- did the trace actually touch `RuntimeTelemetry.uplugin`?
- did it inspect `RuntimeTelemetry.Build.cs` before making dependency claims?
- did it read both the subsystem and Blueprint library files before assigning responsibilities?
- did it cite local file evidence before recommending an Editor module split?
- did it admit uncertainty where the corpus stops?

## Red flags

- claims about automated Unreal Editor launch or hot reload with no supporting evidence
- detailed packaging or marketplace guidance not grounded in the attached corpus
- assertions about reflection correctness without reference to the actual headers/macros
- dependency recommendations that ignore the existing `Build.cs`
- broad claims about file export paths when the example only logs/flushes conservatively

## Good follow-up prompts after trace review

- `Only use facts supported by the attached plugin files. Rewrite the architecture summary.`
- `List which claims came from RuntimeTelemetry.uplugin versus Build.cs versus the subsystem headers.`
- `Narrow the recommendation to the smallest Runtime-only implementation slice.`
- `Turn the uncertain areas into an engine-validation checklist instead of guessing.`
