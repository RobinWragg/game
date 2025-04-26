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

fn ray_unitcube_intersection(ray_origin: Vec3, ray_dir: Vec3, cube_corner: IVec3) -> Option<Vec3> {
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

pub fn closest_ray_grid_intersection(
    ray_origin: Vec3,
    ray_dir: Vec3,
    cubes: &[IVec3],
) -> Option<(IVec3, Vec3)> {
    let mut intersections = vec![];

    for cube in cubes {
        if let Some(intersection) = ray_unitcube_intersection(ray_origin, ray_dir, *cube) {
            intersections.push((*cube, intersection));
        }
    }

    if intersections.is_empty() {
        return None;
    }

    let half = Vec3::splat(0.5);
    let sorter = |a: &(IVec3, Vec3), b: &(IVec3, Vec3)| {
        let a_dist = ray_origin.distance(a.1);
        let b_dist = ray_origin.distance(b.1);
        a_dist.partial_cmp(&b_dist).unwrap()
    };

    intersections.sort_by(sorter);
    Some(intersections[0])
}

pub fn adjacent_cube(origin_cube: IVec3, nearby_pos: Vec3) -> IVec3 {
    let origin = origin_cube.as_vec3() + Vec3::splat(0.5);
    let mut candidates = [
        origin + Vec3::new(-1.0, 0.0, 0.0),
        origin + Vec3::new(1.0, 0.0, 0.0),
        origin + Vec3::new(0.0, -1.0, 0.0),
        origin + Vec3::new(0.0, 1.0, 0.0),
        origin + Vec3::new(0.0, 0.0, -1.0),
        origin + Vec3::new(0.0, 0.0, 1.0),
    ];

    let sorter = |a: &Vec3, b: &Vec3| {
        let a_dist = nearby_pos.distance(*a);
        let b_dist = nearby_pos.distance(*b);
        a_dist.partial_cmp(&b_dist).unwrap()
    };

    candidates.sort_by(sorter);
    let closest = candidates[0];
    IVec3::new(
        closest.x.floor() as i32,
        closest.y.floor() as i32,
        closest.z.floor() as i32,
    )
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
