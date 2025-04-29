use crate::math::{adjacent_cube, closest_ray_grid_intersection, cube_triangles, transform_2d};
use crate::prelude::*;
use dot_vox;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};

pub const GRID_SIZE: usize = 4;

// TODO: use a hashmap instead?
#[derive(Default, Copy, Clone)]
pub struct EditorState {
    pub current_atom: Atom,
    pub should_reload: bool,
    pub is_playing: bool,
    pub should_step: bool,
}

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Atom {
    Gas(f32),
    Solid,
    Liquid,
}

impl Default for Atom {
    fn default() -> Self {
        Atom::Gas(0.0)
    }
}

pub struct Grid2d {
    atoms: Vec<Vec<Atom>>,
    transform: Mat4,
    mover: f32,
}

impl Grid2d {
    fn new() -> Self {
        let scale = 0.1;
        let translate_z = 0.5; // The viable range is 0 to 1, so put it in the middle.
        Self {
            transform: Mat4::from_translation(Vec3::new(0.0, 0.0, translate_z))
                * Mat4::from_scale(Vec3::new(scale, scale, scale)),
            atoms: vec![vec![Atom::default(); GRID_SIZE]; GRID_SIZE],
            mover: 0.0,
        }
    }

    pub fn load() -> Self {
        fn load_inner() -> Result<Vec<Vec<Atom>>, std::io::Error> {
            let mut file = File::open("nopush/grid_save.json")?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            Ok(serde_json::from_str(&contents)?)
        }

        let mut grid = Self::new();

        grid.atoms = match load_inner() {
            Ok(atoms) => {
                println!("Loading atoms from file");
                atoms
            }
            Err(_) => {
                println!("Creating new atoms");
                vec![vec![Atom::default(); GRID_SIZE]; GRID_SIZE]
            }
        };

        grid
    }

    pub fn modify_under_path(&mut self, start: &Vec2, end: &Vec2, editor: &EditorState) {
        // TODO: I'm not sure when the best time to transform from Vec2 to (usize, usize) is. I think this fn shouldn't be aware of the editor either. The pub interface to the grid can convert Vec2 to (usize, usize) and inspect the editor before getting here.
        let start = transform_2d(&start, &self.transform.inverse());
        let end = transform_2d(end, &self.transform.inverse());

        let start = (
            start.x.clamp(0.0, GRID_SIZE as f32 - 1.0) as usize,
            start.y.clamp(0.0, GRID_SIZE as f32 - 1.0) as usize,
        );
        let end = (
            end.x.clamp(0.0, GRID_SIZE as f32 - 1.0) as usize,
            end.y.clamp(0.0, GRID_SIZE as f32 - 1.0) as usize,
        );

        for (x, y) in Grid2d::atoms_on_path(start, end) {
            self.atoms[x][y] = editor.current_atom;
        }
    }

    pub fn save(&self) {
        let json = serde_json::to_string(&self.atoms).expect("Failed to serialize grid");

        let mut file = File::create("nopush/grid_save.json").expect("Failed to create file");
        file.write_all(json.as_bytes())
            .expect("Failed to write to file");

        println!("Grid saved to nopush/grid_save.json");
    }

    fn atoms_on_path(start: (usize, usize), end: (usize, usize)) -> Vec<(usize, usize)> {
        let mut path: Vec<(i32, i32)> = vec![];

        let mut mover = (start.0 as i32, start.1 as i32);
        let end = (end.0 as i32, end.1 as i32);

        path.push(mover);

        loop {
            if mover == end {
                break;
            }

            if (mover.0 - end.0).abs() > (mover.1 - end.1).abs() {
                if mover.0 < end.0 {
                    mover.0 += 1;
                } else {
                    mover.0 -= 1;
                }
            } else {
                if mover.1 < end.1 {
                    mover.1 += 1;
                } else {
                    mover.1 -= 1;
                }
            }

            path.push(mover);
        }

        path.into_iter()
            .map(|(x, y)| (x as usize, y as usize))
            .collect::<Vec<(usize, usize)>>()
    }

    fn mut_gas_pressures(&mut self, x: usize, y: usize) -> Vec<&mut f32> {
        let mut pressures = vec![];

        let (column_a, column_b) = self.atoms.split_at_mut(x + 1);
        let (cell_a, cell_b) = column_a[x].split_at_mut(y + 1);
        let (cell_c, cell_d) = column_b[0].split_at_mut(y + 1);

        if let Atom::Gas(pressure) = &mut cell_a[y] {
            pressures.push(pressure);
        }
        if let Atom::Gas(pressure) = &mut cell_b[0] {
            pressures.push(pressure);
        }
        if let Atom::Gas(pressure) = &mut cell_c[y] {
            pressures.push(pressure);
        }
        if let Atom::Gas(pressure) = &mut cell_d[0] {
            pressures.push(pressure);
        }

        pressures
    }

    pub fn update(&mut self, editor: &EditorState) {
        if editor.should_reload {
            self.atoms = Self::load().atoms;
        }

        if editor.is_playing || editor.should_step {
            self.update_gas_with_2x2_equilibrium();
        }

        self.mover += 0.01;
    }

    fn update_gas_with_2x2_equilibrium(&mut self) {
        debug_assert!(GRID_SIZE % 2 == 0);

        let mut reach_local_equilibrium = |x: usize, y: usize| {
            let pressures = self.mut_gas_pressures(x, y);

            let mut pressure_total = 0.0;
            for pressure in &pressures {
                pressure_total += **pressure;
            }

            let divided_total = pressure_total / pressures.len() as f32;

            for pressure in pressures {
                *pressure = divided_total;
            }
        };

        for x in (0..GRID_SIZE).step_by(2) {
            for y in (0..GRID_SIZE).step_by(2) {
                reach_local_equilibrium(x, y);
            }
        }

        for x in (1..GRID_SIZE - 1).step_by(2) {
            for y in (1..GRID_SIZE - 1).step_by(2) {
                reach_local_equilibrium(x, y);
            }
        }

        // Erase edges
        for x in 0..GRID_SIZE {
            self.atoms[x][0] = Atom::Gas(0.0);
            self.atoms[x][GRID_SIZE - 1] = Atom::Gas(0.0);
        }
        for y in 0..GRID_SIZE {
            self.atoms[0][y] = Atom::Gas(0.0);
            self.atoms[GRID_SIZE - 1][y] = Atom::Gas(0.0);
        }
    }

    pub fn render_2d(&self, gpu: &mut Gpu) {
        gpu.set_render_features(Gpu::FEATURE_DEPTH);

        let verts = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(0.9, 0.0),
            Vec2::new(0.0, 0.9),
            Vec2::new(0.0, 0.9),
            Vec2::new(0.9, 0.0),
            Vec2::new(0.9, 0.9),
        ];

        let mesh = Mesh::new_2d(&verts, None, None, gpu);

        for x in 0..GRID_SIZE {
            for y in 0..GRID_SIZE {
                let color = match self.atoms[x][y] {
                    Atom::Gas(v) => Vec4::new(v * 0.01, 0.0, 1.0 - v * 0.01, 1.0),
                    Atom::Solid => Vec4::new(0.0, 1.0, 0.0, 1.0),
                    Atom::Liquid => Vec4::new(0.0, 1.0, 1.0, 1.0),
                };
                let m = Mat4::from_translation(Vec3::new(x as f32, y as f32, 0.0));
                gpu.render_mesh(&mesh, &(self.transform * m), Some(color));
            }
        }
    }
}

#[derive(Clone)]
pub struct Cube {
    pub pos: IVec3,
    pub color: Vec4,
}

pub struct Grid {
    cubes: Vec<Cube>,
}

impl Grid {
    pub fn new() -> Self {
        Self { cubes: vec![] }
    }

    pub fn is_empty(&self) -> bool {
        self.cubes.is_empty()
    }

    pub fn add(&mut self, pos: IVec3) {
        debug_assert!(!self.contains(&pos), "Cube already exists at this position");
        self.cubes.push(Cube {
            pos,
            color: Vec4::new(1.0, rand::random::<f32>(), rand::random::<f32>(), 1.0),
        });
    }

    pub fn overwrite(&mut self, pos: IVec3) {
        debug_assert!(self.contains(&pos), "Cube doesn't exist at this position");
        self.cubes
            .iter_mut()
            .find(|cube| cube.pos == pos)
            .unwrap()
            .pos = pos;
    }

    pub fn contains(&self, pos: &IVec3) -> bool {
        self.cubes.iter().any(|cube| cube.pos == *pos)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Cube> {
        self.cubes.iter()
    }

    pub fn positions(&self) -> impl Iterator<Item = &IVec3> {
        self.cubes.iter().map(|cube| &cube.pos)
    }

    pub fn remove(&mut self, pos: IVec3) {
        self.cubes.retain(|cube| cube.pos != pos);
    }
}

pub struct Editor {
    global_transform: Mat4,
    rotation: Vec2,
    mouse_pos: Option<Vec2>,
    highlighted_cube: Option<IVec3>,
    proposed_cube: Option<IVec3>,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            global_transform: Mat4::IDENTITY,
            rotation: Vec2::ZERO,
            mouse_pos: None,
            highlighted_cube: None,
            proposed_cube: None,
        }
    }

    pub fn update(&mut self, grid: &mut Grid, events: &mut VecDeque<Event>) {
        let mut should_add_cube = false;
        let mut should_remove_cube = false;
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
                should_add_cube = true;
                false
            }
            Event::RightClickPressed(_) => {
                should_remove_cube = true;
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

        self.global_transform = {
            let depth_buffer_resolution = 0.01;
            let arbitrary_scale = Mat4::from_scale(Vec3::new(0.01, 0.01, depth_buffer_resolution));
            // The viable Z range is 0 to 1, so put it in the middle.
            let translate_z = Mat4::from_translation(Vec3::new(0.0, 0.0, 0.5));
            let rotation =
                Mat4::from_rotation_x(self.rotation.y) * Mat4::from_rotation_y(self.rotation.x);
            translate_z * arbitrary_scale * rotation
        };

        let selection = if let Some(mouse_pos) = self.mouse_pos {
            let global_transform_inv = self.global_transform.inverse();
            let ray_origin =
                (global_transform_inv * Vec4::new(mouse_pos.x, mouse_pos.y, 0.0, 1.0)).xyz();
            let ray_direction = (global_transform_inv * Vec4::new(0.0, 0.0, 1.0, 0.0))
                .xyz()
                .normalize();

            if let Some((cube, intersection_location)) =
                closest_ray_grid_intersection(ray_origin, ray_direction, grid.positions())
            {
                Some((cube, intersection_location))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((highlighted_cube, intersection_location)) = selection {
            self.highlighted_cube = Some(highlighted_cube);
            self.proposed_cube = Some(adjacent_cube(highlighted_cube, intersection_location));
            if self.highlighted_cube == self.proposed_cube {
                self.proposed_cube = None;
            }
        } else {
            self.highlighted_cube = None;
            self.proposed_cube = None;
        }

        if should_add_cube {
            if let Some(proposed_cube) = self.proposed_cube {
                if !grid.contains(&proposed_cube) {
                    grid.add(proposed_cube);
                }
            }
        } else if should_remove_cube {
            if let Some(highlighted_cube) = self.highlighted_cube {
                grid.remove(highlighted_cube);
            }
        }
    }

    pub fn render_ortho(&self, grid: &Grid, gpu: &mut Gpu) {
        gpu.set_render_features(Gpu::FEATURE_DEPTH | Gpu::FEATURE_LIGHT);

        let mesh = Mesh::new(&cube_triangles(), None, None, gpu);

        let half_trans = Mat4::from_translation(Vec3::splat(0.5));
        let shrink = half_trans * Mat4::from_scale(Vec3::splat(1.0)) * half_trans.inverse();

        for cube in grid.iter() {
            let local_translation = Mat4::from_translation(cube.pos.as_vec3());
            let cube_transform = self.global_transform * local_translation * shrink;

            let color = if self.highlighted_cube == Some(cube.pos) {
                Some(Vec4::new(0.0, 1.0, 0.0, 1.0))
            } else {
                Some(cube.color)
            };

            gpu.render_mesh(&mesh, &cube_transform, color);
        }

        if let Some(proposed_cube) = self.proposed_cube {
            let local_translation = Mat4::from_translation(proposed_cube.as_vec3());
            let cube_transform = self.global_transform * local_translation * shrink;
            gpu.render_mesh(&mesh, &cube_transform, Some(Vec4::new(0.0, 1.0, 1.0, 1.0)));
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
        let rectangle_verts = [
            Vec2::new(0.0, 0.0),
            Vec2::new(4.0, 0.0),
            Vec2::new(4.0, 1.0),
            Vec2::new(4.0, 1.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(0.0, 0.0),
        ];

        let mesh = Mesh::new_2d(&rectangle_verts, None, None, gpu);
        let mat = Mat4::from_translation(global_translation.extend(0.5))
            * Mat4::from_scale(Vec3::splat(0.01));

        let xhat = Vec3::new(2.0, 1.0, 1.0);
        let yhat = Vec3::new(0.0, 3.0, -1.0); // TODO: could do 0,3,0 instead and handle the depth using the mesh.
        let zhat = Vec3::new(-2.0, 1.0, 1.0);
        let r = Mat3::from_cols(xhat, yhat, zhat);

        for cube in grid.iter() {
            let p = r * cube.pos.as_vec3();
            let cube_mat = Mat4::from_translation(p);

            let front_color = (cube.color.xyz() * 0.7).extend(1.0);
            gpu.render_mesh(&mesh, &(mat * cube_mat), Some(cube.color));
            gpu.render_mesh(
                &mesh,
                &(mat * cube_mat * Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0))),
                Some(front_color),
            );
            gpu.render_mesh(
                &mesh,
                &(mat * cube_mat * Mat4::from_translation(Vec3::new(0.0, -2.0, 0.0))),
                Some(front_color),
            );
            gpu.render_mesh(
                &mesh,
                &(mat * cube_mat * Mat4::from_translation(Vec3::new(0.0, -3.0, 0.0))),
                Some(front_color),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_path() {
        let path = Grid2d::atoms_on_path((2, 2), (2, 2));
        assert_eq!(path, vec![(2, 2)]);
    }
}
