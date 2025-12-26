import csv
import os

import bpy
import mathutils


blend_dir = os.path.dirname(bpy.data.filepath)
if not blend_dir:
    raise Exception("Please save your .blend file before running this script!")

csv_path = os.path.join(blend_dir, "output", "skeleton.csv")


# https://docs.blender.org/api/current/bpy.types.Bone.html#bpy.types.Bone.convert_local_to_pose
def set_pose_matrices(obj, matrix_map):
    "Assign pose space matrices of all bones at once, ignoring constraints."

    def rec(pbone, parent_matrix):
        if pbone.name in matrix_map:
            matrix = matrix_map[pbone.name]

            # # Instead of:
            # pbone.matrix = matrix
            # bpy.context.view_layer.update()

            # Compute and assign local matrix, using the new parent matrix
            if pbone.parent:
                pbone.matrix_basis = pbone.bone.convert_local_to_pose(
                    matrix,
                    pbone.bone.matrix_local,
                    parent_matrix=parent_matrix,
                    parent_matrix_local=pbone.parent.bone.matrix_local,
                    invert=True
                )
            else:
                pbone.matrix_basis = pbone.bone.convert_local_to_pose(
                    matrix,
                    pbone.bone.matrix_local,
                    invert=True
                )
        else:
            # Compute the updated pose matrix from local and new parent matrix
            if pbone.parent:
                matrix = pbone.bone.convert_local_to_pose(
                    pbone.matrix_basis,
                    pbone.bone.matrix_local,
                    parent_matrix=parent_matrix,
                    parent_matrix_local=pbone.parent.bone.matrix_local,
                )
            else:
                matrix = pbone.bone.convert_local_to_pose(
                    pbone.matrix_basis,
                    pbone.bone.matrix_local,
                )

        # Recursively process children, passing the new matrix through
        for child in pbone.children:
            rec(child, matrix)

    # Scan all bone trees from their roots
    for pbone in obj.pose.bones:
        if not pbone.parent:
            rec(pbone, None)


arm = bpy.data.armatures.new("auto_armature")
obj = bpy.data.objects.new("auto_armature", arm)
bpy.context.scene.collection.objects.link(obj)
    
bpy.context.view_layer.objects.active = obj   

if bpy.ops.object.mode_set.poll():
    bpy.ops.object.mode_set(mode="EDIT")

with open(csv_path) as f:
    reader = csv.reader(f)
    
    relations = {}
    matrices = {}
    
    for row in reader:
        name = row[0]
        parent_name = row[1]
        
        bone = arm.edit_bones.new(name)
        bone.name = name
        bone.head = [0.0, 0.0, 0.0]
        bone.tail = [0.0, 0.0, 1.0]
        relations[name] = parent_name
        
        location = mathutils.Vector([float(row[2]), float(row[3]), float(row[4])])
        rotation = mathutils.Quaternion([float(row[5]), float(row[6]), float(row[7]), float(row[8])])
        scale = mathutils.Vector([float(row[9]), float(row[9]), float(row[9])])
        matrices[name] = mathutils.Matrix.LocRotScale(location, rotation, scale)
        
    for name, parent_name in relations.items():
        if parent_name != "NULL":
            arm.edit_bones[name].parent = arm.edit_bones[parent_name]
            
    if bpy.ops.object.mode_set.poll():
        bpy.ops.object.mode_set(mode="POSE")
        
    set_pose_matrices(obj, matrices)
        
    is_handscale_fix = True
    if is_handscale_fix:
        if "NPC L Hand [LHnd]" in matrices:
            obj.pose.bones["NPC L Hand [LHnd]"].scale *= 1.333
        if "NPC R Hand [RHnd]" in matrices:
            obj.pose.bones["NPC R Hand [RHnd]"].scale *= 1.333
        
    if bpy.ops.pose.armature_apply.poll():
        bpy.ops.pose.armature_apply()
    