use std::fs::{remove_file, rename};
use std::process::Command;

use anyhow::Context;
use globset::{Glob, GlobSetBuilder};
use indicatif::ProgressBar;
use inquire::{Select, Text};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use walkdir::{DirEntry, WalkDir};

fn main() {
    let (target_dir, dimension_limit, upscale_factor) = prompts();
    let mut builder = GlobSetBuilder::new();
    builder.add(Glob::new("*/Textures/**/*.{png,jpg}").unwrap());
    builder.add(Glob::new("*/UI/*").unwrap());
    let glob = builder.build().unwrap();

    let now = std::time::Instant::now();

    let images = WalkDir::new(
        target_dir,
    )
    .into_iter()
    .filter_map(|entry| entry.ok())
    .filter(|entry| glob.matches(entry.path()) == &[0])
    .collect::<Vec<DirEntry>>();
    let size = images.len();
    let pb = ProgressBar::new(size as u64);

    images.into_par_iter().for_each(|entry| {
        if let Err(e) = process_image(entry.clone(), dimension_limit as u32, &upscale_factor) {
            eprintln!("error: {}", e)
        }
        pb.inc(1);
        pb.println(format!("{} finish.", entry.file_name().to_string_lossy()));
        //println!("{}", entry.path().display());
    });
    pb.finish();
    println!("use time {:?}", now.elapsed());

    glob::glob("waifu2x-ncnn-vulkan.*.log")
        .unwrap()
        .for_each(|p| remove_file(&p.unwrap()).unwrap());
    let _ = Command::new("cmd.exe").arg("/c").arg("pause").status();
}

fn prompts() -> (String, u16, String) {
    let target_dir = Text::new("What is your mod folder?")
        .with_help_message("usually it is your rimworld creative workshop content directory")
        .prompt()
        .unwrap();

    let dimension_limit_options: Vec<&str> =
        vec!["64", "128", "256", "512", "1024", "2048", "4096"];
    let dimension_limit = Select::new(
        "Upscale only if texture is smaller than",
        dimension_limit_options,
    )
        .with_help_message("The higher the resolution, the clearer and worse the performance.maybe you only need 128 or 256.")
        .with_starting_cursor(1)
        .prompt()
        .unwrap()
        .parse()
        .unwrap();

    let upscale_options: Vec<&str> =
        vec!["2", "4", "8", "16", "32"];
    let upscale = Select::new(
        "Upscale factor",
        upscale_options,
    )
        .with_help_message("rimpy uses 2x by default, and most players only need 2x.")
        .prompt()
        .unwrap()
        .to_owned();

    (target_dir, dimension_limit, upscale)
}

fn process_image(entry: DirEntry, dimension_limit: u32, upscale_factor: &str) -> anyhow::Result<()> {
    let (width, height) = image::image_dimensions(entry.path())?;
    if u32::min(width, height) <= dimension_limit {
        let up_path = entry.path().with_file_name(format!(
            "{}.upscaled.png",
            entry.file_name().to_string_lossy()
        ));
        let mut cmd = Command::new("./waifu2x-ncnn-vulkan.exe");
        cmd.args([
            "-i",
            &entry.path().display().to_string(),
            "-o",
            &up_path.display().to_string(),
            "-n",
            "1",
            "-s",
            upscale_factor,
            "-m",
            "models-upconv_7_anime_style_art_rgb",
        ]);
        let output = cmd.output().unwrap();
        if output.status.success() {
            texconv(
                &up_path.display().to_string(),
                &entry.path().parent().unwrap().display().to_string(),
            )?;

            rename(
                &up_path.with_extension("dds"),
                &up_path
                    .with_file_name(entry.file_name())
                    .with_extension("dds"),
            )
            .with_context(|| "rename")?;
            remove_file(&up_path).with_context(|| "remove")?;
        } else {
            anyhow::bail!("waifu2x error: {}", std::str::from_utf8(&output.stderr)?);
        }
    } else {
        //only conv
        texconv(
            &entry.path().display().to_string(),
            &entry.path().parent().unwrap().display().to_string(),
        )?;
    }
    Ok(())
}

fn texconv(input: &str, output_dir: &str) -> anyhow::Result<()> {
    let mut cmd = Command::new("./texconv.exe");
    cmd.args(["-y", "-o", output_dir, input]);
    let output = cmd.output()?;
    if !output.status.success() {
        anyhow::bail!("texconv error: {:?}", output)
    }
    Ok(())
}
