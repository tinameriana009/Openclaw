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
- Files already spot-checked manually: [src/main.py, src/runtime.py, ...]

What I verified manually:
- Confirmed facts: [list exact file-backed facts]
- Missing or weakly supported claims: [list]
- Important files/tests the last answer missed: [list]
- Trace review findings: [list]
- If this is a replay/resume pass, what changed since the last answer: [list]

Task:
[ask one narrow follow-up question]

Constraints:
- Cite the specific files that support each conclusion.
- Distinguish facts from inferences.
- If evidence is thin or conflicting, say so.
- End with the next 3 files or tests I should inspect manually.

Output:
1. Answer to the narrow question
2. Evidence by file
3. Uncertainties / conflicting signals
4. Recommended next manual checks
```
