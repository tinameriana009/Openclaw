# Repo analysis operator session template

Use this while reviewing a repo-analysis run so evidence, doubts, and follow-up asks survive handoff.

## Environment
- Date/time (UTC):
- Operator:
- Repo under analysis:
- Corpus roots attached:
- Profile used: balanced / deep / research
- Session/run artifact path:
- Queue state when you picked it up: queued / claimed / in-review / deferred / handoff-ready / completed / dropped
- continuity-status.json reviewed? yes / no
- operator-transition-brief.md inherited? yes / no

## Prompt flow used
- Initial brief prompt:
- Follow-up prompt(s):
- Did you resume the same session? yes / no
- If this is a later pass, which prior run bundle or dashboard did you inherit?
- Which prior reviewed run are you comparing against?

## Grounding review
- Files the answer cited confidently:
- Files it should have cited but missed:
- Tests or reference-data surfaces mentioned:
- Places where the answer distinguished facts from inferences well:
- Places where the answer sounded overconfident:
- Claims that changed versus the prior reviewed run:

## Validation notes
- Entry point findings:
- Query/runtime/registry findings:
- Risky files and why:
- Reading order quality:
- Divergence from expected-findings.md:

## Trace review
- Trace file(s) inspected:
- Important missing evidence:
- Broad claims that need re-checking:
- If you replayed or resumed a saved trace, what changed between passes?
- Which trace or approval packet should the next operator inspect first?

## Handoff payload
- Exact follow-up question to ask next:
- File paths to force into the next prompt:
- Evidence snippets or quotes worth carrying forward:
- What the next operator can trust without re-reading everything:
- What the next operator must still verify manually:
