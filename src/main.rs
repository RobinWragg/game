use std::f32::consts::SQRT_2;

use crate::GuiDefaultProperty::*;
use rand::prelude::*;
use raylib::prelude::*;
use std::ffi::{CStr, CString};

const GRID_SIZE: i32 = 64;

fn update_with_2x2_equilibrium(grid: &mut Vec<Vec<f32>>) {
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

fn checkbox(handle: &mut RaylibDrawHandle, label: &str, state: &mut bool) {
    let rect = raylib::math::Rectangle::new(10.0, 10.0, 20.0, 20.0);
    let c = CString::new(label).unwrap();
    let c2: &CStr = &c;
    handle.gui_check_box(rect, Some(c2), state);
}

fn main() {
    println!("Hello, world!");

    let mut grid = vec![vec![0.0f32; GRID_SIZE as usize]; GRID_SIZE as usize];
    for column in grid.iter_mut() {
        for y in column {
            *y = 0.0;
        }
    }
    grid[(GRID_SIZE / 2) as usize][(GRID_SIZE / 2) as usize] = 10000.0;

    let (mut rl, thread) = raylib::init()
        .log_level(TraceLogLevel::LOG_WARNING)
        .size(1200, 800)
        .title("game")
        .build();
    rl.gui_set_style(GuiControl::DEFAULT, TEXT_SIZE as i32, 20);

    // update_grid(&mut grid);
    // update_grid(&mut grid);
    const VOX_SIZE: i32 = 12;
    let mut should_play = false;
    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::BLACK);
        // d.draw_text("Hello, world!", 12, 12, 20, Color::WHITE);

        checkbox(&mut d, "sup", &mut should_play);

        for x in 0..GRID_SIZE {
            for y in 0..GRID_SIZE {
                let v = (grid[x as usize][y as usize] * 50.0).clamp(0.0, 255.0) as u8;
                let c = Color::new(v, v, v, v);
                d.draw_rectangle(x * VOX_SIZE, y * VOX_SIZE, VOX_SIZE, VOX_SIZE, c);
            }
        }

        if should_play {
            // update_with_shuffle23(&mut grid);
            update_with_2x2_equilibrium(&mut grid);
        }

        // Sleep a bit
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
