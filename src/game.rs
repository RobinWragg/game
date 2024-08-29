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
        let delta_time =
            (frame_start_time - self.prev_frame_start_time).as_micros() as f32 / 1000000.0;
        let total_time = frame_start_time
            .duration_since(self.launch_time)
            .as_micros() as f64
            / 1000000.0;

        gpu.begin_frame();
        self.debugger.render_test(gpu);
        self.debugger.render(gpu, delta_time, total_time);
        gpu.finish_frame();

        std::thread::sleep(std::time::Duration::from_millis(1)); // TODO
        self.prev_frame_start_time = frame_start_time;
    }
}
