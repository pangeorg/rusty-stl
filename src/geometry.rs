use core::f32;

use parry3d::na::Isometry3;
use stl_io::IndexedMesh;

use parry3d::mass_properties::details::trimesh_signed_volume_and_center_of_mass;
use parry3d::math::{Point, Vector};
use parry3d::query::{Ray, RayCast};
use parry3d::shape::{TriMesh, TriMeshFlags, Triangle};
use parry3d::utils::median;

use parry3d::transformation::{self};

pub struct VolumeInfo {
    pub mesh: f32,
    pub bounding_box: f32,
    pub thickness: Statistics,
    pub convex_volume: f32,
}

pub struct Statistics {
    pub avg: f32,
    pub median: f32,
    pub std_dev: f32,
    pub thicknesses: Vec<f32>,
}

#[derive(Clone, Copy)]
pub struct OutlierLimits {
    pub min: f32,
    pub max: f32,
}

pub struct StlMesh {
    mesh: TriMesh,
}

#[allow(dead_code)]
impl StlMesh {
    pub fn new(vertices: &[f32], indices: &[u32]) -> StlMesh {
        let vertices: Vec<Point<f32>> = vertices
            .to_vec()
            .chunks(3)
            .map(|chunk| Point::new(chunk[0], chunk[1], chunk[2]))
            .collect();

        let indices: Vec<[u32; 3]> = indices
            .to_vec()
            .chunks(3)
            .map(|chunk| [chunk[0], chunk[1], chunk[2]])
            .collect();

        let mesh = TriMesh::with_flags(vertices, indices, TriMeshFlags::all())
            .expect("Could not create trimesh");

        StlMesh { mesh }
    }

    pub fn new_from_indexed_mesh(mesh: &IndexedMesh) -> StlMesh {
        mesh.into()
    }

    pub fn vertex_count(&self) -> u32 {
        self.mesh.vertices().len() as u32
    }

    pub fn mesh_volume(&self) -> f32 {
        trimesh_signed_volume_and_center_of_mass(self.mesh.vertices(), self.mesh.indices()).0
    }

    pub fn facing_area(&self, plane_normal: &[f32]) -> f32 {
        if plane_normal.len() != 3 {
            panic!("Provide 3D normal");
        }

        let normal = Vector::new(plane_normal[0], plane_normal[1], plane_normal[2]).normalize();

        let mut total_area = 0.0;

        // Iterate over all triangles in the mesh
        for triangle in self.mesh.triangles() {
            // Project each vertex onto the plane
            let v1_proj = project_point_onto_plane(&triangle.a, &normal);
            let v2_proj = project_point_onto_plane(&triangle.b, &normal);
            let v3_proj = project_point_onto_plane(&triangle.c, &normal);

            // Calculate the area of the projected triangle
            let t = Triangle::new(v1_proj, v2_proj, v3_proj);
            let area = t.area();
            if area > 0.0 {
                total_area += area;
            }
        }

        total_area
    }

    pub fn calculate_thickness(&self, outlier_range: Option<OutlierLimits>) -> Statistics {
        let mut thicknesses: Vec<f32> = Vec::new();
        let mut areas: Vec<f32> = Vec::new();

        let bounds = match outlier_range {
            Some(r) => r,
            None => OutlierLimits {
                min: 0.,
                max: std::f32::MAX,
            },
        };

        for triangle in self.mesh.triangles() {
            let p1: Point<f32> = triangle.a;
            let p2: Point<f32> = triangle.b;
            let p3: Point<f32> = triangle.c;

            let n = triangle.normal().unwrap().into_inner();

            let midpoint: Point<f32> = Point::new(
                (p1.x + p2.x + p3.x) / 3.0,
                (p1.y + p2.y + p3.y) / 3.0,
                (p1.z + p2.z + p3.z) / 3.0,
            );

            let mut ray = Ray::new(midpoint, n);
            let da = self
                .mesh
                .cast_ray(&Isometry3::identity(), &ray, 100., false);

            ray = Ray::new(midpoint, -n);
            let db = self
                .mesh
                .cast_ray(&Isometry3::identity(), &ray, 100., false);

            if let (Some(da), Some(db)) = (da, db) {
                let thickness = if da > db { da } else { db };
                if thickness > bounds.min && thickness < bounds.max {
                    thicknesses.push(thickness);
                    areas.push(triangle.area());
                }
            }
        }

        let total_area: f32 = areas.iter().sum();
        let avg: f32 = thicknesses
            .iter()
            .zip(areas)
            .map(|(t, a)| t * a / total_area)
            .sum();

        let median = median(&mut thicknesses);
        let std_dev = std(&thicknesses, avg);

        Statistics {
            std_dev,
            avg,
            median,
            thicknesses,
        }
    }

    pub fn convex(&self) -> StlMesh {
        let convex_hull = transformation::convex_hull(self.mesh.vertices());
        let mesh = TriMesh::with_flags(convex_hull.0, convex_hull.1, TriMeshFlags::all())
            .expect("Could not create trimesh");
        StlMesh { mesh }
    }
}

impl From<&IndexedMesh> for StlMesh {
    fn from(mesh: &IndexedMesh) -> Self {
        let vertices: Vec<Point<f32>> = mesh
            .vertices
            .iter()
            .map(|v| Point::new(v[0], v[1], v[2]))
            .collect();

        let indices: Vec<[u32; 3]> = mesh
            .faces
            .iter()
            .map(|tri| {
                [
                    tri.vertices[0] as u32,
                    tri.vertices[1] as u32,
                    tri.vertices[2] as u32,
                ]
            })
            .collect();

        let mesh = TriMesh::with_flags(vertices, indices, TriMeshFlags::all())
            .expect("Could not create trimesh");

        StlMesh { mesh }
    }
}

impl Into<VolumeInfo> for StlMesh {
    fn into(self) -> VolumeInfo {
        VolumeInfo {
            bounding_box: self.mesh.local_aabb().volume(),
            thickness: self.calculate_thickness(None),
            convex_volume: self.convex().mesh_volume(),
            mesh: self.mesh_volume(),
        }
    }
}

fn average(it: &[f32]) -> f32 {
    it.iter().sum::<f32>() / it.len() as f32
}

#[allow(dead_code)]
fn mad(it: &[f32], m: f32) -> f32 {
    let mut dev = it.into_iter().map(|t| (t - m).abs()).collect::<Vec<f32>>();
    median(dev.as_mut_slice()) * 1.4826
}

fn std(it: &[f32], avg: f32) -> f32 {
    let std_dev: f32 = it.iter().map(|t| (t - avg) * (t - avg)).sum::<f32>() / it.len() as f32;
    std_dev.sqrt()
}

/// Project a 3D point onto a plane defined by its normal.
fn project_point_onto_plane(point: &Point<f32>, plane_normal: &Vector<f32>) -> Point<f32> {
    let normal = plane_normal.normalize();
    let distance = point.coords.dot(&normal);
    Point::new(
        point.x - distance * normal.x,
        point.y - distance * normal.y,
        point.z - distance * normal.z,
    )
}
