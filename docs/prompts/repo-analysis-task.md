# Prompt template: repository analysis task

Fill in the placeholders and paste into `claw prompt ...` or a REPL session.

```text
Analyze the attached repository and ground the answer in the local corpus.
If something is uncertain, say what evidence is missing.

Focus:
[architecture | feature trace | bug hunt | onboarding | refactor planning]

Question:
[Describe exactly what you want to understand]

Constraints:
- Prefer file-level evidence over vague summaries
- Distinguish facts from inferences
- Keep the answer actionable

Please produce:
1. A concise answer to the question
2. Key files and why they matter
3. Important dependencies or subsystem boundaries
4. Risks or ambiguities
5. Recommended next questions or next reads
```
