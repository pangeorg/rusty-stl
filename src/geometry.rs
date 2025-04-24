use std::fmt::{self, Display};
use stl_io::IndexedMesh;

use parry3d::mass_properties::details::trimesh_signed_volume_and_center_of_mass;
use parry3d::math::{Point, Vector};
use parry3d::na::Vector3;
use parry3d::query::{Ray, RayCast};
use parry3d::shape::{TriMesh, TriMeshFlags};

use parry3d::transformation::{self};

pub struct VolumeInfo {
    pub mesh: f32,
    pub bounding_box: f32,
    pub thickness: f32,
    pub convex_volume: f32,
}

impl Display for VolumeInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:<7.2}{:<7.2}{:<7.2}{:<7.2}",
            self.mesh / 1e6,
            self.bounding_box / 1e6,
            self.convex_volume / 1e6,
            self.thickness
        )
    }
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
            let area = triangle_area_2d(&v1_proj, &v2_proj, &v3_proj);
            if area > 0.0 {
                total_area += area;
            }
        }

        total_area
    }

    pub fn calculate_thickness(&self) -> f32 {
        let mut thicknesses: Vec<f32> = Vec::new();

        for triangle in self.mesh.triangles() {
            let v1 = triangle.a;
            let v2 = triangle.b;
            let v3 = triangle.c;

            let p1: Vector3<f32> = Vector3::new(v1.x, v1.y, v1.z);
            let p2: Vector3<f32> = Vector3::new(v2.x, v2.y, v2.z);
            let p3: Vector3<f32> = Vector3::new(v3.x, v3.y, v3.z);

            let a = p2 - p1;
            let b = p3 - p1;
            let n = a.cross(&b);

            let midpoint: Point<f32> = Point::new(
                (v1.x + v2.x + v3.x) / 3.0,
                (v1.y + v2.y + v3.y) / 3.0,
                (v1.z + v2.z + v3.z) / 3.0,
            );

            let mut ray = Ray::new(midpoint, n);
            let location_a = self.mesh.cast_local_ray(&ray, std::f32::MAX, true);

            ray = Ray::new(midpoint, -n);
            let location_b = self.mesh.cast_local_ray(&ray, std::f32::MAX, true);

            if let (Some(location_a), Some(location_b)) = (location_a, location_b) {
                let thickness = location_a - location_b;
                if thickness > 0.5 && thickness < 4.0 {
                    thicknesses.push(thickness);
                }
            }
        }

        if thicknesses.len() > 0 {
            thicknesses.sort_by(f32::total_cmp);
            let mid = thicknesses.len() / 2;
            thicknesses[mid]
        } else {
            0.0
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
            thickness: self.calculate_thickness(),
            convex_volume: self.convex().mesh_volume(),
            mesh: self.mesh_volume(),
        }
    }
}

/// Project a 3D point onto a plane defined by its normal.
#[allow(dead_code)]
fn project_point_onto_plane(point: &Point<f32>, plane_normal: &Vector<f32>) -> Point<f32> {
    let normal = plane_normal.normalize();
    let distance = point.coords.dot(&normal);
    Point::new(
        point.x - distance * normal.x,
        point.y - distance * normal.y,
        point.z - distance * normal.z,
    )
}

/// Calculate the area of a 2D triangle using the shoelace formula.
#[allow(dead_code)]
fn triangle_area_2d(v1: &Point<f32>, v2: &Point<f32>, v3: &Point<f32>) -> f32 {
    (v1.x * (v2.y - v3.y) + v2.x * (v3.y - v1.y) + v3.x * (v1.y - v2.y)) / 2.0
}
