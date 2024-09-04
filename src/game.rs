use crate::grid;
use crate::prelude::*;

pub struct Game {
    debugger: Debugger,
    launch_time: Instant,
    prev_frame_start_time: Instant,
    grid: Vec<Vec<f32>>,
    pub user: User,
}

impl Game {
    pub fn new() -> Game {
        let mut grid = vec![vec![0.0f32; grid::GRID_SIZE as usize]; grid::GRID_SIZE as usize];
        for column in grid.iter_mut() {
            for y in column {
                *y = 0.0;
            }
        }
        grid[(grid::GRID_SIZE / 2) as usize][(grid::GRID_SIZE / 2) as usize] = 10000.0;

        Self {
            debugger: Debugger::default(),
            launch_time: Instant::now(),
            prev_frame_start_time: Instant::now(),
            grid,
            user: User::default(),
        }
    }

    fn render_grid(&self, gpu: &mut Gpu) {
        let verts = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(0.9, 0.0),
            Vec2::new(0.0, 0.9),
            Vec2::new(0.0, 0.9),
            Vec2::new(0.9, 0.0),
            Vec2::new(0.9, 0.9),
        ];

        let mesh = Mesh::new(&verts, None, None, gpu);

        let scale = Mat4::from_scale(Vec3::new(0.1 / gpu.aspect_ratio(), 0.1, 1.0));
        for x in 0..grid::GRID_SIZE {
            for y in 0..grid::GRID_SIZE {
                let v = (self.grid[x as usize][y as usize] * 50.0).clamp(0.0, 255.0) as u8;
                let m = Mat4::from_translation(Vec3::new(x as f32, y as f32, 0.0));
                gpu.render_mesh(&mesh, &(scale * m), Some(Vec4::new(0.0, 1.0, 0.0, 1.0)));
            }
        }
    }

    pub fn update_and_render(&mut self, gpu: &mut Gpu) {
        gpu.begin_frame();

        let frame_start_time = Instant::now();
        let delta_time = (frame_start_time - self.prev_frame_start_time).as_secs_f32();
        let total_time = (frame_start_time - self.launch_time).as_secs_f64();

        // grid::update_with_2x2_equilibrium(&mut self.grid);

        self.render_grid(gpu);
        self.debugger.render_test(gpu);

        // std::thread::sleep(std::time::Duration::from_millis(100)); // TODO
        self.debugger.render(&self.user, gpu, delta_time);
        gpu.finish_frame();
        self.prev_frame_start_time = frame_start_time;
    }
}
