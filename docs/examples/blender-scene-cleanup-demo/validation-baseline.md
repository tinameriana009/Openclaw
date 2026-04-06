# Validation baseline for the scene cleanup demo

Use this file to make the Blender workflow feel concrete.

The point is not to build a perfect QA matrix. The point is to give the operator a **known disposable scene** with a small set of expected scan results.

## Disposable scene recipe

Create a fresh Blender scene and set it up like this:

### Objects

1. Leave the default cube in the scene.
2. Move the cube to `X = 2.0` so it has an unapplied location offset.
3. Add a UV Sphere and scale it to `2.0` on all axes so it has an unapplied scale.
4. Add a Cone, hide it in the viewport, and rotate it on `Z` by roughly `45°` so it has an unapplied rotation while hidden.

### Materials

1. Create a material named `Demo_Mat` and assign it to the cube.
2. Duplicate that material so Blender creates a second name like `Demo_Mat.001`.
3. Rename the duplicate so it is also named `Demo_Mat`.

That leaves you with two materials sharing the same visible name, which the demo counts as one duplicate beyond the first instance.

## Expected scan results

With `Include Hidden Objects` **disabled**:

- Duplicate materials: **1**
- Objects with unapplied transforms: **2**

With `Include Hidden Objects` **enabled**:

- Duplicate materials: **1**
- Objects with unapplied transforms: **3**

## Why these counts matter

These expectations let an operator tell the difference between:

- the add-on failing to register
- the panel drawing but not scanning the scene correctly
- the `include_hidden_objects` toggle not affecting the result
- a prompt-generated patch accidentally changing the behavior

## Known limitations of this baseline

- The demo treats any non-zero location/rotation or non-unit scale as a risk.
- It does not yet distinguish mesh objects from cameras, lights, or empties.
- It does not yet detect whether duplicate names are actually harmful for a given export path.
- It does not perform destructive cleanup actions; it only scans and reports.

That is fine for a first showcase. The baseline is meant to support confidence, not to oversell the add-on.
