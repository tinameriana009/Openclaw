# Demo brief: Scene cleanup helper add-on

## Scenario

An artist wants a lightweight Blender add-on that helps clean a scene before export.

The add-on should focus on small, understandable checks instead of trying to become a full pipeline manager.

## User problems

- Some materials are empty or unused.
- Duplicate material names make the scene harder to audit.
- Objects with unapplied transforms create export surprises.
- Artists want one obvious sidebar panel instead of scattered manual checks.

## Target environment

- Blender 4.x
- Python-only add-on
- Manual install and manual test loop

## Product shape

Keep the package small and explicit:

- one property group for scene-level settings and cached counts
- one or two operators for scanning and cleanup actions
- one sidebar panel under the 3D Viewport
- registration that is easy to read and debug

## Good prompt to start with

```text
You are helping evolve a Blender add-on demo corpus.
Ground your answer in the attached local files.

Task:
Review this scene-cleanup helper demo and propose the next smallest productization step.

Please provide:
1. A short description of the current user workflow
2. The package/file tree and responsibility of each file
3. The smallest useful next feature
4. The exact files to edit
5. Manual Blender validation steps
6. Any Blender API assumptions or risks
```
