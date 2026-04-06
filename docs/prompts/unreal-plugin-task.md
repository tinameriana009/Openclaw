# Prompt template: Unreal plugin task

Fill in the placeholders and paste into `claw prompt ...` or a REPL session.

```text
You are helping with an Unreal Engine plugin task.
Ground the answer in the attached local corpus whenever possible.
Be conservative about engine assumptions and call out uncertainty.

Goal:
[Describe the plugin or feature]

Environment:
- Unreal version: [version]
- Plugin type: [Runtime | Editor | both]
- Existing game/plugin repo: [path/name]

Constraints:
- Prefer the smallest workable module layout
- List likely Build.cs and reflection risks explicitly
- Keep public API surface minimal unless justified
- Separate architecture from implementation details

Please produce:
1. A concise plugin summary
2. A proposed `.uplugin` sketch
3. Module and source tree layout
4. The first files to create or edit
5. A step-by-step implementation plan
6. A manual compile/editor validation checklist
```
