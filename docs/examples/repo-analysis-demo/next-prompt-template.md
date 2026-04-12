# Next prompt template: repo analysis demo

Use this after you have reviewed the prior answer and manually checked a few files.
Replace placeholders with exact evidence.

```text
Continue the grounded repo-analysis session.
Use the same attached local corpus and stay explicit about uncertainty.

Context from the previous run:
- Profile used: [balanced/deep/research]
- Prior artifacts reviewed: [.demo-artifacts/repo-analysis-demo/<timestamp>/...]
- Dashboard or handoff bundle reviewed: [operator-dashboard.html / bundle-summary.json / operator-handoff.json]
- Continuity state reviewed: [review-status.json / continuity-status.json / operator-transition-brief.md]
- Files already spot-checked manually: [src/main.py, src/runtime.py, ...]
- Prior reviewed run compared against: [bundle-summary.priorReviewedRun.runId or none]
- Current handoff state: [continuity-status.handoffState]

What I verified manually:
- Confirmed facts: [list exact file-backed facts]
- Missing or weakly supported claims: [list]
- Important files/tests the last answer missed: [list]
- Trace review findings: [list]
- If this is a replay/resume pass, what changed since the last answer: [list]
- What the next operator should *not* assume yet: [list]

Task:
[ask one narrow follow-up question]

Constraints:
- Cite the specific files that support each conclusion.
- Distinguish facts from inferences.
- If evidence is thin or conflicting, say so.
- Call out what changed versus the prior reviewed pass.
- End with the next 3 files or tests I should inspect manually.

Output:
1. Answer to the narrow question
2. Evidence by file
3. Uncertainties / conflicting signals
4. What changed from the prior pass
5. Recommended next manual checks
```
