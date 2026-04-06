bl_info = {
    "name": "Scene Cleanup Helper",
    "author": "Openclaw demo corpus",
    "version": (0, 1, 0),
    "blender": (4, 0, 0),
    "location": "View3D > Sidebar > Scene Cleanup",
    "description": "Scan a scene for a few common pre-export cleanup risks.",
    "category": "3D View",
}

from .operators import SCENECLEANUP_OT_scan_scene
from .properties import SceneCleanupProperties
from .ui import SCENECLEANUP_PT_sidebar

CLASSES = (
    SceneCleanupProperties,
    SCENECLEANUP_OT_scan_scene,
    SCENECLEANUP_PT_sidebar,
)


def register():
    import bpy

    for cls in CLASSES:
        bpy.utils.register_class(cls)
    bpy.types.Scene.scene_cleanup_helper = bpy.props.PointerProperty(type=SceneCleanupProperties)


def unregister():
    import bpy

    del bpy.types.Scene.scene_cleanup_helper
    for cls in reversed(CLASSES):
        bpy.utils.unregister_class(cls)
