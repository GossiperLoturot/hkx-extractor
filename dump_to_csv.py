import io
import pathlib
import struct
import csv
import copy

import numpy as np
import quaternionic as qn


# interprete bytes as int16
def read_int16(reader: io.BufferedReader) -> int:
    return struct.unpack("h", reader.read(2))[0]


# interprete bytes as int32
def read_int32(reader: io.BufferedReader) -> int:
    return struct.unpack("i", reader.read(4))[0]


# interprete bytes as uint32
def read_uint32(reader: io.BufferedReader) -> int:
    return struct.unpack("I", reader.read(4))[0]


# interprete bytes as float32
def read_float32(reader: io.BufferedReader) -> float:
    return struct.unpack("f", reader.read(4))[0]


# interprete bytes as str
def read_cstring(reader: io.BufferedReader) -> str:
    all_bytes = bytearray()
    while True:
        new_bytes = reader.read(1)
        if new_bytes[0] == 0:
            break
        else:
            all_bytes.append(new_bytes[0])
    return all_bytes.decode(encoding="ascii")


# interprete bytes as header str
def read_hstring(reader: io.BufferedReader) -> str:
    all_bytes = bytearray()
    while True:
        new_bytes = reader.read(1)
        if new_bytes[0] == 10:
            break
        else:
            all_bytes.append(new_bytes[0])
    return all_bytes.decode(encoding="ascii")


class Transform:
    translation: np.ndarray
    rotation: qn.array
    scale: float


# interprete bytes as Transform
def read_transform(reader: io.BufferedReader) -> Transform:
    transform = Transform()

    x = read_float32(reader)
    y = read_float32(reader)
    z = read_float32(reader)
    w = read_float32(reader)
    transform.translation = np.array([x, y, z])

    x = read_float32(reader)
    y = read_float32(reader)
    z = read_float32(reader)
    w = read_float32(reader)
    transform.rotation = qn.array([w, x, y, z])

    x = read_float32(reader)
    y = read_float32(reader)
    z = read_float32(reader)
    w = read_float32(reader)
    transform.scale = z

    return transform


def mul_transform(self: Transform, other: Transform) -> Transform:
    transform = Transform()
    transform.translation = (
        self.translation + self.rotation.rotate(other.translation) * self.scale
    )
    transform.rotation = self.rotation * other.rotation
    transform.scale = self.scale * other.scale
    return transform


class Pose:
    time: float
    transforms: list[Transform]
    floats: list[float]


# interprete bytes as Pose
def read_pose(reader: io.BufferedReader, n_transforms: int, n_floats: int) -> Pose:
    pose = Pose()

    pose.time = read_float32(reader)

    pose.transforms = []
    for _ in range(n_transforms):
        pose.transforms.append(read_transform(reader))

    pose.floats = []
    for _ in range(n_floats):
        pose.floats.append(read_float32(reader))

    return pose


class Annotation:
    time: float
    text: str


# interprete bytes as Annotation
def read_annotation(reader: io.BufferedReader) -> Annotation:
    annotation = Annotation()
    annotation.time = read_float32(reader)
    annotation.text = read_cstring(reader)
    return annotation


class Animation:
    n_frames: int
    duration: float
    n_transforms: int
    n_floats: int
    poses: list[Pose]
    annotations: list[Annotation]


# interprete bytes as Animation
def read_animation(reader: io.BufferedReader) -> Animation:
    animation = Animation()

    header = read_hstring(reader)

    # validation checks
    version = read_uint32(reader)
    if version != 0x01000200:
        raise Exception("invalid version")

    n_skeltons = read_int32(reader)
    if n_skeltons != 0:
        raise Exception("found skeltons")

    n_animations = read_int32(reader)
    if n_animations != 1:
        raise Exception("not found animations")

    # read overview variables
    animation.n_frames = read_int32(reader)
    animation.duration = read_float32(reader)
    animation.n_transforms = read_int32(reader)
    animation.n_floats = read_int32(reader)

    # read pose per all frames
    animation.poses = []
    for _ in range(animation.n_frames):
        animation.poses.append(
            read_pose(reader, animation.n_transforms, animation.n_floats)
        )

    # read annotation overview variables
    n_annotation_tracks = read_int32(reader)
    n_annotations = read_int32(reader)

    # read annotations
    animation.annotations = []
    for i in range(n_annotations):
        animation.annotations = read_annotation(reader)

    return animation


class Skeleton:
    name: str
    parents: list[int]
    n_transforms: int
    transform_names: list[int]
    transforms: list[Transform]
    floats: list[float]
    float_names: list[str]


def read_skeleton(reader: io.BufferedReader) -> Skeleton:
    skeleton = Skeleton()

    header = read_hstring(reader)

    # validation checks
    version = read_uint32(reader)
    # if version != 0x01000200:
    #     raise Exception("invalid version")

    n_skeltons = read_int32(reader)
    # if n_skeltons != 1:
    #     raise Exception("not found skeleton")

    skeleton.name = read_cstring(reader)

    # read parent bone index per bones
    n_parents = read_int32(reader)
    skeleton.parents = []
    for _ in range(n_parents):
        skeleton.parents.append(read_int16(reader))

    # read bone index per bones
    skeleton.n_transforms = read_int32(reader)
    skeleton.transform_names = []
    for _ in range(skeleton.n_transforms):
        skeleton.transform_names.append(read_cstring(reader))

    # read transform per bone
    skeleton.n_transforms = read_int32(reader)
    skeleton.transforms = []
    for _ in range(skeleton.n_transforms):
        skeleton.transforms.append(read_transform(reader))

    # read float value per float slots
    skeleton.n_floats = read_int32(reader)
    skeleton.floats = []
    for _ in range(skeleton.n_floats):
        skeleton.floats.append(read_float32(reader))

    # read float name per float slots
    skeleton.n_floats = read_int32(reader)
    skeleton.float_names = []
    for _ in range(skeleton.n_floats):
        skeleton.float_names.append(read_cstring(reader))

    # n_animations = read_int32(reader)
    # if n_animations != 0:
    #     raise Exception("found animations")

    return skeleton


# apply parent transform influence
def get_global_transform_from_skeleton(skeleton: Skeleton, index: int) -> Transform:
    transform = skeleton.transforms[index]
    next = skeleton.parents[index]

    while next != -1:
        transform = mul_transform(skeleton.transforms[next], transform)
        next = skeleton.parents[next]

    return transform


def _debug_plot_skeleton(skeleton: Skeleton):
    lines = []
    for i in range(skeleton.n_transforms):
        parent = skeleton.parents[i]

        if parent == -1:
            continue

        t0 = get_global_transform_from_skeleton(skeleton, i)
        t1 = get_global_transform_from_skeleton(skeleton, parent)

        lines.append([t0.translation, t1.translation])

    import matplotlib.pyplot
    import mpl_toolkits.mplot3d.art3d

    fig = matplotlib.pyplot.figure()
    ax = fig.add_subplot(projection="3d")
    ax.add_collection(mpl_toolkits.mplot3d.art3d.Line3DCollection(lines))
    ax.set_xlim([-50, 50])
    ax.set_ylim([-50, 50])
    ax.set_zlim([0, 100])
    matplotlib.pyplot.show()


def _debug_plot_animation(skeleton: Skeleton, animation: Animation, pose_index: int):
    n_transforms = min(skeleton.n_transforms, animation.n_transforms)

    for i in range(n_transforms):
        skeleton.transforms[i] = animation.poses[pose_index].transforms[i]

    lines = []
    for i in range(skeleton.n_transforms):
        parent = skeleton.parents[i]

        if parent == -1:
            continue

        t0 = get_global_transform_from_skeleton(skeleton, i)
        t1 = get_global_transform_from_skeleton(skeleton, parent)

        lines.append([t0.translation, t1.translation])

    import matplotlib.pyplot
    import mpl_toolkits.mplot3d.art3d

    fig = matplotlib.pyplot.figure()
    ax = fig.add_subplot(projection="3d")
    ax.add_collection(mpl_toolkits.mplot3d.art3d.Line3DCollection(lines))
    ax.set_xlim([-50, 50])
    ax.set_ylim([-50, 50])
    ax.set_zlim([0, 100])
    matplotlib.pyplot.show()


def main():
    in_path = pathlib.Path("resources\dumpfiles")
    out_path = pathlib.Path("resources\csvfiles")

    skeleton_path = pathlib.Path("resources\skeleton.bin")
    with open(skeleton_path, "rb") as reader:
        print("file: {}".format(skeleton_path))
        skeleton = read_skeleton(reader)
        print("\tskeleton name: {}".format(skeleton.name))
        print("\ttransform count: {}".format(skeleton.n_transforms))
        print("\tfloat count: {}".format(skeleton.n_floats))

        # # require matplotlib to plot
        # _debug_plot_skeleton(skeleton)

    # output skeleton as csv
    with open(out_path / "skeleton.csv", "w", newline="") as f:
        w = csv.writer(f)

        for i in range(skeleton.n_transforms):
            name = skeleton.transform_names[i]

            parent = skeleton.parents[i]
            if parent != -1:
                parent_name = skeleton.transform_names[parent]
            else:
                parent_name = "NULL"

            transform = get_global_transform_from_skeleton(skeleton, i)
            t = transform.translation
            r = transform.rotation
            s = transform.scale

            # name, parent_name, x, y, z, qw, qx, qy, qz, scale
            row = [name, parent_name, t[0], t[1], t[2], r.w, r.x, r.y, r.z, s]
            w.writerow(row)

    # output animation as csv
    for file_path in in_path.iterdir():
        if not file_path.name.startswith(".") and file_path.name.endswith(".bin"):
            # read hkx dump file
            with open(file_path, "rb") as reader:
                print("file: {}".format(file_path))
                animation = read_animation(reader)
                print("\tframe count: {}".format(animation.n_frames))
                print("\tduration secs: {}".format(animation.duration))
                print("\ttransform count: {}".format(animation.n_transforms))
                print("\tfloat count: {}".format(animation.n_floats))

            # # require matplotlib to plot
            # _debug_plot_animation(skeleton, animation, 0)

            with open(out_path / (file_path.stem + ".csv"), "w", newline="") as f:
                w = csv.writer(f)
            
                n_transforms = min(skeleton.n_transforms, animation.n_transforms)

                clone_skeleton = copy.deepcopy(skeleton)
            
                for f in range(animation.n_frames):
                    for i in range(n_transforms):
                        clone_skeleton.transforms[i] = animation.poses[f].transforms[i]
            
                    for i in range(n_transforms):
                        name = skeleton.transform_names[i]
            
                        pose = get_global_transform_from_skeleton(clone_skeleton, i)
                        t = pose.translation
                        r = pose.rotation
                        s = pose.scale
            
                        # frame, name, x, y, z, qw, qx, qy, qz, scale
                        row = [f, name, t[0], t[1], t[2], r.w, r.x, r.y, r.z, s]
                        w.writerow(row)


main()
