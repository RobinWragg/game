use raylib::prelude::*;

fn print_grid(grid: &Vec<Vec<i32>>) {
    for row in grid.iter() {
        for cell in row.iter() {
            print!("{:x}\t", cell);
        }
        println!();
    }
}

fn update_grid(grid: &mut Vec<Vec<i32>>) {
    fn cut_in_half(src_pressure: &mut i32) -> i32 {
        let half = (*src_pressure as f32) / 2.0;
        let new_src_pressure = f32::floor(half) as i32;
        let outgoing_pressure = f32::ceil(half) as i32;
        assert!(new_src_pressure + outgoing_pressure == *src_pressure);
        outgoing_pressure
    }

    TODO: create a new array that will contain the changes

    for x in 1..grid.len() - 1 {
        for y in 1..grid[x].len() - 1 {
            let mut this = grid[x][y];
            let mut adj_indices = vec![];

            adj_indices.push((x + 1, y));
            adj_indices.push((x - 1, y));
            adj_indices.push((x, y + 1));
            adj_indices.push((x, y - 1));
            // adj_indices.push((x + 1, y - 1));
            // adj_indices.push((x - 1, y + 1));
            // adj_indices.push((x - 1, y - 1));
            // adj_indices.push((x + 1, y + 1));

            let mut low_pressure_adj_indices = vec![];
            for (x, y) in adj_indices {
                let adj = grid[x][y];
                if this > adj {
                    low_pressure_adj_indices.push((x, y));
                }
            }

            if low_pressure_adj_indices.len() != 0 {
                this -= low_pressure_adj_indices.len() as i32;
                if this < 0 {
                    this = 0;
                }
                for (x, y) in low_pressure_adj_indices {
                    grid[x][y] += 1;
                    if grid[x][y] > 255 {
                        grid[x][y] = 255;
                    }
                }

                grid[x][y] = this;
                // should_break = true;
                // break;
            }
            // if should_break {
            //     break;
            // }
        }
    }
}

fn main() {
    println!("Hello, world!");

    const GRID_SIZE: i32 = 16;

    let mut grid = vec![vec![0; GRID_SIZE as usize]; GRID_SIZE as usize];
    for column in grid.iter_mut() {
        for y in column {
            *y = 0;
        }
    }
    grid[4][4] = 10;

    // print_grid(&grid);

    let (mut rl, thread) = raylib::init().size(800, 600).title("game").build();

    const VOX_SIZE: i32 = 30;
    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::BLACK);
        // d.draw_text("Hello, world!", 12, 12, 20, Color::WHITE);
        for x in 0..GRID_SIZE {
            for y in 0..GRID_SIZE {
                let v = (grid[x as usize][y as usize] as u32 * 80) as u8;
                let c = Color::new(v, v, v, v);
                d.draw_rectangle(x * VOX_SIZE, y * VOX_SIZE, VOX_SIZE, VOX_SIZE, c);
            }
        }

        update_grid(&mut grid);

        // Sleep a bit
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
