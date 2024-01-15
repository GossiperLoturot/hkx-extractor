use std::{
    io::{Read, Write},
    path,
};

use anyhow::Context;
use clap::Parser;

#[rustfmt::skip]
#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value = r".\input")]
    input_dir: path::PathBuf,
    #[arg(short, long, default_value = r".\output")]
    output_dir: path::PathBuf,

    #[arg(long, default_value = r".\tmp")]
    tmp_dir: path::PathBuf,

    #[arg(long, default_value = r"C:\Program Files\Havok\HavokContentTools\hctStandAloneFilterManager.exe")]
    hct_exe: path::PathBuf,
}

#[tokio::main()]
async fn main() -> anyhow::Result<()> {
    simplelog::SimpleLogger::init(simplelog::LevelFilter::Info, simplelog::Config::default())?;

    let args = Args::parse();

    let hkx64_dir = args.input_dir;
    log::debug!("hkx64 dir: {:?}", hkx64_dir);

    let hkx86_dir = args.tmp_dir.join("hkx86");
    log::debug!("hkx86 dir: {:?}", hkx86_dir);

    let hct_exe = args.hct_exe;
    log::debug!("hct exe: {:?}", hct_exe);

    let hko_dir = args.tmp_dir.join("hko");
    log::debug!("hko dir: {:?}", hko_dir);

    let dump_dir = args.tmp_dir.join("dump");
    log::debug!("dump dir: {:?}", dump_dir);

    let hk_dump_exe = args.tmp_dir.join("hkdump.exe");
    log::debug!("hk dump exe: {:?}", hk_dump_exe);

    let skeleton_csv = args.output_dir.join("skeleton.csv");
    log::debug!("skeleton csv: {:?}", skeleton_csv);

    let csv_dir = args.output_dir;
    log::debug!("csv dir: {:?}", csv_dir);

    // hkx64 to hkx86

    log::info!("hkx64 to hkx86");

    std::fs::create_dir_all(&hkx86_dir)?;
    std::fs::create_dir_all(&hko_dir)?;

    let mut join_set = tokio::task::JoinSet::new();
    for entry in std::fs::read_dir(&hkx64_dir)? {
        if let Ok(entry) = entry {
            let input_file = entry.path();
            let output_file = hkx86_dir.join(entry.file_name());
            let hct_exe = hct_exe.clone();
            let hko_file = hko_dir.join(entry.file_name()).with_extension("hko");

            let fut = hkx64_to_hkx86(input_file, output_file, hct_exe, hko_file);
            join_set.spawn(fut);
        }
    }
    while let Some(_) = join_set.join_next().await {}

    // hkx86 to dump

    log::info!("hkx86 to dump");

    std::fs::create_dir_all(&dump_dir)?;

    let hk_dump = include_bytes!(r"hkdump.exe");
    std::fs::write(&hk_dump_exe, hk_dump)?;

    let mut join_set = tokio::task::JoinSet::new();
    for entry in std::fs::read_dir(&hkx86_dir)? {
        if let Ok(entry) = entry {
            let input_file = entry.path();
            let output_file = dump_dir.join(entry.file_name()).with_extension("bin");
            let hk_dump_exe = hk_dump_exe.clone();

            let fut = hkx86_to_dump(input_file, output_file, hk_dump_exe);
            join_set.spawn(fut);
        }
    }
    while let Some(_) = join_set.join_next().await {}

    // skeleton to csv

    std::fs::create_dir_all(&csv_dir)?;

    let skeleton = {
        let bytes = include_bytes!("skeleton.bin");
        let mut reader = std::io::Cursor::new(bytes);
        read_skeleton(&mut reader)?
    };

    skeleton_to_csv(&skeleton, &skeleton_csv).await?;

    // dump to csv

    log::info!("dump to csv");

    let mut join_set = tokio::task::JoinSet::new();
    for entry in std::fs::read_dir(&dump_dir)? {
        if let Ok(entry) = entry {
            let input_file = entry.path();
            let output_file = csv_dir.join(entry.file_name()).with_extension("csv");
            let skeleton = skeleton.clone();

            let fut = dump_to_csv(input_file, output_file, skeleton);
            join_set.spawn(fut);
        }
    }
    while let Some(_) = join_set.join_next().await {}

    // cleanup

    log::info!("clean up tmp dir");
    std::fs::remove_dir_all(args.tmp_dir)?;

    Ok(())
}

async fn hkx64_to_hkx86(
    input_file: path::PathBuf,
    output_file: path::PathBuf,
    hct_exe: path::PathBuf,
    hko_file: path::PathBuf,
) -> anyhow::Result<()> {
    let hko_template = include_str!(r"template.hko");
    let hko_placeholder = output_file
        .to_str()
        .with_context(|| format!("non utf8 path {:?}", output_file))?;
    let hko = hko_template.replace("{}", hko_placeholder);
    tokio::fs::write(&hko_file, hko).await?;

    let exit_status = tokio::process::Command::new(&hct_exe)
        .arg("-s")
        .arg(&hko_file)
        .arg(&input_file)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await?;

    if !exit_status.success() {
        tokio::fs::copy(&input_file, &output_file).await?;
    }

    log::info!("{:?} -[CONVERT]-> {:?}", input_file, output_file);

    Ok(())
}

async fn hkx86_to_dump(
    input_file: path::PathBuf,
    output_file: path::PathBuf,
    hk_dump_exe: path::PathBuf,
) -> anyhow::Result<()> {
    let exit_status = tokio::process::Command::new(&hk_dump_exe)
        .arg("-o")
        .arg(&output_file)
        .arg(&input_file)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await?;

    if !exit_status.success() {
        anyhow::bail!("failed to convert from hkx86 to dump");
    }

    log::info!("{:?} -[CONVERT]-> {:?}", input_file, output_file);

    Ok(())
}

async fn skeleton_to_csv(skeleton: &Skeleton, output_file: &path::Path) -> anyhow::Result<()> {
    let mut writer = std::fs::File::create(&output_file)?;

    for index in 0..skeleton.n_transforms {
        let name = &skeleton.transform_names[index as usize];

        let parent_index = skeleton.parents[index as usize];
        let parent_name = if parent_index != -1 {
            &skeleton.transform_names[parent_index as usize]
        } else {
            "NULL"
        };

        let transform = world_transform_from_skeleton(&skeleton, index);

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

async fn dump_to_csv(
    input_file: path::PathBuf,
    output_file: path::PathBuf,
    mut skeleton: Skeleton,
) -> anyhow::Result<()> {
    let mut reader = std::fs::File::open(&input_file)?;
    let animation = read_animation(&mut reader)?;

    let mut writer = std::fs::File::create(&output_file)?;
    for frame in 0..animation.n_frames {
        let n_transforms = i32::min(animation.n_transforms, skeleton.n_transforms);

        for index in 0..n_transforms {
            skeleton.transforms[index as usize] =
                animation.poses[frame as usize].transforms[index as usize];
        }

        for index in 0..n_transforms {
            let name = &skeleton.transform_names[index as usize];
            let transform = world_transform_from_skeleton(&skeleton, index);

            let row = format!(
                "{},{},{},{},{},{},{},{},{},{}\n",
                frame,
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

    log::info!("{:?} -[CONVERT]-> {:?}", input_file, output_file);

    Ok(())
}

#[derive(Clone, Copy, Default, Debug)]
struct Transform {
    location: glam::Vec3,
    rotation: glam::Quat,
    scale: f32,
}

#[derive(Clone, Default, Debug)]
struct Pose {
    time: f32,
    transforms: Vec<Transform>,
    floats: Vec<f32>,
}

#[derive(Clone, Default, Debug)]
struct Animation {
    n_frames: i32,
    duration: f32,
    n_transforms: i32,
    n_floats: i32,
    poses: Vec<Pose>,
}

#[derive(Clone, Default, Debug)]
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

fn read_string<R: Read>(reader: &mut R) -> anyhow::Result<String> {
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

fn read_header<R: Read>(reader: &mut R) -> anyhow::Result<String> {
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

fn read_animation<R: Read>(reader: &mut R) -> anyhow::Result<Animation> {
    let header = read_header(reader)?;
    log::debug!("\theader: {}", header);

    let version = read_u32(reader)?;
    log::debug!("\tversion: {}", version);
    if version != 0x1000200 {
        anyhow::bail!("invalid version");
    }

    let n_skeletons = read_i32(reader)?;
    log::debug!("\tskeleton count: {}", n_skeletons);
    if n_skeletons != 0 {
        anyhow::bail!("found skeleton");
    }

    let n_animations = read_i32(reader)?;
    log::debug!("\tanimation count: {}", n_skeletons);
    if n_animations == 0 {
        anyhow::bail!("missing animation");
    }

    let n_frames = read_i32(reader)?;
    log::debug!("\tframe count: {}", n_frames);

    let duration = read_f32(reader)?;
    log::debug!("\tduraion secs: {}", duration);

    let n_transforms = read_i32(reader)?;
    log::debug!("\ttransform count: {}", n_transforms);

    let n_floats = read_i32(reader)?;
    log::debug!("\tfloat count: {}", n_floats);

    let mut poses = vec![];
    for _ in 0..n_frames {
        poses.push(read_pose(reader, n_transforms, n_floats)?);
    }

    Ok(Animation {
        n_frames,
        duration,
        n_transforms,
        n_floats,
        poses,
    })
}

fn read_skeleton<R: Read>(reader: &mut R) -> anyhow::Result<Skeleton> {
    let header = read_header(reader)?;
    log::debug!("header: {}", header);

    let version = read_u32(reader)?;
    log::debug!("\tversion: {}", version);

    let n_skeletons = read_i32(reader)?;
    log::debug!("\tskeleton count: {}", n_skeletons);
    if n_skeletons == 0 {
        anyhow::bail!("missing skeleton");
    }

    let name = read_string(reader)?;
    log::debug!("\tskeleton name: {}", name);

    let n_transforms = read_i32(reader)?;
    let mut parents = vec![];
    for _ in 0..n_transforms {
        parents.push(read_i16(reader)?);
    }

    let n_transforms = read_i32(reader)?;
    let mut transform_names = vec![];
    for _ in 0..n_transforms {
        transform_names.push(read_string(reader)?);
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
        float_names.push(read_string(reader)?);
    }

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

fn mul_transform(t1: Transform, t2: Transform) -> Transform {
    let location = t1.location + t1.rotation * t2.location * t1.scale;
    let rotation = t1.rotation * t2.rotation;
    let scale = t1.scale * t2.scale;

    Transform {
        location,
        rotation,
        scale,
    }
}

fn world_transform_from_skeleton(skeleton: &Skeleton, index: i32) -> Transform {
    let mut transform = skeleton.transforms[index as usize].clone();
    let mut index = skeleton.parents[index as usize];

    while index != -1 {
        transform = mul_transform(skeleton.transforms[index as usize], transform);
        index = skeleton.parents[index as usize];
    }

    return transform;
}
