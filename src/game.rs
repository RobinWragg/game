use crate::grid::*;
use crate::prelude::*;
use serde_json;

pub struct Game {
    debugger: Debugger,
    launch_time: Instant,
    prev_frame_start_time: Instant,
    grid: Grid,
    grid_viewer: Viewer,
    events_for_next_frame: VecDeque<Event>,
    dragging_pos: Option<Vec2>,
    previous_mouse_pos_for_deduplication: Vec2,
}

impl Game {
    pub fn new() -> Game {
        Self {
            debugger: Debugger::default(),
            launch_time: Instant::now(),
            prev_frame_start_time: Instant::now(),
            grid: Grid::load(),
            grid_viewer: Viewer::new(),
            events_for_next_frame: VecDeque::new(),
            dragging_pos: None,
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

    fn update_and_render_grid(
        &mut self,
        events: &mut VecDeque<Event>,
        editor: EditorState,
        gpu: &mut Gpu,
    ) {
        events.retain(|event| match event {
            Event::MousePos(end) => {
                if let Some(start) = self.dragging_pos {
                    // TODO: This can currently be called multiple times per atom when dragging, so my dragging_pos should be a Option<(usize, usize)> instead.
                    self.grid.modify_under_path(&start, &end, &editor);
                    self.dragging_pos = Some(*end);
                }
                false
            }
            Event::LeftClickPressed(pos) => {
                self.grid.modify_under_path(&pos, &pos, &editor);
                self.dragging_pos = Some(*pos);
                false
            }
            Event::LeftClickReleased(_) => {
                self.dragging_pos = None;
                false
            }
            _ => true,
        });

        self.grid.update(&editor);
        self.grid_viewer.update(&editor);
        self.grid.render_2d(gpu);
        self.grid_viewer.render_ortho(gpu);
    }

    pub fn update_and_render(&mut self, gpu: &mut Gpu) {
        gpu.begin_frame();

        let frame_start_time = Instant::now();
        let delta_time = (frame_start_time - self.prev_frame_start_time).as_secs_f32();
        let total_time = (frame_start_time - self.launch_time).as_secs_f64();

        let mut events = std::mem::take(&mut self.events_for_next_frame);

        self.debugger.update(&mut events, delta_time, gpu);

        self.update_and_render_grid(&mut events, self.debugger.editor_state, gpu);

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
