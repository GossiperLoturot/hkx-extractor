import csv
import pathlib
import os

import bpy
import mathutils


# Get the directory where this .blend file is saved
blend_dir = os.path.dirname(bpy.data.filepath)
if not blend_dir:
    raise Exception("Please save your .blend file before running this script!")

csv_dir = os.path.join(blend_dir, "output")


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


for entry in pathlib.Path(csv_dir).iterdir():
    if not entry.name.startswith(".") and entry.name.endswith(".csv") and entry.name != "skeleton.csv":
        with open(entry) as f:
            reader = csv.reader(f)
            
            matrix_map_per_frame = {}
            for row in reader:
                frame = int(row[0])
                
                if not frame in matrix_map_per_frame:
                    matrix_map_per_frame[frame] = {}
                
                name = row[1]
                
                lx, ly, lz = float(row[2]), float(row[3]), float(row[4])
                qw, qx, qy, qz = float(row[5]), float(row[6]), float(row[7]), float(row[8])
                s = float(row[9])
                
                location = mathutils.Vector([lx, ly, lz])
                rotation = mathutils.Quaternion([qw, qx, qy, qz])
                scale = mathutils.Vector([s, s, s])
                matrix_map_per_frame[frame][name] = mathutils.Matrix.LocRotScale(location, rotation, scale)
                
        obj = bpy.context.object
        
        if not obj.animation_data:
            obj.animation_data_create()
        
        obj.animation_data.action = bpy.data.actions.new(entry.name)
            
        for frame in matrix_map_per_frame:
            set_pose_matrices(obj, matrix_map_per_frame[frame])
            for bone in obj.pose.bones:
                bone.keyframe_insert("location", frame=frame)
                bone.keyframe_insert("rotation_quaternion", frame=frame)
                bone.keyframe_insert("scale", frame=frame)
