use crate::grid::*;
use crate::prelude::*;
use serde_json;
use std::fs::File;
use std::io::{Read, Write};

pub struct Game {
    debugger: Debugger,
    launch_time: Instant,
    prev_frame_start_time: Instant,
    grid: Vec<Vec<Atom>>,
    transform: Mat4,
    events_for_next_frame: VecDeque<Event>,
    dragging_pos: Option<Vec2>,
    previous_mouse_pos_for_deduplication: Vec2,
}

impl Game {
    pub fn new(aspect_ratio: f32) -> Game {
        let grid = Self::load_grid();

        let transform = Mat4::from_translation(Vec3::new(-0.9, -0.9, 0.0))
            * Mat4::from_scale(Vec3::new(0.05 / aspect_ratio, 0.05, 1.0));

        Self {
            debugger: Debugger::default(),
            launch_time: Instant::now(),
            prev_frame_start_time: Instant::now(),
            grid,
            transform,
            events_for_next_frame: VecDeque::new(),
            dragging_pos: None,
            previous_mouse_pos_for_deduplication: Vec2::new(0.0, 0.0),
        }
    }

    fn load_grid() -> Vec<Vec<Atom>> {
        let result = File::open("nopush/grid_save.json")
            .and_then(|mut file| {
                let mut contents = String::new();
                file.read_to_string(&mut contents)?;
                Ok(contents)
            })
            .and_then(|contents| {
                serde_json::from_str(&contents)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            });

        match result {
            Ok(grid) => {
                println!("Grid loaded from nopush/grid_save.json");
                grid
            }
            Err(_) => {
                println!("Creating new grid");
                vec![vec![Atom::Gas(0.0); GRID_SIZE]; GRID_SIZE]
            }
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
            self.grid = Self::load_grid();
        }

        let mut modify_grid_under_path = |start: &Vec2, end: &Vec2| {
            let start = transform_2d(start, &self.transform.inverse());
            let end = transform_2d(end, &self.transform.inverse());

            let start = (
                start.x.clamp(0.0, GRID_SIZE as f32 - 1.0) as usize,
                start.y.clamp(0.0, GRID_SIZE as f32 - 1.0) as usize,
            );
            let end = (
                end.x.clamp(0.0, GRID_SIZE as f32 - 1.0) as usize,
                end.y.clamp(0.0, GRID_SIZE as f32 - 1.0) as usize,
            );

            for (x, y) in atoms_on_path(start, end) {
                self.grid[x][y] = editor.current_atom;
            }
        };

        events.retain(|event| match event {
            Event::MousePos(end) => {
                if let Some(start) = self.dragging_pos {
                    // TODO: This can currently be called multiple times per atom when dragging, so my dragging_pos should be a Option<(usize, usize)> instead.
                    modify_grid_under_path(&start, end);
                    self.dragging_pos = Some(*end);
                }
                false
            }
            Event::LeftClickPressed(pos) => {
                modify_grid_under_path(pos, pos);
                self.dragging_pos = Some(*pos);
                false
            }
            Event::LeftClickReleased(_) => {
                self.dragging_pos = None;
                false
            }
            _ => true,
        });

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
                let color = match self.grid[x][y] {
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

        update_with_2x2_equilibrium(&mut self.grid);
        self.update_and_render_grid(&mut events, self.debugger.editor_state, gpu);

        // std::thread::sleep(std::time::Duration::from_millis(500)); // TODO
        self.debugger.render(gpu);
        gpu.finish_frame();
        self.prev_frame_start_time = frame_start_time;
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        // Serialize the grid to JSON
        let json = serde_json::to_string(&self.grid).expect("Failed to serialize grid");

        // Save the JSON to a file
        let mut file = File::create("nopush/grid_save.json").expect("Failed to create file");
        file.write_all(json.as_bytes())
            .expect("Failed to write to file");

        println!("Grid saved to nopush/grid_save.json");
    }
}
