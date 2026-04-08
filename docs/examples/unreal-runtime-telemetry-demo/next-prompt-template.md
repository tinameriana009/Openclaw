# Unreal follow-up prompt template

Use this after a real Unreal build/editor/runtime validation pass.

The point is to convert exact engine evidence into a tighter next prompt instead of re-asking for the whole plugin from scratch.

```text
You are helping continue an Unreal Engine plugin workflow.
Ground your answer in the local plugin files and the operator evidence below.
Distinguish facts from inferences.
Prefer the smallest plausible fix before suggesting broader rewrites.

Task:
[describe the specific fix, feature, or investigation]

Environment:
- Unreal version: [version]
- OS: [platform]
- Validation path: IDE build / UBT / editor compile / runtime test
- Project path/context: [project or disposable test project]

Observed validation results:
- Compile result: [success/failure]
- Plugin enable/load result: [success/failure]
- Blueprint node visibility result: [visible/not visible/not tested]
- Runtime behavior observed: [what actually happened]
- Divergence from expected-findings.md: [notes]

Relevant evidence:
- Exact compiler / UHT / editor log lines:
  [paste exact text]
- Files most likely involved:
  [list files]
- Operator findings summary:
  [paste concise notes]

Output:
1. Most likely root cause ranked by confidence
2. Smallest code/config patch to try next
3. Any engine/build assumptions that still need manual confirmation
4. A re-test checklist for the next compile/editor/runtime pass
```

## Handoff rule

If another operator picks this up, pass them:
- `manual-validation-checklist.md`
- `expected-findings.md`
- `error-feedback-playbook.md`
- your filled `operator-session-template.md`
- your filled `operator-findings-template.md`
- the exact logs/errors you pasted into the next prompt
