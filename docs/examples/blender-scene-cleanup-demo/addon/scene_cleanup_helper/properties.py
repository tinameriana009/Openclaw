import bpy


class SceneCleanupProperties(bpy.types.PropertyGroup):
    include_hidden_objects: bpy.props.BoolProperty(
        name="Include Hidden Objects",
        description="Include hidden objects when scanning for unapplied transforms",
        default=False,
    )
    duplicate_material_count: bpy.props.IntProperty(
        name="Duplicate Materials",
        default=0,
        min=0,
    )
    unapplied_transform_count: bpy.props.IntProperty(
        name="Objects With Unapplied Transforms",
        default=0,
        min=0,
    )
