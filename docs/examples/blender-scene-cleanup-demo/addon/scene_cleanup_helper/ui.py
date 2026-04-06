import bpy


class SCENECLEANUP_PT_sidebar(bpy.types.Panel):
    bl_label = "Scene Cleanup"
    bl_idname = "SCENECLEANUP_PT_sidebar"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Scene Cleanup"

    def draw(self, context):
        layout = self.layout
        props = context.scene.scene_cleanup_helper

        layout.label(text="Pre-export scan")
        layout.prop(props, "include_hidden_objects")
        layout.operator("scene_cleanup.scan_scene", icon="VIEWZOOM")

        box = layout.box()
        box.label(text="Latest scan")
        box.label(text=f"Duplicate materials: {props.duplicate_material_count}")
        box.label(text=f"Unapplied transforms: {props.unapplied_transform_count}")
