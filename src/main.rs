use std::{
    fs,
    io::{Read, Write},
    path::Path,
    process,
};

fn main() {
    convert_hkx64_to_hkx86().unwrap();
    convert_hkx86_to_dump().unwrap();
    convert_dump_to_csv().unwrap();
}

fn convert_hkx64_to_hkx86() -> anyhow::Result<()> {
    let hct_path: &Path =
        Path::new(r"C:\Program Files\Havok\HavokContentTools\hctStandAloneFilterManager.exe");
    let hco_path = Path::new(r".\resources\hkx64_to_hkx86.hko");
    let indir_path = Path::new(r".\resources\hkx64files");
    let outdir_path = Path::new(r".\resources\hkx86files");
    let tmpfile_path = Path::new(r".\tmp.hkx");

    if !hct_path.exists() {
        anyhow::bail!("missing Havok Content Tools {:?}", hct_path);
    }

    if !hco_path.exists() {
        anyhow::bail!("missing hco file {:?}", hco_path);
    }

    if !indir_path.exists() {
        anyhow::bail!("missing hkx64files {:?}", indir_path);
    }

    if !outdir_path.exists() {
        anyhow::bail!("missing hkx86files {:?}", indir_path);
    }

    println!("convert hkx64 to hkx86");
    fs::read_dir(indir_path)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "hkx"))
        .for_each(|entry| {
            let infile_path = entry.path();
            let outfile_path = outdir_path.join(entry.file_name());

            let result = convert_hkx64_to_hkx86_entry(
                &hct_path,
                &hco_path,
                &infile_path,
                &outfile_path,
                &tmpfile_path,
            );

            if result.is_ok() {
                println!("{:?} -[CONVERT]-> {:?}", infile_path, outfile_path);
                return;
            }

            let result = convert_hkx64_to_hkx86_entry_fallback(&infile_path, &outfile_path);

            if result.is_ok() {
                println!("{:?} -[COPY]-> x", infile_path);
                return;
            }

            println!("{:?} -[FAILED]-> x", infile_path);
        });

    return Ok(());
}

fn convert_hkx64_to_hkx86_entry(
    hct_path: &Path,
    hco_path: &Path,
    infile_path: &Path,
    outfile_path: &Path,
    tmpfile_path: &Path,
) -> anyhow::Result<()> {
    process::Command::new(hct_path)
        .arg("-s")
        .arg(hco_path)
        .arg(infile_path)
        .spawn()?
        .wait()?;

    fs::rename(tmpfile_path, outfile_path)?;

    Ok(())
}

fn convert_hkx64_to_hkx86_entry_fallback(
    infile_path: &Path,
    outfile_path: &Path,
) -> anyhow::Result<()> {
    fs::copy(infile_path, outfile_path)?;

    Ok(())
}

fn convert_hkx86_to_dump() -> anyhow::Result<()> {
    let hkdump_path: &Path = Path::new(r".\resources\hkdump.exe");
    let indir_path = Path::new(r".\resources\hkx86files");
    let outdir_path = Path::new(r".\resources\dumpfiles");

    if !hkdump_path.exists() {
        anyhow::bail!("missing hkdump {:?}", hkdump_path);
    }

    if !indir_path.exists() {
        println!("missing hkx86files");
    }

    if !outdir_path.exists() {
        println!("missing dumpfiles");
        fs::create_dir(outdir_path)?;
    }

    println!("convert hkx86 to dump");
    fs::read_dir(indir_path)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "hkx"))
        .for_each(|entry| {
            let infile_path = entry.path();
            let outfile_path =
                outdir_path.join(entry.path().with_extension("bin").file_name().unwrap());

            let result = convert_hkx86_to_dump_entry(&hkdump_path, &infile_path, &outfile_path);

            if result.is_ok() {
                println!("{:?} -[CONVERT]-> {:?}", infile_path, outfile_path);
                return;
            }

            println!("{:?} -[FAILED]-> x", infile_path);
        });

    return Ok(());
}

fn convert_hkx86_to_dump_entry(
    hkdump_path: &Path,
    infile_path: &Path,
    outfile_path: &Path,
) -> anyhow::Result<()> {
    process::Command::new(hkdump_path)
        .arg("-o")
        .arg(outfile_path)
        .arg(infile_path)
        .spawn()?
        .wait()?;

    Ok(())
}

fn convert_dump_to_csv() -> anyhow::Result<()> {
    let skeleton_path = Path::new(r".\resources\skeleton.bin");
    let outfile_path = Path::new(r".\resources\csvfiles\skeleton.csv");
    let indir_path = Path::new(r".\resources\dumpfiles");
    let outdir_path = Path::new(r".\resources\csvfiles");

    let skeleton = read_skeleton_from_dump(skeleton_path)?;
    write_csv_from_skeleton(&skeleton, outfile_path)?;

    println!("convert dump to csv");
    fs::read_dir(indir_path)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "bin"))
        .for_each(|entry| {
            let infile_path = entry.path();
            let outfile_path =
                outdir_path.join(entry.path().with_extension("csv").file_name().unwrap());

            let result = read_animation_from_dump(&infile_path).and_then(|animation| {
                write_csv_from_animation(&skeleton, &animation, &outfile_path)
            });

            if result.is_err() {
                println!("{:?} -[FAILED]-> x", infile_path);
                return;
            }

            println!("{:?} -[CONVERT]-> {:?}", infile_path, outfile_path);
        });

    Ok(())
}

fn read_skeleton_from_dump(skeleton_path: &Path) -> anyhow::Result<Skeleton> {
    println!("load skeleton from dump file");
    let mut reader = fs::File::open(skeleton_path)?;
    let skeleton = read_skeleton(&mut reader)?;
    Ok(skeleton)
}

fn write_csv_from_skeleton(skeleton: &Skeleton, outfile_path: &Path) -> anyhow::Result<()> {
    let mut writer = fs::File::create(outfile_path)?;

    for i in 0..skeleton.n_transforms {
        let name = &skeleton.transform_names[i as usize];

        let parent_index = skeleton.parents[i as usize];

        let parent_name = if parent_index != -1 {
            &skeleton.transform_names[parent_index as usize]
        } else {
            "NULL"
        };

        let transform = get_world_transform_from_skeleton(&skeleton, i);

        let row = format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            name,
            parent_name,
            transform.location.x,
            transform.location.y,
            transform.location.z,
            transform.rotation.w,
            transform.rotation.x,
            transform.rotation.y,
            transform.rotation.z,
            transform.scale
        );

        writer.write(row.as_bytes())?;
    }

    Ok(())
}

fn read_animation_from_dump(infile_path: &Path) -> anyhow::Result<Animation> {
    println!("load animation from dump file {:?}", infile_path);
    let mut reader = fs::File::open(infile_path)?;
    let animation = read_animation(&mut reader)?;
    Ok(animation)
}

fn write_csv_from_animation(
    skeleton: &Skeleton,
    animation: &Animation,
    outfile_path: &Path,
) -> anyhow::Result<()> {
    let mut writer = fs::File::create(outfile_path)?;

    let mut skeleton = skeleton.clone();

    for f in 0..animation.n_frames {
        let n_transforms = i32::min(animation.n_transforms, skeleton.n_transforms);

        for i in 0..n_transforms {
            skeleton.transforms[i as usize] =
                animation.poses[f as usize].transforms[i as usize].clone();
        }

        for i in 0..n_transforms {
            let name = &skeleton.transform_names[i as usize];

            let transform = get_world_transform_from_skeleton(&skeleton, i);

            let row = format!(
                "{},{},{},{},{},{},{},{},{},{}\n",
                f,
                name,
                transform.location.x,
                transform.location.y,
                transform.location.z,
                transform.rotation.w,
                transform.rotation.x,
                transform.rotation.y,
                transform.rotation.z,
                transform.scale
            );

            writer.write(row.as_bytes())?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct Transform {
    location: glam::Vec3,
    rotation: glam::Quat,
    scale: f32,
}

#[derive(Debug, Clone)]
struct Pose {
    time: f32,
    transforms: Vec<Transform>,
    floats: Vec<f32>,
}

#[derive(Debug, Clone)]
struct Annotation {
    time: f32,
    text: String,
}

#[derive(Debug, Clone)]
struct Animation {
    n_frames: i32,
    duration: f32,
    n_transforms: i32,
    n_floats: i32,
    poses: Vec<Pose>,
    annotations: Vec<Annotation>,
}

#[derive(Debug, Clone)]
struct Skeleton {
    name: String,
    n_transforms: i32,
    parents: Vec<i16>,
    transform_names: Vec<String>,
    transforms: Vec<Transform>,
    n_floats: i32,
    float_names: Vec<String>,
    floats: Vec<f32>,
}

fn read_i16<R: Read>(reader: &mut R) -> anyhow::Result<i16> {
    let mut buf = [0u8; 2];
    reader.read(&mut buf)?;
    Ok(i16::from_ne_bytes(buf))
}

fn read_u32<R: Read>(reader: &mut R) -> anyhow::Result<u32> {
    let mut buf = [0u8; 4];
    reader.read(&mut buf)?;
    Ok(u32::from_ne_bytes(buf))
}

fn read_i32<R: Read>(reader: &mut R) -> anyhow::Result<i32> {
    let mut buf = [0u8; 4];
    reader.read(&mut buf)?;
    Ok(i32::from_ne_bytes(buf))
}

fn read_f32<R: Read>(reader: &mut R) -> anyhow::Result<f32> {
    let mut buf = [0u8; 4];
    reader.read(&mut buf)?;
    Ok(f32::from_ne_bytes(buf))
}

fn read_cstring<R: Read>(reader: &mut R) -> anyhow::Result<String> {
    let mut buf = [0u8; 1];
    let mut acc = vec![];
    loop {
        reader.read(&mut buf)?;
        if buf[0] != 0 {
            acc.push(buf[0]);
        } else {
            break;
        }
    }
    let s = String::from_utf8(acc)?;
    Ok(s)
}

fn read_hstring<R: Read>(reader: &mut R) -> anyhow::Result<String> {
    let mut buf = [0u8; 1];
    let mut acc = vec![];
    loop {
        reader.read(&mut buf)?;
        if buf[0] != 10 {
            acc.push(buf[0]);
        } else {
            break;
        }
    }
    let s = String::from_utf8(acc)?;
    Ok(s)
}

fn read_transform<R: Read>(reader: &mut R) -> anyhow::Result<Transform> {
    let x = read_f32(reader)?;
    let y = read_f32(reader)?;
    let z = read_f32(reader)?;
    let w = read_f32(reader)?;
    let location = glam::Vec3::new(x, y, z);

    let x = read_f32(reader)?;
    let y = read_f32(reader)?;
    let z = read_f32(reader)?;
    let w = read_f32(reader)?;
    let rotation = glam::Quat::from_xyzw(x, y, z, w);

    let x = read_f32(reader)?;
    let y = read_f32(reader)?;
    let z = read_f32(reader)?;
    let w = read_f32(reader)?;
    let scale = z;

    Ok(Transform {
        location,
        rotation,
        scale,
    })
}

fn read_pose<R: Read>(reader: &mut R, n_transforms: i32, n_floats: i32) -> anyhow::Result<Pose> {
    let time = read_f32(reader)?;
    let mut transforms = vec![];
    for _ in 0..n_transforms {
        transforms.push(read_transform(reader)?);
    }
    let mut floats = vec![];
    for _ in 0..n_floats {
        floats.push(read_f32(reader)?);
    }
    Ok(Pose {
        time,
        transforms,
        floats,
    })
}

fn read_annotation<R: Read>(reader: &mut R) -> anyhow::Result<Annotation> {
    let time = read_f32(reader)?;
    let text = read_cstring(reader)?;
    Ok(Annotation { time, text })
}

fn read_animation<R: Read>(reader: &mut R) -> anyhow::Result<Animation> {
    let header = read_hstring(reader)?;
    println!("\theader: {}", header);

    let version = read_u32(reader)?;
    println!("\tversion: {}", version);

    let n_skeletons = read_i32(reader)?;
    println!("\tskeleton count: {}", n_skeletons);

    // including no skeleton

    let n_animations = read_i32(reader)?;
    println!("\tanimation count: {}", n_skeletons);
    if n_animations == 0 {
        anyhow::bail!("missing animation");
    }

    let n_frames = read_i32(reader)?;
    println!("\tframe count: {}", n_frames);

    let duration = read_f32(reader)?;
    println!("\tduraion secs: {}", duration);

    let n_transforms = read_i32(reader)?;
    println!("\ttransform count: {}", n_transforms);

    let n_floats = read_i32(reader)?;
    println!("\tfloat count: {}", n_floats);

    let mut poses = vec![];
    for _ in 0..n_frames {
        poses.push(read_pose(reader, n_transforms, n_floats)?);
    }

    let n_annotation_tracks = read_i32(reader)?;
    println!("\tannotation track count: {}", n_floats);

    let n_annotations = read_i32(reader)?;
    println!("\tannotation count: {}", n_floats);

    let mut annotations = vec![];
    for _ in 0..n_annotations {
        annotations.push(read_annotation(reader)?);
    }

    Ok(Animation {
        n_frames,
        duration,
        n_transforms,
        n_floats,
        poses,
        annotations,
    })
}

fn read_skeleton<R: Read>(reader: &mut R) -> anyhow::Result<Skeleton> {
    let header = read_hstring(reader)?;
    println!("header: {}", header);

    let version = read_u32(reader)?;
    println!("\tversion: {}", version);

    let n_skeletons = read_i32(reader)?;
    println!("\tskeleton count: {}", n_skeletons);
    if n_skeletons == 0 {
        anyhow::bail!("missing akeleton");
    }

    let name = read_cstring(reader)?;
    println!("\tskeleton name: {}", name);

    let n_transforms = read_i32(reader)?;
    let mut parents = vec![];
    for _ in 0..n_transforms {
        parents.push(read_i16(reader)?);
    }

    let n_transforms = read_i32(reader)?;
    let mut transform_names = vec![];
    for _ in 0..n_transforms {
        transform_names.push(read_cstring(reader)?);
    }

    let n_transforms = read_i32(reader)?;
    let mut transforms = vec![];
    for _ in 0..n_transforms {
        transforms.push(read_transform(reader)?);
    }

    let n_floats = read_i32(reader)?;
    let mut floats = vec![];
    for _ in 0..n_floats {
        floats.push(read_f32(reader)?);
    }

    let n_floats = read_i32(reader)?;
    let mut float_names = vec![];
    for _ in 0..n_floats {
        float_names.push(read_cstring(reader)?);
    }

    let n_animations = read_i32(reader)?;
    println!("\tanimation count: {}", n_animations);

    // including no animation

    Ok(Skeleton {
        name,
        n_transforms,
        parents,
        transform_names,
        transforms,
        n_floats,
        floats,
        float_names,
    })
}

fn mul_transform(t1: &Transform, t2: &Transform) -> Transform {
    let location = t1.location + t1.rotation * t2.location * t1.scale;
    let rotation = t1.rotation * t2.rotation;
    let scale = t1.scale * t2.scale;
    Transform {
        location,
        rotation,
        scale,
    }
}

fn get_world_transform_from_skeleton(skeleton: &Skeleton, index: i32) -> Transform {
    let mut transform = skeleton.transforms[index as usize].clone();
    let mut next = skeleton.parents[index as usize];

    while next != -1 {
        transform = mul_transform(&skeleton.transforms[next as usize], &transform);
        next = skeleton.parents[next as usize];
    }

    return transform;
}
