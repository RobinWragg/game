use crate::common_types::*;
use crate::debugger::Debugger;
use crate::gpu::Gpu;
use std::time::Instant;

pub struct Game {
    debugger: Debugger,
    launch_time: Instant,
    prev_frame_start_time: Instant,
}

impl Game {
    pub fn new() -> Game {
        Self {
            debugger: Debugger::default(),
            launch_time: Instant::now(),
            prev_frame_start_time: Instant::now(),
        }
    }

    pub fn update_and_render(&mut self, gpu: &mut Gpu) {
        let frame_start_time = Instant::now();
        let dt = (frame_start_time - self.prev_frame_start_time).as_micros() as f32 / 1000000.0;
        let tt = frame_start_time
            .duration_since(self.launch_time)
            .as_micros() as f64
            / 1000000.0;

        gpu.begin_frame();
        gpu.render_triangles(&[], None, None, Mat4::IDENTITY); // TODO
        self.debugger.render_test(gpu);
        self.debugger.render(gpu, dt, tt);
        gpu.finish_frame();

        self.prev_frame_start_time = Instant::now();

        // std::thread::sleep(std::time::Duration::from_millis(14)); // TODO
    }
}
