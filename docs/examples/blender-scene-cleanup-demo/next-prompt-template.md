# Blender follow-up prompt template

Use this after you complete a real Blender validation pass.

The goal is to hand the next model turn **exactly what changed, what failed, and what still needs confirmation** instead of a vague "please fix it" request.

```text
You are helping continue a Blender add-on workflow.
Ground your answer in the local add-on files plus the operator findings below.
Distinguish facts from guesses.
Prefer the smallest patch that explains the observed behavior.

Task:
[describe the specific fix or extension you want]

Environment:
- Blender version: [version]
- OS: [platform]
- Install path: zip / copied package / repo checkout

Observed validation results:
- Registration result: [success/failure + exact traceback if any]
- Duplicate materials count observed: [count]
- Unapplied transforms count (hidden disabled): [count]
- Unapplied transforms count (hidden enabled): [count]
- Matches validation-baseline.md: [yes/no + how it differs]
- UI wording/layout issues: [notes]

Relevant evidence:
- Exact traceback or console output:
  [paste exact text]
- Files most likely involved:
  [list files]
- Operator findings summary:
  [paste concise notes]

Output:
1. Likely root cause ranked by confidence
2. Smallest patch or edit plan
3. Any Blender API assumptions still requiring manual validation
4. A re-test checklist for the next Blender pass
```

## Handoff rule

If another operator picks this up, pass them:
- `manual-test-checklist.md`
- `validation-baseline.md`
- your filled `operator-findings-template.md`
- the exact traceback or mismatched counts
- the next prompt you actually used
