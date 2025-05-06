use crate::math::{cube_triangles, ray_unitcube_intersection};
use crate::prelude::*;
use dot_vox;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};

const SIZE: usize = 32;
const SPREAD_FREQUENCY: u64 = 2;

pub mod grid2d;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Atom {
    Solid(Vec4), // Color. TODO: f32 is gross overkill here.
    Gas((f32, Vec3)),
    GasSource(Vec3),
}

fn sum_gas(a: &(f32, Vec3), b: &(f32, Vec3)) -> Atom {
    let total_p = a.0 + b.0;
    let total_v = a.1 * a.0 + b.1 * b.0;
    Gas((total_p, total_v))
}

use Atom::*;

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
                UVec3::new(voxel.x.into(), voxel.z.into(), voxel.y.into()),
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

    atoms = {
        // axes
        for i in 1..32 {
            atoms.push(AtomWithPos::with_color(
                UVec3::new(i, 0, 0),
                Vec4::new(1.0, 0.0, 0.0, 1.0),
            ));
            atoms.push(AtomWithPos::with_color(
                UVec3::new(0, i, 0),
                Vec4::new(0.0, 1.0, 0.0, 1.0),
            ));
            atoms.push(AtomWithPos::with_color(
                UVec3::new(0, 0, i),
                Vec4::new(0.0, 0.0, 1.0, 1.0),
            ));
        }
        atoms
    };

    atoms
}

fn hollow_out(atoms: &mut Vec<AtomWithPos>) {
    let atoms_2 = atoms.clone();
    atoms.retain(|a| {
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
}

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
    match atom {
        Solid(color) => *color,
        Gas(_) => Vec4::new(1.0, 0.0, 1.0, 1.0),
        GasSource(_) => Vec4::splat(1.0),
    }
}

#[derive(Clone)]
struct AtomWithPos {
    pub pos: UVec3, // TODO: i16 or even i8 might be ok here.
    pub variant: Atom,
}

impl AtomWithPos {
    fn with_color(pos: UVec3, color: Vec4) -> Self {
        Self {
            pos,
            variant: Solid(color),
        }
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
                let mut atoms = vec![vec![vec![Gas((0.0, Vec3::ZERO)); SIZE]; SIZE]; SIZE];
                atoms[1][1][1] = Solid(Vec4::new(0.5, 0.5, 0.5, 1.0));
                atoms[SIZE - 2][1][1] = Solid(Vec4::new(1.0, 0.0, 0.0, 1.0));
                atoms[1][SIZE - 2][1] = Solid(Vec4::new(0.0, 1.0, 0.0, 1.0));
                atoms[1][1][SIZE - 2] = Solid(Vec4::new(0.0, 0.0, 1.0, 1.0));
                atoms[SIZE - 2][SIZE - 2][SIZE - 2] = Solid(Vec4::splat(1.0));
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

    fn mut_gases_2x2x2(&mut self, pos: UVec3) -> Vec<&mut (f32, Vec3)> {
        let mut gases = Vec::with_capacity(8); // Preallocate for 2x2x2 section

        let atoms_ptr = self.atoms.as_mut_ptr(); // Get a raw pointer to the outer grid

        unsafe {
            for dx in 0..2 {
                for dy in 0..2 {
                    for dz in 0..2 {
                        // Calculate the raw pointer to the current voxel
                        let slice_ptr = atoms_ptr.add(pos.x + dx);
                        let column_ptr = (*slice_ptr).as_mut_ptr().add(pos.y + dy);
                        let atom_ptr = (*column_ptr).as_mut_ptr().add(pos.z + dz);

                        // Dereference the pointer and check if it's a Gas atom
                        if let Atom::Gas(gas) = &mut *atom_ptr {
                            gases.push(gas);
                        }
                    }
                }
            }
        }

        gases
    }

    fn spread_gas(&mut self) {
        let step_counter = self.step_counter;

        let mut reach_local_equilibrium = |pos: UVec3| {
            let gases = self.mut_gases_2x2x2(pos);
            debug_assert_ne!(gases.len(), 0);

            let mut pressure_total = 0.0;
            let mut velocity_total = Vec3::ZERO;
            for gas in &gases {
                pressure_total += gas.0;
                velocity_total += gas.1;
            }

            let divided_total_pressure = pressure_total / gases.len() as f32;
            let divided_total_velocity = velocity_total / gases.len() as f32;

            for gas in gases {
                gas.0 = divided_total_pressure;
                gas.1 = divided_total_velocity;
            }
        };

        if step_counter % (SPREAD_FREQUENCY * 2) == 0 {
            for x in (0..SIZE).step_by(2) {
                for y in (0..SIZE).step_by(2) {
                    for z in (0..SIZE).step_by(2) {
                        reach_local_equilibrium(UVec3::new(x, y, z));
                    }
                }
            }
        } else if step_counter % (SPREAD_FREQUENCY * 2) == SPREAD_FREQUENCY {
            for x in (1..SIZE - 1).step_by(2) {
                for y in (1..SIZE - 1).step_by(2) {
                    for z in (1..SIZE - 1).step_by(2) {
                        reach_local_equilibrium(UVec3::new(x, y, z));
                    }
                }
            }
        }
    }

    fn apply_edge_vacuum(&mut self) {
        // TODO: This isn't writing over the edge planes, only the corners.
        let s = SIZE - 1;
        for a in 0..SIZE {
            for b in 0..SIZE {
                self.atoms[a][b][0] = Gas((0.0, Vec3::ZERO));
                self.atoms[a][0][b] = Gas((0.0, Vec3::ZERO));
                self.atoms[0][a][b] = Gas((0.0, Vec3::ZERO));
                self.atoms[a][b][s] = Gas((0.0, Vec3::ZERO));
                self.atoms[a][s][b] = Gas((0.0, Vec3::ZERO));
                self.atoms[s][a][b] = Gas((0.0, Vec3::ZERO));
            }
        }
    }

    fn step_gas_source(&mut self) {
        const SOURCE_PRESSURE: f32 = 1.0;

        let sources: Vec<(UVec3, Vec3)> = self
            .positions()
            .filter_map(|pos| {
                if let GasSource(v) = self.at(pos) {
                    Some((pos, *v))
                } else {
                    None
                }
            })
            .collect();

        for source in sources {
            let pos = source.0;
            let source_v = source.1;
            let adjacent = (pos.as_vec3() + Vec3::splat(0.5) + source_v.normalize()).as_usizevec3();
            debug_assert_eq!(pos.manhattan_distance(adjacent), 1);

            if let Gas(old) = self.atoms[adjacent.x][adjacent.y][adjacent.z] {
                let new_atom = sum_gas(&old, &(SOURCE_PRESSURE, source_v));
                self.atoms[adjacent.x][adjacent.y][adjacent.z] = new_atom;
            }
        }
    }

    fn simulate_gas_velocity(&mut self) {
        let mut gases_to_zero = vec![];
        let mut gases_to_add = vec![];

        for src_pos in self.positions() {
            if let Gas(src) = self.at(src_pos) {
                if src.1.length_squared() < 0.001 {
                    continue;
                }

                let dst_pos =
                    (src_pos.as_vec3() + Vec3::splat(0.5) + src.1.normalize()).as_usizevec3();
                debug_assert!(src_pos.chebyshev_distance(dst_pos) <= 1);
                debug_assert_ne!(src_pos, dst_pos);

                if dst_pos.x < SIZE && dst_pos.y < SIZE && dst_pos.z < SIZE {
                    if let Gas(_) = self.atoms[dst_pos.x][dst_pos.y][dst_pos.z] {
                        gases_to_add.push((dst_pos, *src));
                    }
                }

                // TODO: If a gas hits a solid obliquely, it should maintain part of its velocity.
                gases_to_zero.push(src_pos);
            }
        }

        for to_zero in gases_to_zero {
            self.atoms[to_zero.x][to_zero.y][to_zero.z] = Gas((0.0, Vec3::ZERO));
        }

        for (dst_pos, src) in gases_to_add {
            if let Gas(dst) = &self.atoms[dst_pos.x][dst_pos.y][dst_pos.z] {
                self.atoms[dst_pos.x][dst_pos.y][dst_pos.z] = sum_gas(dst, &src);
            }
        }
    }

    fn print_p(&self, label: &str) {
        let mut total_p = 0.0;
        for p in self.positions() {
            if let Gas((p, _)) = self.at(p) {
                total_p += p;
            }
        }
        println!("{} {}", label, total_p);
    }

    fn step(&mut self) {
        self.simulate_gas_velocity();
        self.step_gas_source();
        self.spread_gas();
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
}

impl Editor {
    pub fn new() -> Self {
        Self {
            camera_transform: Mat4::IDENTITY,
            rotation: Vec2::splat(PI / -8.0),
            mouse_pos: None,
            highlighted_atom: None,
            proposed_atom: None,
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
            grid.step();
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

            let selectable_positions = grid.positions().filter(|pos| match grid.at(*pos) {
                Gas((p, _)) => *p > 0.1,
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
                let new_atom = match global.selected_atom_type {
                    Solid(_) => Solid(Vec4::new(0.5, 0.5, 0.5, 1.0)),
                    Gas(_) => Gas((1.0, Vec3::ZERO)),
                    GasSource(_) => GasSource(Vec3::new(100.0, 0.0, 0.0)),
                };
                *grid.at_mut(position) = new_atom;
            }
        } else if should_remove_atom {
            if let Some(highlighted_atom) = self.highlighted_atom {
                *grid.at_mut(highlighted_atom) = Gas((0.0, Vec3::ZERO));
            }
        }
    }

    pub fn render_ortho(&self, grid: &Grid, gpu: &mut Gpu) {
        gpu.set_render_features(RenderFeatures::DEPTH | RenderFeatures::LIGHT);

        let mesh = Mesh::new(&cube_triangles(), None, None, gpu);

        let half_trans = Mat4::from_translation(Vec3::splat(0.5));
        let half_trans_inv = half_trans.inverse();

        for pos in grid.positions() {
            let atom = grid.at(pos);

            if let Gas((p, _)) = *atom {
                if p < 0.05 {
                    continue;
                }
            }

            let atom_size = if let Gas((pressure, _)) = *atom {
                pressure
            } else {
                0.8
            };

            let shrink = half_trans * Mat4::from_scale(Vec3::splat(atom_size)) * half_trans_inv;
            let model_transform = Mat4::from_translation(pos.as_vec3()) * shrink;
            let total_transform = self.camera_transform * model_transform;

            let color = if self.highlighted_atom == Some(pos) {
                Some(Vec4::new(1.0, 0.5, 0.5, 1.0))
            } else {
                Some(atom_color(atom))
            };

            gpu.render_mesh(&mesh, &total_transform, color);
        }

        if let Some(proposed_atom) = self.proposed_atom {
            let shrink = half_trans * Mat4::from_scale(Vec3::splat(0.5)) * half_trans_inv;
            let model_transform = Mat4::from_translation(proposed_atom.as_vec3()) * shrink;
            let total_transform = self.camera_transform * model_transform;
            gpu.render_mesh(&mesh, &total_transform, Some(Vec4::new(0.5, 1.0, 0.5, 1.0)));
        }
    }
}

pub struct Viewer {}

impl Viewer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(&self, grid: &Grid, global_translation: Vec2, gpu: &mut Gpu) {
        gpu.set_render_features(RenderFeatures::DEPTH);

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

        for pos in grid.positions() {
            let atom = grid.at(pos);

            if let Gas(p) = *atom {
                continue;
            }

            let isometric_pos = isometric_transform_cpu * pos.as_vec3(); // Maybe add 0.5?
            let model_transform = Mat4::from_translation(isometric_pos);
            gpu.render_mesh(
                &mesh,
                &(camera_transform * model_transform),
                Some(atom_color(atom)),
            );
        }
    }
}
