# Prompt template: Blender add-on task

Fill in the placeholders and paste into `claw prompt ...` or a REPL session.

```text
You are helping with a Blender add-on task.
Ground your answer in the attached local corpus whenever possible.
If the corpus is missing key evidence, say so instead of inventing details.

Goal:
[Describe the add-on or feature]

Environment:
- Blender version: [version]
- Python version if relevant: [version]
- Existing repo path or package name: [path/name]

Constraints:
- Prefer a minimal file layout
- Prefer understandable code over clever abstractions
- Call out Blender API assumptions explicitly
- Separate planning from implementation

Please produce:
1. A concise feature summary
2. The proposed package/file tree
3. Operators, panels, properties, and preferences involved
4. The exact files to create or edit first
5. A minimal implementation plan
6. Risks, unknowns, and manual validation steps in Blender
```
