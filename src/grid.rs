use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};

pub const GRID_SIZE: usize = 8;

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

pub struct Grid {
    atoms: Vec<Vec<Atom>>,
    transform: Mat4,
}

impl Grid {
    pub fn new() -> Self {
        Self {
            transform: Mat4::IDENTITY,
            atoms: vec![vec![Atom::default(); GRID_SIZE]; GRID_SIZE],
        }
    }

    pub fn set_view_aspect_ratio(&mut self, aspect_ratio: f32) {
        let scale = 0.1;
        self.transform = Mat4::from_translation(Vec3::new(-0.9, -0.9, 0.0))
            * Mat4::from_scale(Vec3::new(scale / aspect_ratio, scale, 1.0));
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

        for (x, y) in Grid::atoms_on_path(start, end) {
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

    pub fn load() -> Self {
        fn load_inner() -> Result<Vec<Vec<Atom>>, std::io::Error> {
            let mut file = File::open("nopush/grid_save.json")?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            Ok(serde_json::from_str(&contents)?)
        }

        match load_inner() {
            Ok(atoms) => {
                println!("Loading grid from file");
                let mut grid = Self::new();
                grid.atoms = atoms;
                grid
            }
            Err(_) => {
                println!("Creating new grid");
                Self::new()
            }
        }
    }

    pub fn atoms_on_path(start: (usize, usize), end: (usize, usize)) -> Vec<(usize, usize)> {
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

    pub fn update(&mut self) {
        self.update_gas_with_2x2_equilibrium();
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

    pub fn render(&mut self, gpu: &mut Gpu) {
        let verts = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(0.9, 0.0),
            Vec2::new(0.0, 0.9),
            Vec2::new(0.0, 0.9),
            Vec2::new(0.9, 0.0),
            Vec2::new(0.9, 0.9),
        ];

        let mesh = Mesh::new(&verts, None, None, gpu);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_path() {
        let path = Grid::atoms_on_path((2, 2), (2, 2));
        assert_eq!(path, vec![(2, 2)]);
    }
}
