// Read .stl files and calculate volumes
// IO components mostly copied from 'https://docs.rs/stl_io/latest/stl_io/"

use std::fmt::{self, Display};
use std::fs::OpenOptions;
use std::io::Result;

use clap::Parser;
use stl_io::{IndexedMesh, Vector};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    paths: Vec<std::path::PathBuf>,
}

#[derive(Debug)]
struct BoundingBox {
    xmin: f32,
    xmax: f32,
    ymin: f32,
    ymax: f32,
    zmin: f32,
    zmax: f32,
}

impl BoundingBox {
    pub fn volume(&self) -> f32 {
        (self.xmax - self.xmin).abs()
            * (self.ymax - self.ymin).abs()
            * (self.zmax - self.zmin).abs()
    }
}

impl From<&IndexedMesh> for BoundingBox {
    fn from(mesh: &IndexedMesh) -> Self {
        let mut bbox = BoundingBox {
            xmin: f32::MAX,
            xmax: f32::MIN,
            ymin: f32::MAX,
            ymax: f32::MIN,
            zmin: f32::MAX,
            zmax: f32::MIN,
        };
        for vertex in mesh.vertices.iter() {
            bbox.xmin = bbox.xmin.min(vertex[0]);
            bbox.ymin = bbox.ymin.min(vertex[1]);
            bbox.zmin = bbox.zmin.min(vertex[2]);

            bbox.xmax = bbox.xmax.max(vertex[0]);
            bbox.ymax = bbox.ymax.max(vertex[1]);
            bbox.zmax = bbox.xmax.max(vertex[2]);
        }
        bbox
    }
}

fn volume(mesh: &IndexedMesh) -> f32 {
    let mut sum = 0.0;
    // we should have them as shared references at some point i'd say
    let mut p1: Vector<f32>;
    let mut p2: Vector<f32>;
    let mut p3: Vector<f32>;
    for face in mesh.faces.iter() {
        p1 = mesh.vertices[face.vertices[0]];
        p2 = mesh.vertices[face.vertices[1]];
        p3 = mesh.vertices[face.vertices[2]];
        sum += vol_triangle_sgn(&p1, &p2, &p3);
    }
    sum
}

fn vector_dot(a: &Vector<f32>, b: &Vector<f32>) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn vector_cross(a: &Vector<f32>, b: &Vector<f32>) -> Vector<f32> {
    let x = a[1] * b[2] - a[2] * b[1];
    let y = a[2] * b[0] - a[0] * b[2];
    let z = a[0] * b[1] - a[1] * b[0];
    let v: [f32; 3] = [x, y, z];
    Vector::new(v)
}

fn vol_triangle_sgn(p1: &Vector<f32>, p2: &Vector<f32>, p3: &Vector<f32>) -> f32 {
    let c = vector_cross(p2, p3);
    vector_dot(p1, &c) / 6.0
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

struct VolumeInfo {
    filename: String,
    mesh: f32,
    bounding_box: f32,
}

impl Display for VolumeInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:<30}{:<7.2}{:<7.2}",
            self.filename,
            self.mesh / 1e6,
            self.bounding_box / 1e6
        )
    }
}

impl From<&IndexedMesh> for VolumeInfo {
    fn from(mesh: &IndexedMesh) -> Self {
        let info = VolumeInfo {
            filename: String::new(),
            mesh: volume(mesh),
            bounding_box: BoundingBox::from(mesh).volume(),
        };
        info
    }
}

fn process_files(files: FileList) -> Vec<VolumeInfo> {
    let mut infos: Vec<VolumeInfo> = Vec::new();
    for path in files.iter() {
        match OpenOptions::new().read(true).open(path) {
            Err(e) => println!("Error opening file {} - {}", path.display(), e),
            Ok(mut file) => match stl_io::read_stl(&mut file) {
                Err(e) => println!("Error opening file {} - {}", path.display(), e),
                Ok(stl) => {
                    let mut info = VolumeInfo::from(&stl);
                    // this is fuckin ugly
                    info.filename = path.file_name().unwrap().to_str().unwrap().to_string();
                    infos.push(info);
                }
            },
        }
    }
    infos
}

fn main() -> Result<()> {
    let args = Args::parse();
    let files = get_filenames(args);
    let volumes = process_files(files);

    for vol in volumes.iter() {
        println!("{}", vol);
    }

    Ok(())
}
