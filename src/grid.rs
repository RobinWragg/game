use std::f32::consts::SQRT_2;

use rand::prelude::*;
use std::ffi::{CStr, CString};

pub const GRID_SIZE: i32 = 64; // TODO 64

// This also works with i32, but the /4 causes small losses. Not a problem in a vaccuum where we expect losses, but doesn't model an airtight space well.
pub fn update_with_2x2_equilibrium(grid: &mut Vec<Vec<f32>>) {
    debug_assert!(GRID_SIZE % 2 == 0);

    fn reach_local_equilibrium(grid: &mut Vec<Vec<f32>>, x: usize, y: usize) {
        let total = grid[x][y] + grid[x + 1][y] + grid[x][y + 1] + grid[x + 1][y + 1];
        let total_over_4 = total / 4.0;
        grid[x][y] = total_over_4;
        grid[x + 1][y] = total_over_4;
        grid[x][y + 1] = total_over_4;
        grid[x + 1][y + 1] = total_over_4;
    }

    for x in (0..GRID_SIZE).step_by(2) {
        for y in (0..GRID_SIZE).step_by(2) {
            reach_local_equilibrium(grid, x as usize, y as usize);
        }
    }

    for x in (1..GRID_SIZE - 1).step_by(2) {
        for y in (1..GRID_SIZE - 1).step_by(2) {
            reach_local_equilibrium(grid, x as usize, y as usize);
        }
    }
}

fn update_with_shuffle23(grid: &mut Vec<Vec<f32>>) {
    fn iterate(grid: &mut Vec<Vec<f32>>, iteration: usize) {
        let mut changes = vec![vec![0.0; GRID_SIZE as usize]; GRID_SIZE as usize];

        let directions = vec![
            (1i32, 0i32),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
        ];

        let inv_sqrt2 = 1.0 / SQRT_2;
        let weights = vec![
            1.0, inv_sqrt2, 1.0, inv_sqrt2, 1.0, inv_sqrt2, 1.0, inv_sqrt2,
        ];

        let direction = directions[iteration];
        let weight = weights[iteration];

        for x in 1..grid.len() - 1 {
            for y in 1..grid[x].len() - 1 {
                if grid[x][y] < weight {
                    // TODO: set to 0?
                    continue;
                }

                let x32 = x as i32;
                let y32 = y as i32;
                if grid[(x32 + direction.0) as usize][(y32 + direction.1) as usize] < grid[x][y] {
                    // println!(
                    //     "transferring from {} {} to {} {}",
                    //     x,
                    //     y,
                    //     x32 + direction.0,
                    //     y32 + direction.1,
                    // );
                    changes[x][y] -= weight;
                    changes[(x32 + direction.0) as usize][(y32 + direction.1) as usize] += weight;
                }
            }
        }

        for x in 1..grid.len() - 1 {
            for y in 1..grid[x].len() - 1 {
                grid[x][y] += changes[x][y];
                grid[x][y] = grid[x][y].clamp(0.0, 100000.0);
            }
        }
    }

    let mut seeds = vec![0, 1, 2, 3, 4, 5, 6, 7];
    seeds.shuffle(&mut thread_rng());
    for i in &seeds {
        iterate(grid, *i);
    }
}
