# Manual Blender test checklist

Use this after generating or editing the add-on. The current harness does not automate these steps.

## Install

1. Zip the `scene_cleanup_helper` package or copy it into your Blender add-ons folder.
2. In Blender, open **Edit → Preferences → Add-ons**.
3. Install or enable the add-on.
4. Confirm registration succeeds with no traceback.

## UI smoke test

1. Open the 3D Viewport sidebar.
2. Find the **Scene Cleanup** tab.
3. Confirm the panel renders the scan button and scene options.
4. Confirm the count fields update after running a scan.

## Functional smoke test

1. Create or open a scene with:
   - at least one object with unapplied transforms
   - one or more duplicate material names
   - one material slot that can be cleaned up
2. Run **Scan Scene Cleanup Risks**.
3. Confirm the operator reports counts in Blender's status area.
4. Toggle the `include_hidden_objects` option and re-run the scan.
5. If cleanup actions were generated or extended, run them on a disposable scene first.

## Release-readiness checks

1. Disable and re-enable the add-on.
2. Reopen Blender and verify the add-on still loads.
3. Confirm `bl_info` metadata matches the intended Blender version.
4. Check that panel labels and operator names are understandable to artists.
5. Record any traceback or surprising behavior and feed that back into the corpus.
