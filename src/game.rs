use crate::grid::*;
use crate::prelude::*;

pub struct Game {
    debugger: Debugger,
    launch_time: Instant,
    prev_frame_start_time: Instant,
    grid: Grid,
    grid_editor: Editor,
    grid_viewer: Viewer,
    events_for_next_frame: VecDeque<Event>,
    previous_mouse_pos_for_deduplication: Vec2,
}

impl Game {
    pub fn new(gpu: &dyn Gpu) -> Game {
        let mut grid = Grid::from_file();
        Self {
            debugger: Debugger::default(),
            launch_time: Instant::now(),
            prev_frame_start_time: Instant::now(),
            grid,
            grid_editor: Editor::new(gpu),
            grid_viewer: Viewer::new(gpu),
            events_for_next_frame: VecDeque::new(),
            previous_mouse_pos_for_deduplication: Vec2::new(0.0, 0.0),
        }
    }

    pub fn push_event(&mut self, event: Event) {
        let event = match event {
            Event::MousePos(pos) => {
                if pos.distance(self.previous_mouse_pos_for_deduplication) > 0.0001 {
                    self.previous_mouse_pos_for_deduplication = pos;
                    Some(event)
                } else {
                    None
                }
            }
            _ => Some(event),
        };
        if let Some(event) = event {
            self.events_for_next_frame.push_back(event);
        }
    }

    pub fn update_and_render(&mut self, gpu: &mut dyn Gpu) {
        gpu.begin_frame();

        let frame_start_time = Instant::now();
        let delta_time = (frame_start_time - self.prev_frame_start_time).as_secs_f32();
        let total_time = (frame_start_time - self.launch_time).as_secs_f64();

        let mut events = std::mem::take(&mut self.events_for_next_frame);

        self.debugger.update(&mut events, delta_time, gpu);

        self.debugger.profile("Update", || {
            self.grid_editor.update(&mut self.grid, &mut events);
        });

        self.debugger.profile("Render", || {
            self.grid_editor.render_ortho(&self.grid, gpu);
            self.grid_viewer
                .render(&self.grid, Vec2::new(1.0, 0.0), gpu);
        });

        self.debugger.render(gpu);
        gpu.finish_frame();

        self.prev_frame_start_time = frame_start_time;
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        self.grid.save();
    }
}
