use crate::prelude::*;

pub fn transform_2d(pos: Vec2, mat: Mat4) -> Vec2 {
    let pos4 = Vec4::new(pos.x, pos.y, 0.0, 1.0);
    (mat * pos4).xy()
}

fn ray_triangle_intersection(
    origin: Vec3,
    direction: Vec3,
    t_a: Vec3,
    t_b: Vec3,
    t_c: Vec3,
) -> Option<Vec3> {
    let e1 = t_b - t_a;
    let e2 = t_c - t_a;

    let ray_cross_e2 = direction.cross(e2);
    let det = e1.dot(ray_cross_e2);

    if det > -f32::EPSILON && det < f32::EPSILON {
        return None; // This ray is parallel to this triangle.
    }

    let inv_det = 1.0 / det;
    let s = origin - t_a;
    let u = inv_det * s.dot(ray_cross_e2);
    if u < 0.0 || u > 1.0 {
        return None;
    }

    let s_cross_e1 = s.cross(e1);
    let v = inv_det * direction.dot(s_cross_e1);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    // At this stage we can compute t to find out where the intersection point is on the line.
    let t = inv_det * e2.dot(s_cross_e1);

    if t > f32::EPSILON {
        // ray intersection
        let intersection_point = origin + direction * t;
        return Some(intersection_point);
    } else {
        // This means that there is a line intersection but not a ray intersection.
        return None;
    }
}

pub fn unit_triangle() -> Vec<Vec3> {
    vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ]
}

pub fn cube_triangles() -> Vec<Vec3> {
    let left_bottom_front = Vec3::new(0.0, 0.0, 0.0);
    let right_bottom_front = Vec3::new(1.0, 0.0, 0.0);
    let left_top_front = Vec3::new(0.0, 1.0, 0.0);
    let right_top_front = Vec3::new(1.0, 1.0, 0.0);
    let left_bottom_back = Vec3::new(0.0, 0.0, 1.0);
    let right_bottom_back = Vec3::new(1.0, 0.0, 1.0);
    let left_top_back = Vec3::new(0.0, 1.0, 1.0);
    let right_top_back = Vec3::new(1.0, 1.0, 1.0);
    vec![
        // Front face
        left_bottom_front,
        right_bottom_front,
        left_top_front,
        left_top_front,
        right_bottom_front,
        right_top_front,
        // Left face
        left_bottom_back,
        left_bottom_front,
        left_top_back,
        left_top_back,
        left_bottom_front,
        left_top_front,
        // Right face
        right_bottom_front,
        right_bottom_back,
        right_top_front,
        right_top_front,
        right_bottom_back,
        right_top_back,
        // Back face
        right_bottom_back,
        left_bottom_back,
        right_top_back,
        right_top_back,
        left_bottom_back,
        left_top_back,
        // Top face
        left_top_front,
        right_top_front,
        left_top_back,
        left_top_back,
        right_top_front,
        right_top_back,
        // Bottom face
        left_bottom_back,
        right_bottom_back,
        left_bottom_front,
        left_bottom_front,
        right_bottom_back,
        right_bottom_front,
    ]
}

pub fn cone_triangles() -> Vec<Vec3> {
    let mut triangles = vec![];
    let num_segments = 8;
    let radius = 0.25;
    let height = 1.0;

    for i in 0..num_segments {
        let angle1 = (i as f32) * (2.0 * PI / num_segments as f32);
        let angle2 = ((i + 1) % num_segments) as f32 * (2.0 * PI / num_segments as f32);

        let p1 = Vec3::new(radius * angle1.cos(), radius * angle1.sin(), 0.0);
        let p2 = Vec3::new(radius * angle2.cos(), radius * angle2.sin(), 0.0);
        let p3 = Vec3::new(0.0, 0.0, height);

        // Cone base segment
        triangles.push(Vec3::ZERO);
        triangles.push(p1);
        triangles.push(p2);

        // Cone side (out of order to make it anticlockwise)
        triangles.push(p1);
        triangles.push(p3);
        triangles.push(p2);
    }
    triangles
}

// fn intersect_grid_1d(cube_size: i32, ray_start: f32, ray_end: f32) -> Vec<i32> {
// }

#[derive(PartialEq)]
enum CheckFace {
    Front,
    Back,
    Both,
}

fn plane_ray_intersection(
    plane_normal: Vec3,
    plane_point: Vec3,
    ray_origin: Vec3,
    ray_direction: Vec3,
    check_face: CheckFace,
) -> Option<Vec3> {
    let epsilon = 0.0001;
    let denom = plane_normal.dot(ray_direction);
    if (check_face == CheckFace::Front && denom < epsilon)
        || (check_face == CheckFace::Back && denom > epsilon)
        || (check_face == CheckFace::Both && denom.abs() > epsilon)
    {
        let v = plane_point - ray_origin;
        let t = v.dot(plane_normal) / denom;
        if t >= 0.0 {
            return Some(ray_origin + ray_direction * t);
        }
    }
    return None;
}

pub fn ray_unitcube_intersection(
    ray_origin: Vec3,
    ray_dir: Vec3,
    cube_corner: UVec3,
) -> Option<Vec3> {
    let inv_dir = ray_dir.recip();
    let cube_corner = cube_corner.as_vec3();

    let min = cube_corner;
    let max = cube_corner + Vec3::splat(1.0);

    let t1 = (min - ray_origin) * inv_dir;
    let t2 = (max - ray_origin) * inv_dir;

    let tmin = t1.min(t2);
    let tmax = t1.max(t2);

    let t_enter = tmin.max_element();
    let t_exit = tmax.min_element();

    if t_exit >= t_enter && t_exit >= 0.0 {
        let t_hit = t_enter.max(0.0); // Clamp to zero if ray starts inside the cube
        Some(ray_origin + ray_dir * t_hit)
    } else {
        None
    }
}

pub fn rotation_from_z_axis_to_direction(target_dir: Vec3) -> Quat {
    debug_assert!(target_dir.is_normalized());

    let from = Vec3::Z;

    // If already aligned, or isn't normalized (failure state, see assertion above)
    if from.abs_diff_eq(target_dir, 1e-6) || !target_dir.is_normalized() {
        Quat::IDENTITY
    }
    // If opposite
    else if from.abs_diff_eq(-target_dir, 1e-6) {
        // 180-degree rotation around an arbitrary axis perpendicular to from
        let axis = if from.cross(Vec3::X).length_squared() > 1e-6 {
            from.cross(Vec3::X).normalize()
        } else {
            from.cross(Vec3::Y).normalize()
        };
        Quat::from_axis_angle(axis, std::f32::consts::PI)
    }
    // General case
    else {
        Quat::from_rotation_arc(from, target_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plane_ray_intersection() {
        let r_origin = Vec3::new(1.5, 0.5, -1.0);
        let target = Vec3::new(0.0, 0.5, 1.0);
        let r_dir = (target - r_origin).normalize();

        let p_point = Vec3::new(0.0, 0.0, 10.0);
        let mut p_norm = Vec3::new(0.0, 0.0, 0.9).normalize();

        let i = plane_ray_intersection(p_norm, p_point, r_origin, r_dir, CheckFace::Back);
        dbg!(i);
        assert!(i.is_some());
        let i = plane_ray_intersection(p_norm, p_point, r_origin, r_dir, CheckFace::Both);
        dbg!(i);
        assert!(i.is_some());
        let i = plane_ray_intersection(p_norm, p_point, r_origin, r_dir, CheckFace::Front);
        dbg!(i);
        assert!(i.is_none());

        p_norm.z *= -1.0;

        let i = plane_ray_intersection(p_norm, p_point, r_origin, r_dir, CheckFace::Back);
        dbg!(i);
        assert!(i.is_none());
        let i = plane_ray_intersection(p_norm, p_point, r_origin, r_dir, CheckFace::Both);
        dbg!(i);
        assert!(i.is_some());
        let i = plane_ray_intersection(p_norm, p_point, r_origin, r_dir, CheckFace::Front);
        dbg!(i);
        assert!(i.is_some());
    }
}

pub fn sphere_triangles() -> Vec<Vec3> {
    // determines the level of detail (higher = more triangles).
    let subdivisions = 1;

    // Nested function to find or create the middle point
    fn get_middle_point(
        p1: usize,
        p2: usize,
        vertices: &mut Vec<Vec3>,
        cache: &mut HashMap<(usize, usize), usize>,
    ) -> usize {
        // Ensure consistent ordering of the key
        let key = if p1 < p2 { (p1, p2) } else { (p2, p1) };

        // Check if the middle point is already in the cache
        if let Some(&index) = cache.get(&key) {
            return index;
        }

        // Calculate the middle point and normalize it
        let middle = (vertices[p1] + vertices[p2]).normalize();
        let index = vertices.len();
        vertices.push(middle);

        // Store it in the cache
        cache.insert(key, index);

        index
    }

    // Create the initial icosahedron
    let t = (1.0 + 5.0f32.sqrt()) / 2.0;

    let mut vertices = vec![
        Vec3::new(-1.0, t, 0.0).normalize(),
        Vec3::new(1.0, t, 0.0).normalize(),
        Vec3::new(-1.0, -t, 0.0).normalize(),
        Vec3::new(1.0, -t, 0.0).normalize(),
        Vec3::new(0.0, -1.0, t).normalize(),
        Vec3::new(0.0, 1.0, t).normalize(),
        Vec3::new(0.0, -1.0, -t).normalize(),
        Vec3::new(0.0, 1.0, -t).normalize(),
        Vec3::new(t, 0.0, -1.0).normalize(),
        Vec3::new(t, 0.0, 1.0).normalize(),
        Vec3::new(-t, 0.0, -1.0).normalize(),
        Vec3::new(-t, 0.0, 1.0).normalize(),
    ];

    let mut faces = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];

    // Subdivide the faces
    let mut middle_point_cache = HashMap::new();
    for _ in 0..subdivisions {
        let mut new_faces = Vec::new();
        for face in &faces {
            let [a, b, c] = *face;

            // Get middle points
            let ab = get_middle_point(a, b, &mut vertices, &mut middle_point_cache);
            let bc = get_middle_point(b, c, &mut vertices, &mut middle_point_cache);
            let ca = get_middle_point(c, a, &mut vertices, &mut middle_point_cache);

            // Create new faces
            new_faces.push([a, ab, ca]);
            new_faces.push([b, bc, ab]);
            new_faces.push([c, ca, bc]);
            new_faces.push([ab, bc, ca]);
        }
        faces = new_faces;
    }

    // Convert faces into a flat list of triangles
    let mut triangles = Vec::new();
    for face in faces {
        triangles.push(vertices[face[0]]);
        triangles.push(vertices[face[2]]);
        triangles.push(vertices[face[1]]);
    }

    triangles
}
