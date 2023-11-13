// Read .stl files and calculate volumes
// IO components mostly copied from 'https://docs.rs/stl_io/latest/stl_io/"

use std::fs::OpenOptions;
use std::io::Result;

use stl_io::{IndexedMesh, Vector};

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
        (self.xmax - self.xmin).abs() * (self.ymax - self.ymin).abs() * (self.zmax - self.zmin).abs()
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


fn main() -> Result<()> {
    let mut file = OpenOptions::new()
        .read(true)
        .open("test-steering-wheel.stl")?;
    let stl = stl_io::read_stl(&mut file)?;
    let bbox = BoundingBox::from(&stl);
    println!("Box:    {:.?}", bbox);
    let mesh_vol = volume(&stl);
    let bbox_vol = bbox.volume();

    println!("Mesh Volume:    {}", mesh_vol / 1e6);
    println!("Box Volume:     {}", bbox_vol / 1e6);
    Ok(())
}
