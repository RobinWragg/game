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

fn update_grid(grid: &mut Vec<Vec<i32>>, iteration: usize) {
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
    let mut seeds = vec![0, 1, 2, 3, 4, 5, 6, 7];
    const VOX_SIZE: i32 = 12;
    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::BLACK);
        // d.draw_text("Hello, world!", 12, 12, 20, Color::WHITE);
        for x in 0..GRID_SIZE {
            for y in 0..GRID_SIZE {
                let v = (grid[x as usize][y as usize] * 30).clamp(0, 255) as u8;
                let c = Color::new(v, v, v, v);
                d.draw_rectangle(x * VOX_SIZE, y * VOX_SIZE, VOX_SIZE, VOX_SIZE, c);
            }
        }

        seeds.shuffle(&mut thread_rng());
        for i in &seeds {
            update_grid(&mut grid, *i);
        }

        // Sleep a bit
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
