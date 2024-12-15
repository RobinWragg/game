use crate::prelude::*;

pub fn transform_2d(pos: &Vec2, mat: &Mat4) -> Vec2 {
    let pos4 = Vec4::new(pos.x, pos.y, 0.0, 1.0);
    (*mat * pos4).xy()
}

fn moller_trumbore_intersection(
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

fn intersect_grid(cube_size: i32, ray_origin: Vec3, ray_direction: Vec3) -> Vec<(i32, i32, i32)> {
    // TODO: Return all intersections, not just one or zero.
    // TODO: Sort intersected cubes by their position on the ray.
    let triangle_verts = cube_triangles();
    for x in 0..cube_size {
        for y in 0..cube_size {
            for z in 0..cube_size {
                let cube_origin = Vec3::new(x as f32, y as f32, z as f32);
                let transformed_ray_origin = ray_origin - cube_origin;

                for i in (0..triangle_verts.len()).step_by(3) {
                    let a = triangle_verts[i];
                    let b = triangle_verts[i + 1];
                    let c = triangle_verts[i + 2];
                    if let Some(intersection) =
                        moller_trumbore_intersection(transformed_ray_origin, ray_direction, a, b, c)
                    {
                        return vec![(x, y, z)];
                    }
                }
            }
        }
    }
    return [].to_vec();
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_intersect_grid_1_true() {
        use super::*;
        let origin = Vec3::new(1.5, 0.5, -1.0);
        let target = Vec3::new(0.0, 0.5, 1.0);
        let direction = (target - origin).normalize();

        let results = intersect_grid(1, origin, direction);
        assert_eq!(results, vec![(0, 0, 0)]);
    }

    #[test]
    fn test_intersect_grid_1_false() {
        use super::*;
        let origin = Vec3::new(1.5, 0.5, -1.0);
        let target = Vec3::new(2.0, 0.5, 1.0);
        let direction = (target - origin).normalize();

        let results = intersect_grid(1, origin, direction);
        assert_eq!(results, vec![]);
    }
}
