use crate::prelude::*;

pub fn transform_2d(pos: &Vec2, mat: &Mat4) -> Vec2 {
    let pos4 = Vec4::new(pos.x, pos.y, 0.0, 1.0);
    (*mat * pos4).xy()
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
