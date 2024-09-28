use crate::grid::*;
use crate::prelude::*;
use serde_json;

pub struct Game {
    debugger: Debugger,
    launch_time: Instant,
    prev_frame_start_time: Instant,
    grid: Grid,
    transform: Mat4,
    events_for_next_frame: VecDeque<Event>,
    dragging_pos: Option<Vec2>,
    previous_mouse_pos_for_deduplication: Vec2,
}

impl Game {
    pub fn new(aspect_ratio: f32) -> Game {
        let scale = 0.1;
        let transform = Mat4::from_translation(Vec3::new(-0.9, -0.9, 0.0))
            * Mat4::from_scale(Vec3::new(scale / aspect_ratio, scale, 1.0));

        Self {
            debugger: Debugger::default(),
            launch_time: Instant::now(),
            prev_frame_start_time: Instant::now(),
            grid: Grid::load(),
            transform,
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
        if editor.should_reload {
            self.grid = Grid::load();
        }

        events.retain(|event| match event {
            Event::MousePos(end) => {
                if let Some(start) = self.dragging_pos {
                    // TODO: This can currently be called multiple times per atom when dragging, so my dragging_pos should be a Option<(usize, usize)> instead.
                    {
                        let start = transform_2d(&start, &self.transform.inverse());
                        let end = transform_2d(end, &self.transform.inverse());
                        self.grid.modify_under_path(&start, &end, &editor);
                    }
                    self.dragging_pos = Some(*end);
                }
                false
            }
            Event::LeftClickPressed(pos) => {
                {
                    let pos = transform_2d(&pos, &self.transform.inverse());
                    self.grid.modify_under_path(&pos, &pos, &editor);
                }
                self.dragging_pos = Some(*pos);
                false
            }
            Event::LeftClickReleased(_) => {
                self.dragging_pos = None;
                false
            }
            _ => true,
        });

        if editor.is_playing || editor.should_step {
            self.grid.update();
        }

        let verts = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(0.9, 0.0),
            Vec2::new(0.0, 0.9),
            Vec2::new(0.0, 0.9),
            Vec2::new(0.9, 0.0),
            Vec2::new(0.9, 0.9),
        ];

        let mesh = Mesh::new(&verts, None, None, gpu);

        for x in 0..GRID_SIZE {
            for y in 0..GRID_SIZE {
                let color = match self.grid.atoms[x][y] {
                    Atom::Gas(v) => Vec4::new(v * 0.01, 0.0, 1.0 - v * 0.01, 1.0),
                    Atom::Solid => Vec4::new(0.0, 1.0, 0.0, 1.0),
                    Atom::Liquid => Vec4::new(0.0, 1.0, 1.0, 1.0),
                };
                let m = Mat4::from_translation(Vec3::new(x as f32, y as f32, 0.0));
                gpu.render_mesh(&mesh, &(self.transform * m), Some(color));
            }
        }
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
