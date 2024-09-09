use crate::prelude::*;

pub const GRID_SIZE: usize = 32;

#[derive(Copy, Clone, PartialEq)]
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

fn mut_gas_pressures(grid: &mut Vec<Vec<Atom>>, x: usize, y: usize) -> Vec<&mut f32> {
    let mut pressures = vec![];

    let (column_a, column_b) = grid.split_at_mut(x + 1);
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

pub fn update_with_2x2_equilibrium(grid: &mut Vec<Vec<Atom>>) {
    debug_assert!(GRID_SIZE % 2 == 0);

    fn reach_local_equilibrium(grid: &mut Vec<Vec<Atom>>, x: usize, y: usize) {
        let pressures = mut_gas_pressures(grid, x, y);

        let mut pressure_total = 0.0;
        for pressure in &pressures {
            pressure_total += **pressure;
        }

        let total_over_4 = pressure_total / 4.0;

        for pressure in pressures {
            *pressure = total_over_4;
        }
    }

    for x in (0..GRID_SIZE).step_by(2) {
        for y in (0..GRID_SIZE).step_by(2) {
            reach_local_equilibrium(grid, x, y);
        }
    }

    for x in (1..GRID_SIZE - 1).step_by(2) {
        for y in (1..GRID_SIZE - 1).step_by(2) {
            reach_local_equilibrium(grid, x, y);
        }
    }
}
