use rand::prelude::*;
use raylib::prelude::*;

const GRID_SIZE: i32 = 64;

fn print_grid(grid: &Vec<Vec<i32>>) {
    for row in grid.iter() {
        for cell in row.iter() {
            print!("{:x}\t", cell);
        }
        println!();
    }
}

fn update_with_2x2_equilibrium(grid: &mut Vec<Vec<i32>>) {
    debug_assert!(GRID_SIZE % 2 == 0);

    fn reach_local_equilibrium(grid: &mut Vec<Vec<i32>>, x: usize, y: usize) {
        let total = grid[x][y] + grid[x + 1][y] + grid[x][y + 1] + grid[x + 1][y + 1];
        let total_over_4 = total / 4; // TODO: Lossy!
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

fn update_with_shuffle23(grid: &mut Vec<Vec<i32>>) {
    fn iterate(grid: &mut Vec<Vec<i32>>, iteration: usize) {
        let mut changes = vec![vec![0; GRID_SIZE as usize]; GRID_SIZE as usize];

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

        let weights = vec![3, 2, 3, 2, 3, 2, 3, 2];

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
                grid[x][y] = grid[x][y].clamp(0, 100000);
            }
        }
    }

    let mut seeds = vec![0, 1, 2, 3, 4, 5, 6, 7];
    seeds.shuffle(&mut thread_rng());
    for i in &seeds {
        iterate(grid, *i);
    }
}

fn main() {
    println!("Hello, world!");

    let mut grid = vec![vec![0; GRID_SIZE as usize]; GRID_SIZE as usize];
    for column in grid.iter_mut() {
        for y in column {
            *y = 0;
        }
    }
    grid[(GRID_SIZE / 2) as usize][(GRID_SIZE / 2) as usize] = 10000;

    // print_grid(&grid);

    let (mut rl, thread) = raylib::init().size(1200, 800).title("game").build();

    // update_grid(&mut grid);
    // update_grid(&mut grid);
    const VOX_SIZE: i32 = 12;
    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::BLACK);
        // d.draw_text("Hello, world!", 12, 12, 20, Color::WHITE);
        for x in 0..GRID_SIZE {
            for y in 0..GRID_SIZE {
                let v = (grid[x as usize][y as usize] * 10).clamp(0, 255) as u8;
                let c = Color::new(v, v, v, v);
                d.draw_rectangle(x * VOX_SIZE, y * VOX_SIZE, VOX_SIZE, VOX_SIZE, c);
            }
        }

        // update_with_shuffle23(&mut grid);
        update_with_2x2_equilibrium(&mut grid);

        // Sleep a bit
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
