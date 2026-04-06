from collections import Counter

import bpy


class SCENECLEANUP_OT_scan_scene(bpy.types.Operator):
    bl_idname = "scene_cleanup.scan_scene"
    bl_label = "Scan Scene Cleanup Risks"
    bl_description = "Scan the current scene for a few simple cleanup risks"
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        props = context.scene.scene_cleanup_helper

        materials = [material.name for material in bpy.data.materials]
        duplicate_material_count = sum(count - 1 for count in Counter(materials).values() if count > 1)

        objects = context.scene.objects
        if not props.include_hidden_objects:
            objects = [obj for obj in objects if obj.visible_get()]

        unapplied_transform_count = sum(
            1
            for obj in objects
            if tuple(round(value, 4) for value in obj.location) != (0.0, 0.0, 0.0)
            or tuple(round(value, 4) for value in obj.rotation_euler) != (0.0, 0.0, 0.0)
            or tuple(round(value, 4) for value in obj.scale) != (1.0, 1.0, 1.0)
        )

        props.duplicate_material_count = duplicate_material_count
        props.unapplied_transform_count = unapplied_transform_count

        self.report(
            {"INFO"},
            (
                "Scene cleanup scan complete: "
                f"duplicates={duplicate_material_count}, "
                f"unapplied_transforms={unapplied_transform_count}"
            ),
        )
        return {"FINISHED"}
