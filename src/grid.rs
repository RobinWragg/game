use crate::math::{cone_triangles, cube_triangles, ray_unitcube_intersection};
use crate::prelude::*;
use dot_vox;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};

const SIZE: usize = 16;

pub mod grid2d;

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize, Clone, Copy)]
pub enum AtomVariant {
    Solid,
    Gas,
    GasSource,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Atom {
    pres: f32,
    vel: Vec3,
    variant: AtomVariant,
}

impl Atom {
    pub fn gas() -> Self {
        Self {
            pres: 0.0,
            vel: Vec3::ZERO,
            variant: AtomVariant::Gas,
        }
    }

    pub fn solid() -> Self {
        Self {
            pres: 0.0,
            vel: Vec3::ZERO,
            variant: AtomVariant::Solid,
        }
    }
}

fn sum_gas(a: &Atom, b: &Atom) -> Atom {
    debug_assert_eq!(a.variant, Gas);
    debug_assert_eq!(b.variant, Gas);
    let pres = a.pres + b.pres;
    let vel = a.vel * a.pres + b.vel * b.pres;
    Atom {
        pres,
        vel,
        variant: Gas,
    }
}

use AtomVariant::*;

fn adjacent_atom(origin_atom: UVec3, nearby_pos: Vec3) -> UVec3 {
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
    closest.as_usizevec3()
}

fn closest_ray_grid_intersection<'a>(
    ray_origin: Vec3,
    ray_dir: Vec3,
    atoms: impl IntoIterator<Item = UVec3>,
) -> Option<(UVec3, Vec3)> {
    let mut intersections = vec![];

    for atom in atoms {
        if let Some(intersection) = ray_unitcube_intersection(ray_origin, ray_dir, atom) {
            intersections.push((atom, intersection));
        }
    }

    if intersections.is_empty() {
        return None;
    }

    let sorter = |a: &(UVec3, Vec3), b: &(UVec3, Vec3)| {
        let a_dist = ray_origin.distance(a.1);
        let b_dist = ray_origin.distance(b.1);
        a_dist.partial_cmp(&b_dist).unwrap()
    };

    intersections.sort_by(sorter);
    Some(intersections[0])
}

fn atom_color(atom: &Atom) -> Vec4 {
    match atom.variant {
        Solid => Vec4::new(0.5, 0.5, 0.5, 1.0),
        Gas => Vec4::new(1.0, 0.0, 1.0, 1.0),
        GasSource => Vec4::splat(1.0),
    }
}

pub struct Grid {
    atoms: Vec<Vec<Vec<Atom>>>,
    step_counter: u64,
}

impl Grid {
    pub fn new() -> Self {
        Self {
            atoms: vec![],
            step_counter: 0,
        }
    }

    pub fn from_file() -> Self {
        let mut s = Self::new();
        s.load();
        s
    }

    pub fn load(&mut self) {
        fn load_inner() -> Result<Vec<Vec<Vec<Atom>>>, std::io::Error> {
            let mut file = File::open("nopush/grid_save.json")?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            Ok(serde_json::from_str(&contents)?)
        }

        self.atoms = match load_inner() {
            Ok(atoms) => {
                println!("Loading atoms from file");
                atoms
            }
            Err(_) => {
                println!("Creating new atoms");
                let mut atoms = vec![vec![vec![Atom::gas(); SIZE]; SIZE]; SIZE];
                atoms[1][1][1] = Atom::solid();
                atoms[SIZE - 2][1][1] = Atom::solid();
                atoms[1][SIZE - 2][1] = Atom::solid();
                atoms[1][1][SIZE - 2] = Atom::solid();
                atoms[SIZE - 2][SIZE - 2][SIZE - 2] = Atom::solid();
                atoms
            }
        };
    }

    pub fn save(&self) {
        let json = serde_json::to_string(&self.atoms).expect("Failed to serialize grid");

        let mut file = File::create("nopush/grid_save.json").expect("Failed to create file");
        file.write_all(json.as_bytes())
            .expect("Failed to write to file");

        println!("Grid saved to nopush/grid_save.json");
    }

    fn at(&self, pos: UVec3) -> &Atom {
        &self.atoms[pos.x][pos.y][pos.z]
    }

    fn at_mut(&mut self, pos: UVec3) -> &mut Atom {
        &mut self.atoms[pos.x][pos.y][pos.z]
    }

    fn positions(&self) -> impl Iterator<Item = UVec3> {
        let x_size = self.atoms.len();
        let y_size = self.atoms[0].len();
        let z_size = self.atoms[0][0].len();

        let f = 0..x_size;
        f.into_iter().flat_map(move |x| {
            (0..y_size)
                .into_iter()
                .flat_map(move |y| (0..z_size).into_iter().map(move |z| UVec3::new(x, y, z)))
        })
    }

    fn apply_edge_vacuum(&mut self) {
        // TODO: This isn't writing over the edge planes, only the corners.
        let s = SIZE - 1;
        for a in 0..SIZE {
            for b in 0..SIZE {
                self.atoms[a][b][0] = Atom::gas();
                self.atoms[a][0][b] = Atom::gas();
                self.atoms[0][a][b] = Atom::gas();
                self.atoms[a][b][s] = Atom::gas();
                self.atoms[a][s][b] = Atom::gas();
                self.atoms[s][a][b] = Atom::gas();
            }
        }
    }

    fn step(&mut self, spread_interval: u64) {
        self.apply_edge_vacuum();

        self.step_counter = self.step_counter.wrapping_add(1);
    }
}

pub struct Editor {
    camera_transform: Mat4, // Sans aspect ratio correct for now
    rotation: Vec2,
    mouse_pos: Option<Vec2>,
    highlighted_atom: Option<UVec3>,
    proposed_atom: Option<UVec3>,
    solid_mesh: Mesh,
    gas_mesh: Mesh,
    gas_source_mesh: Mesh,
    proposal_mesh: Mesh,
    deletion_mesh: Mesh,
    uniforms: Vec<Vec<Vec<Uniform>>>,
}

impl Editor {
    pub fn new(gpu: &dyn Gpu) -> Self {
        Self {
            camera_transform: Mat4::IDENTITY,
            rotation: Vec2::splat(PI / -8.0),
            mouse_pos: None,
            highlighted_atom: None,
            proposed_atom: None,
            solid_mesh: gpu
                .create_mesh_with_color(&cube_triangles(), &Vec4::new(0.5, 0.5, 0.5, 1.0)),
            gas_mesh: gpu.create_mesh_with_color(&cube_triangles(), &Vec4::new(1.0, 0.0, 1.0, 1.0)),
            gas_source_mesh: gpu.create_mesh_with_color(&cube_triangles(), &Vec4::splat(1.0)),
            proposal_mesh: gpu
                .create_mesh_with_color(&cube_triangles(), &Vec4::new(0.0, 1.0, 0.0, 1.0)),
            deletion_mesh: gpu
                .create_mesh_with_color(&cube_triangles(), &Vec4::new(1.0, 0.0, 0.0, 1.0)),
            uniforms: vec![],
        }
    }

    pub fn update(&mut self, grid: &mut Grid, events: &mut VecDeque<Event>) {
        let global = GLOBAL.lock().unwrap();

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

        if global.is_playing || global.should_step {
            grid.step(global.spread_interval);
        }

        // Camera rotation TODO: test whether this is framerate dependent
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
            let arbitrary_scale = 0.04;
            let scale = Mat4::from_scale(Vec3::new(
                arbitrary_scale,
                arbitrary_scale,
                depth_buffer_resolution,
            ));
            // The viable Z range is 0 to 1, so put it in the middle.
            let translate_z = Mat4::from_translation(Vec3::new(0.0, 0.0, 0.5));
            let half_size = SIZE as f32 / 2.0;
            let translate_to_center = Mat4::from_translation(Vec3::splat(-half_size));
            let rotation =
                Mat4::from_rotation_x(self.rotation.y) * Mat4::from_rotation_y(self.rotation.x);

            translate_z * scale * rotation * translate_to_center
        };

        let selection = if let Some(mouse_pos) = self.mouse_pos {
            let camera_transform_inv = self.camera_transform.inverse();
            let ray_origin =
                (camera_transform_inv * Vec4::new(mouse_pos.x, mouse_pos.y, 0.0, 1.0)).xyz();
            let ray_direction = (camera_transform_inv * Vec4::new(0.0, 0.0, 1.0, 0.0))
                .xyz()
                .normalize();

            let selectable_positions = grid.positions().filter(|pos| match grid.at(*pos).variant {
                Gas => false,
                _ => true,
            });

            if let Some((atom, intersection_location)) =
                closest_ray_grid_intersection(ray_origin, ray_direction, selectable_positions)
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
            if let Some(position) = self.proposed_atom {
                let new_atom = Atom {
                    pres: 0.0,
                    vel: Vec3::ZERO,
                    variant: global.selected_atom_variant,
                };
                *grid.at_mut(position) = new_atom;
            }
        } else if should_remove_atom {
            if let Some(highlighted_atom) = self.highlighted_atom {
                *grid.at_mut(highlighted_atom) = Atom::gas();
            }
        }
    }

    pub fn render_ortho(&mut self, grid: &Grid, gpu: &mut dyn Gpu) {
        gpu.set_render_features(RenderFeatures::DEPTH | RenderFeatures::LIGHT);
        gpu.set_camera(&self.camera_transform);

        let half_trans = Mat4::from_translation(Vec3::splat(0.5));
        let half_trans_inv = half_trans.inverse();
        let shrink = half_trans * Mat4::from_scale(Vec3::splat(0.7)) * half_trans_inv;

        if self.uniforms.len() == 0 {
            for x in 0..SIZE {
                self.uniforms.push(vec![]);
                for y in 0..SIZE {
                    self.uniforms[x].push(vec![]);
                    for z in 0..SIZE {
                        let pos = UVec3::new(x, y, z);
                        let model_transform = Mat4::from_translation(pos.as_vec3()) * shrink;
                        self.uniforms[x][y].push(gpu.create_uniform(&model_transform));
                    }
                }
            }
        }

        // cubes
        for pos in grid.positions() {
            let atom = grid.at(pos);

            if atom.variant == Gas && atom.pres < 0.05 {
                continue;
            }

            let mesh = if self.highlighted_atom == Some(pos) {
                &self.deletion_mesh
            } else {
                match atom.variant {
                    Solid => &self.solid_mesh,
                    Gas => &self.gas_mesh,
                    GasSource => &self.gas_source_mesh,
                }
            };

            let uniform = &self.uniforms[pos.x][pos.y][pos.z];
            gpu.render_mesh(mesh, &uniform);
        }

        // velocities
        let cone = cone_triangles();
        let cone_mesh = gpu.create_mesh_with_color(&cone, &Vec4::new(1.0, 1.0, 1.0, 1.0));
        for pos in grid.positions() {
            let atom = grid.at(pos);

            if atom.variant != Gas || atom.vel.length() < 0.01 {
                continue;
            }

            let uniform = &self.uniforms[pos.x][pos.y][pos.z];
            gpu.render_mesh(&cone_mesh, &uniform);
        }

        if let Some(proposed_atom) = self.proposed_atom {
            let shrink = half_trans * Mat4::from_scale(Vec3::splat(0.5)) * half_trans_inv;
            let model_transform = Mat4::from_translation(proposed_atom.as_vec3()) * shrink;

            let t = gpu.create_uniform(&model_transform);
            gpu.render_mesh(&self.proposal_mesh, &t);
            gpu.release_uniform(t);
        }
    }
}

pub struct Viewer {
    mesh: Mesh,
}

impl Viewer {
    pub fn new(gpu: &dyn Gpu) -> Self {
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

        let mesh = gpu.create_mesh(&verts, Some(&intensities), None);

        Self { mesh }
    }

    pub fn render(&self, grid: &Grid, global_translation: Vec2, gpu: &mut dyn Gpu) {
        gpu.set_render_features(RenderFeatures::DEPTH);

        let camera_transform = Mat4::from_translation(global_translation.extend(0.5))
            * Mat4::from_scale(Vec3::splat(0.005));
        gpu.set_camera(&camera_transform);

        let xhat = Vec3::new(2.0, 1.0, 1.0);
        let yhat = Vec3::new(0.0, 3.0, -1.0); // TODO: could do 0,3,0 instead and handle the depth using the mesh.
        let zhat = Vec3::new(-2.0, 1.0, 1.0);
        let isometric_transform_cpu = Mat3::from_cols(xhat, yhat, zhat);

        for pos in grid.positions() {
            let atom = grid.at(pos);

            if atom.variant == Gas && atom.pres < 0.05 {
                continue;
            }

            let isometric_pos = isometric_transform_cpu * pos.as_vec3(); // Maybe add 0.5?
            let model_transform = Mat4::from_translation(isometric_pos);

            let t = gpu.create_uniform(&model_transform);
            gpu.render_mesh(&self.mesh, &t);
            gpu.release_uniform(t);
        }
    }
}
