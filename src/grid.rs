use crate::math::{cube_triangles, ray_unitcube_intersection};
use crate::prelude::*;
use dot_vox;

pub mod grid2d;

fn transtellar_list() -> Vec<AtomWithPos> {
    let vox = dot_vox::load("nopush/Transtellar/Transtellar.vox").unwrap();
    assert!(vox.models.len() == 1);
    let model = &vox.models[0];
    dbg!(model.voxels.len());

    let mut atoms: Vec<AtomWithPos> = model
        .voxels
        .iter()
        .map(|voxel| {
            AtomWithPos::with_color(
                IVec3::new(voxel.x as i32, voxel.z as i32, voxel.y as i32),
                Vec4::new(
                    vox.palette[voxel.i as usize].r as f32 / 255.0,
                    vox.palette[voxel.i as usize].g as f32 / 255.0,
                    vox.palette[voxel.i as usize].b as f32 / 255.0,
                    1.0,
                ),
            )
        })
        .collect();
    // atoms = atoms.split_at(8 * 8 * 8).0.to_vec();

    // Center the atoms around the origin
    atoms = {
        let mut minimum = IVec3::splat(i32::MAX);
        let mut maximum = IVec3::splat(i32::MIN);
        for atom in &atoms {
            minimum = atom.pos.min(minimum);
            maximum = atom.pos.max(maximum);
        }
        let center = (maximum + minimum) / 2;
        for atom in &mut atoms {
            atom.pos -= center;
        }

        // axes
        for i in 1..32 {
            atoms.push(AtomWithPos::with_color(
                IVec3::new(i, 0, 0),
                Vec4::new(1.0, 0.0, 0.0, 1.0),
            ));
            atoms.push(AtomWithPos::with_color(
                IVec3::new(0, i, 0),
                Vec4::new(0.0, 1.0, 0.0, 1.0),
            ));
            atoms.push(AtomWithPos::with_color(
                IVec3::new(0, 0, i),
                Vec4::new(0.0, 0.0, 1.0, 1.0),
            ));
        }
        atoms
    };

    atoms
}

fn adjacent_atom(origin_atom: IVec3, nearby_pos: Vec3) -> IVec3 {
    let origin = origin_atom.as_vec3() + Vec3::splat(0.5);
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

fn closest_ray_grid_intersection<'a>(
    ray_origin: Vec3,
    ray_dir: Vec3,
    atoms: impl IntoIterator<Item = &'a IVec3>,
) -> Option<(IVec3, Vec3)> {
    let mut intersections = vec![];

    for atom in atoms {
        if let Some(intersection) = ray_unitcube_intersection(ray_origin, ray_dir, *atom) {
            intersections.push((*atom, intersection));
        }
    }

    if intersections.is_empty() {
        return None;
    }

    let sorter = |a: &(IVec3, Vec3), b: &(IVec3, Vec3)| {
        let a_dist = ray_origin.distance(a.1);
        let b_dist = ray_origin.distance(b.1);
        a_dist.partial_cmp(&b_dist).unwrap()
    };

    intersections.sort_by(sorter);
    Some(intersections[0])
}

#[derive(Clone)]
enum Atom {
    Solid(Vec4), // Color. TODO: f32 is gross overkill here.
    Liquid,
    LiquidSource(IVec3),
    Gas,
}

#[derive(Clone)]
struct AtomWithPos {
    pub pos: IVec3, // TODO: i16 or even i8 might be ok here.
    pub variant: Atom,
}

impl AtomWithPos {
    fn with_color(pos: IVec3, color: Vec4) -> Self {
        Self {
            pos,
            variant: Atom::Solid(color),
        }
    }

    fn color(&self) -> Vec4 {
        match &self.variant {
            Atom::Solid(color) => *color,
            Atom::Liquid => Vec4::new(0.0, 1.0, 1.0, 1.0),
            Atom::LiquidSource(_) => Vec4::new(1.0, 1.0, 1.0, 1.0),
            Atom::Gas => Vec4::new(1.0, 0.0, 1.0, 1.0),
        }
    }
}

pub struct Grid {
    atoms: Vec<AtomWithPos>,
}

impl Grid {
    pub fn new() -> Self {
        Self {
            atoms: vec![AtomWithPos {
                pos: IVec3::splat(1),
                variant: Atom::Solid(Vec4::splat(1.0)),
            }],
        }
    }

    fn is_empty(&self) -> bool {
        self.atoms.is_empty()
    }

    fn add(&mut self, pos: IVec3) {
        debug_assert!(
            !self.contains(&pos),
            "AtomWithPos already exists at this position"
        );
        self.atoms.push(AtomWithPos::with_color(
            pos,
            Vec4::new(1.0, rand::random::<f32>(), rand::random::<f32>(), 1.0),
        ));
    }

    fn contains(&self, pos: &IVec3) -> bool {
        self.atoms.iter().any(|atom| atom.pos == *pos)
    }

    fn iter(&self) -> impl Iterator<Item = &AtomWithPos> {
        self.atoms.iter()
    }

    fn positions(&self) -> impl Iterator<Item = &IVec3> {
        self.atoms.iter().map(|atom| &atom.pos)
    }

    fn remove(&mut self, pos: IVec3) {
        // TODO: This will check all atoms even if the atom is found early.
        self.atoms.retain(|atom| atom.pos != pos);
    }

    pub fn hollow_out(&mut self) {
        let atoms_2 = self.atoms.clone();
        self.atoms.retain(|a| {
            atoms_2
                .iter()
                .find(|b| b.pos.x == a.pos.x + 1 && b.pos.y == a.pos.y && b.pos.z == a.pos.z)
                .is_none()
                || atoms_2
                    .iter()
                    .find(|b| b.pos.x == a.pos.x && b.pos.y == a.pos.y + 1 && b.pos.z == a.pos.z)
                    .is_none()
                || atoms_2
                    .iter()
                    .find(|b| b.pos.x == a.pos.x && b.pos.y == a.pos.y && b.pos.z == a.pos.z + 1)
                    .is_none()
                || atoms_2
                    .iter()
                    .find(|b| b.pos.x == a.pos.x - 1 && b.pos.y == a.pos.y && b.pos.z == a.pos.z)
                    .is_none()
                || atoms_2
                    .iter()
                    .find(|b| b.pos.x == a.pos.x && b.pos.y == a.pos.y - 1 && b.pos.z == a.pos.z)
                    .is_none()
                || atoms_2
                    .iter()
                    .find(|b| b.pos.x == a.pos.x && b.pos.y == a.pos.y && b.pos.z == a.pos.z - 1)
                    .is_none()
        });

        println!("Hollowed out {} atoms", atoms_2.len() - self.atoms.len());
    }
}

pub struct Editor {
    camera_transform: Mat4, // Sans aspect ratio correct for now
    rotation: Vec2,
    mouse_pos: Option<Vec2>,
    highlighted_atom: Option<IVec3>,
    proposed_atom: Option<IVec3>,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            camera_transform: Mat4::IDENTITY,
            rotation: Vec2::ZERO,
            mouse_pos: None,
            highlighted_atom: None,
            proposed_atom: None,
        }
    }

    pub fn update(&mut self, grid: &mut Grid, events: &mut VecDeque<Event>) {
        let mut should_add_atom = false;
        let mut should_remove_atom = false;
        let mut scroll_delta = Vec2::ZERO;

        events.retain(|event| match event {
            Event::MousePos(p) => {
                self.mouse_pos = Some(*p);
                true
            }
            Event::Scroll(s) => {
                scroll_delta = *s;
                false
            }
            Event::LeftClickPressed(_) => {
                should_add_atom = true;
                false
            }
            Event::RightClickPressed(_) => {
                should_remove_atom = true;
                false
            }
            _ => true,
        });

        if grid.is_empty() {
            grid.add(IVec3::splat(0));
        }

        // Rotation TODO: test whether this is framerate dependent
        {
            self.rotation += scroll_delta * -0.002;
            if self.rotation.x > TAU {
                self.rotation.x -= TAU;
            } else if self.rotation.x < -TAU {
                self.rotation.x += TAU;
            }

            let y_rotation_limit = (PI / 2.0) * 0.9;
            self.rotation.y = self.rotation.y.clamp(-y_rotation_limit, y_rotation_limit);
        }

        self.camera_transform = {
            let depth_buffer_resolution = 0.01;
            let arbitrary_scale = Mat4::from_scale(Vec3::new(0.02, 0.02, depth_buffer_resolution));
            // The viable Z range is 0 to 1, so put it in the middle.
            let translate_z = Mat4::from_translation(Vec3::new(0.0, 0.0, 0.5));
            let rotation =
                Mat4::from_rotation_x(self.rotation.y) * Mat4::from_rotation_y(self.rotation.x);
            translate_z * arbitrary_scale * rotation
        };

        let selection = if let Some(mouse_pos) = self.mouse_pos {
            let camera_transform_inv = self.camera_transform.inverse();
            let ray_origin =
                (camera_transform_inv * Vec4::new(mouse_pos.x, mouse_pos.y, 0.0, 1.0)).xyz();
            let ray_direction = (camera_transform_inv * Vec4::new(0.0, 0.0, 1.0, 0.0))
                .xyz()
                .normalize();

            if let Some((atom, intersection_location)) =
                closest_ray_grid_intersection(ray_origin, ray_direction, grid.positions())
            {
                Some((atom, intersection_location))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((highlighted_atom, intersection_location)) = selection {
            self.highlighted_atom = Some(highlighted_atom);
            self.proposed_atom = Some(adjacent_atom(highlighted_atom, intersection_location));
            if self.highlighted_atom == self.proposed_atom {
                self.proposed_atom = None;
            }
        } else {
            self.highlighted_atom = None;
            self.proposed_atom = None;
        }

        if should_add_atom {
            if let Some(proposed_atom) = self.proposed_atom {
                if !grid.contains(&proposed_atom) {
                    grid.add(proposed_atom);
                }
            }
        } else if should_remove_atom {
            if let Some(highlighted_atom) = self.highlighted_atom {
                grid.remove(highlighted_atom);
            }
        }
    }

    pub fn render_ortho(&self, grid: &Grid, gpu: &mut Gpu) {
        gpu.set_render_features(Gpu::FEATURE_DEPTH | Gpu::FEATURE_LIGHT);

        let mesh = Mesh::new(&cube_triangles(), None, None, gpu);

        let half_trans = Mat4::from_translation(Vec3::splat(0.5));
        let shrink = half_trans * Mat4::from_scale(Vec3::splat(1.0)) * half_trans.inverse();

        for atom in grid.iter() {
            let model_transform = Mat4::from_translation(atom.pos.as_vec3()) * shrink;
            let total_transform = self.camera_transform * model_transform;

            let color = if self.highlighted_atom == Some(atom.pos) {
                Some(Vec4::new(0.0, 1.0, 0.0, 1.0))
            } else {
                Some(atom.color())
            };

            gpu.render_mesh(&mesh, &total_transform, color);
        }

        if let Some(proposed_atom) = self.proposed_atom {
            let model_transform = Mat4::from_translation(proposed_atom.as_vec3()) * shrink;
            let total_transform = self.camera_transform * model_transform;
            gpu.render_mesh(&mesh, &total_transform, Some(Vec4::new(0.0, 1.0, 1.0, 1.0)));
        }
    }
}

pub struct Viewer {}

impl Viewer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(&self, grid: &Grid, global_translation: Vec2, gpu: &mut Gpu) {
        gpu.set_render_features(Gpu::FEATURE_DEPTH);

        let verts = [
            // front
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
            Vec3::new(4.0, 3.0, 0.0),
            Vec3::new(4.0, 3.0, 0.0),
            Vec3::new(0.0, 3.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            // top
            Vec3::new(0.0, 3.0, 0.0),
            Vec3::new(4.0, 3.0, 0.0),
            Vec3::new(4.0, 4.0, 0.0),
            Vec3::new(4.0, 4.0, 0.0),
            Vec3::new(0.0, 4.0, 0.0),
            Vec3::new(0.0, 3.0, 0.0),
        ];
        let front_intensity = Vec3::splat(0.7).extend(1.0);
        let top_intensity = Vec4::splat(1.0);
        let intensities = [
            // front
            front_intensity,
            front_intensity,
            front_intensity,
            front_intensity,
            front_intensity,
            front_intensity,
            // top
            top_intensity,
            top_intensity,
            top_intensity,
            top_intensity,
            top_intensity,
            top_intensity,
        ];

        let mesh = Mesh::new(&verts, Some(&intensities), None, gpu);
        let camera_transform = Mat4::from_translation(global_translation.extend(0.5))
            * Mat4::from_scale(Vec3::splat(0.005));

        let xhat = Vec3::new(2.0, 1.0, 1.0);
        let yhat = Vec3::new(0.0, 3.0, -1.0); // TODO: could do 0,3,0 instead and handle the depth using the mesh.
        let zhat = Vec3::new(-2.0, 1.0, 1.0);
        let isometric_transform_cpu = Mat3::from_cols(xhat, yhat, zhat);

        for atom in grid.iter() {
            let isometric_pos = isometric_transform_cpu * atom.pos.as_vec3();
            let model_transform = Mat4::from_translation(isometric_pos);
            gpu.render_mesh(
                &mesh,
                &(camera_transform * model_transform),
                Some(atom.color()),
            );
        }
    }
}
