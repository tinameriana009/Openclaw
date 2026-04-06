# Manual Blender test checklist

Use this after generating or editing the add-on. The current harness does not automate these steps.

Before opening Blender, read `validation-baseline.md` so you know what numbers a healthy demo scan should return.

## Install

1. Build a zip with `python3 docs/examples/blender-scene-cleanup-demo/tools/package_demo_addon.py` or copy the `scene_cleanup_helper` package into your Blender add-ons folder.
2. In Blender, open **Edit → Preferences → Add-ons**.
3. Install or enable the add-on.
4. Confirm registration succeeds with no traceback.

## UI smoke test

1. Open the 3D Viewport sidebar.
2. Find the **Scene Cleanup** tab.
3. Confirm the panel renders the scan button and scene options.
4. Confirm the count fields update after running a scan.
5. Confirm the panel wording is understandable without reading the source code.

## Functional smoke test

1. Create the disposable scene from `validation-baseline.md` or open an equivalent test file.
2. Run **Scan Scene Cleanup Risks** with `Include Hidden Objects` disabled.
3. Confirm the operator reports counts in Blender's status area.
4. Confirm the panel shows:
   - duplicate materials = `1`
   - unapplied transforms = `2`
5. Toggle `Include Hidden Objects` on and re-run the scan.
6. Confirm the panel now shows:
   - duplicate materials = `1`
   - unapplied transforms = `3`
7. If cleanup actions were generated or extended, run them on a disposable scene first.

## Release-readiness checks

1. Disable and re-enable the add-on.
2. Reopen Blender and verify the add-on still loads.
3. Confirm `bl_info` metadata matches the intended Blender version.
4. Check that panel labels and operator names are understandable to artists.
5. Record any traceback, confusing UI text, or unexpected counts and feed that back into the corpus.

## What to report back into the prompt loop

Capture the smallest useful facts:

- exact traceback text, if any
- whether registration failed or runtime behavior failed
- whether the expected counts matched the baseline
- whether the wording or panel layout confused the artist/operator

That keeps the next prompt grounded in observable behavior instead of vague "it didn't work" feedback.
