use crate::grid;
use crate::prelude::*;

pub struct Game {
    debugger: Debugger,
    launch_time: Instant,
    prev_frame_start_time: Instant,
    grid: Vec<Vec<f32>>,
    pub events: EventMgr,
    transform: Mat4,
}

impl Game {
    pub fn new(aspect_ratio: f32) -> Game {
        let mut grid = vec![vec![0.0f32; grid::GRID_SIZE]; grid::GRID_SIZE];
        for column in grid.iter_mut() {
            for y in column {
                *y = 0.0;
            }
        }

        let transform = Mat4::from_translation(Vec3::new(-0.9, -0.9, 0.0))
            * Mat4::from_scale(Vec3::new(0.05 / aspect_ratio, 0.05, 1.0));

        Self {
            debugger: Debugger::default(),
            launch_time: Instant::now(),
            prev_frame_start_time: Instant::now(),
            grid,
            events: EventMgr::default(),
            transform,
        }
    }

    fn consume_event(&mut self, event: Event) -> bool {
        match event {
            Event::LeftClickPressed(pos) => {
                let v = transform_2d(&pos, &self.transform.inverse());
                let x = v.x as usize;
                let y = v.y as usize;
                self.grid[x][y] = 5000.0;
                true
            }
            _ => false,
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

        for x in 0..grid::GRID_SIZE {
            for y in 0..grid::GRID_SIZE {
                let v = self.grid[x][y] * 0.01;
                let m = Mat4::from_translation(Vec3::new(x as f32, y as f32, 0.0));
                gpu.render_mesh(
                    &mesh,
                    &(self.transform * m),
                    Some(Vec4::new(v, 0.0, 1.0 - v, 1.0)),
                );
            }
        }
    }

    pub fn update_and_render(&mut self, gpu: &mut Gpu) {
        gpu.begin_frame();

        let frame_start_time = Instant::now();
        let delta_time = (frame_start_time - self.prev_frame_start_time).as_secs_f32();
        let total_time = (frame_start_time - self.launch_time).as_secs_f64();

        self.events.begin_frame();
        while let Some(event) = self.events.pop() {
            if self.debugger.consume_event(&event) {
                continue;
            } else if self.consume_event(event) {
                continue;
            }
        }

        grid::update_with_2x2_equilibrium(&mut self.grid);
        self.render_grid(gpu);

        // std::thread::sleep(std::time::Duration::from_millis(500)); // TODO
        self.debugger.render(&self.events, gpu, delta_time);
        gpu.finish_frame();
        self.prev_frame_start_time = frame_start_time;
    }
}
