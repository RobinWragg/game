use crate::prelude::*;
use serde::{Deserialize, Serialize};

pub const GRID_SIZE: usize = 32;

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
    pub atoms: Vec<Vec<Atom>>,
}

impl Grid {
    pub fn new() -> Self {
        Self {
            atoms: vec![vec![Atom::default(); GRID_SIZE]; GRID_SIZE],
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
        self.update_with_2x2_equilibrium();
    }

    fn update_with_2x2_equilibrium(&mut self) {
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
