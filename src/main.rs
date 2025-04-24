mod geometry;
use rayon::prelude::*;
use std::fs::OpenOptions;
use std::io::Result;
use std::path::PathBuf;

use clap::Parser;
use geometry::{StlMesh, VolumeInfo};

#[derive(Parser)]
#[command(author, version, about, long_about = None, arg_required_else_help = true)]
struct Args {
    /// Absolute path(s) to directory containing stl files or individual .stl files.
    /// Examples: rusty-stl /path/to/folder some_file.stl
    paths: Vec<std::path::PathBuf>,
}

type FileList = Vec<std::path::PathBuf>;

fn get_filenames(args: Args) -> FileList {
    use glob::glob;
    let mut files: FileList = Vec::new();

    for path in args.paths.iter() {
        if path.is_dir() {
            let pstr = path.to_str().unwrap();
            let pstar = format!("{pstr}/*.stl");
            for entry in glob(&pstar).unwrap() {
                match entry {
                    Ok(p) => files.push(p),
                    Err(e) => println!("{:?}", e),
                }
            }
        }
        if path.is_file() {
            files.push(path.to_path_buf());
        }
    }
    files
}

fn process_file(path: &PathBuf) -> Result<VolumeInfo> {
    let mut file = OpenOptions::new().read(true).open(path)?;
    let stl = stl_io::read_stl(&mut file)?;
    let stl_mesh: StlMesh = StlMesh::new_from_indexed_mesh(&stl);
    let info: VolumeInfo = stl_mesh.into();
    Ok(info)
}

fn process_files(files: &[PathBuf]) -> Vec<(&PathBuf, VolumeInfo)> {
    files
        .par_iter()
        .flat_map(|path| match process_file(path) {
            Err(e) => {
                println!("Error opening file {} - {}", path.display(), e);
                Option::None
            }
            Ok(info) => Some((path, info)),
        })
        .collect()
}

fn main() -> Result<()> {
    let args = Args::parse();
    let files = get_filenames(args);
    let volumes = process_files(files.as_slice());

    println!(
        "{:<70} | {:<20} | {:<20} | {:<20} | {:<20}",
        "Filename", "Mesh volume", "Bounding box volume", "Convex volume", "Thickness"
    );

    for (path, vol) in volumes.iter() {
        let s = path.file_name().unwrap().to_str().unwrap();
        println!(
            "{:<70} | {:<20.2} | {:<20.2} | {:<20.2} | {:<20.2}",
            s,
            vol.mesh / 1e6,
            vol.bounding_box / 1e6,
            vol.convex_volume / 1e6,
            vol.thickness
        );
    }

    Ok(())
}
