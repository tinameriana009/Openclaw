# Repo analysis demo brief

## Goal

Onboard a new engineer into the Python parity workspace quickly enough that they can:

- find the CLI entrypoint
- understand where query routing and runtime session bootstrapping live
- identify the registry/snapshot surfaces that mirror commands and tools
- recognize which files are safest to read first versus riskiest to edit

## Corpus

Attach:

- `src/`
- `tests/`

## Suggested first prompt

```text
Analyze the attached repository for a new engineer.
Ground the answer in the local corpus and be explicit about uncertainty.

Output:
1. Top-level system summary
2. Likely entrypoints
3. Main subsystems or packages
4. Important generated/reference data or persisted artifacts
5. High-risk areas for change
6. Suggested reading order
```

## Suggested follow-up prompt

```text
Trace the path from the CLI entrypoint through query routing, runtime/bootstrap state, and execution registry selection. Name the files involved and explain what each contributes.
```
