use crate::grid::{Atom, EditorState};
use crate::math::transform_2d;
use crate::prelude::*;
use egui::epaint::{image::ImageData, textures::*};
use egui::{self, Modifiers};

// TODO: I'm not clipping the primitives as instructed.

#[derive(Default)]
pub struct Debugger {
    ctx: egui::Context,
    egui_to_gpu_tex_id: HashMap<u64, usize>,
    mesh: Option<Mesh>,
    delta_times: VecDeque<f32>,
    input: egui::RawInput,
    matrix: Mat4,
    full_output: egui::FullOutput,
    pub editor_state: EditorState,
}

impl Debugger {
    fn max_dt(delta_times: &VecDeque<f32>) -> f32 {
        *delta_times
            .iter()
            .max_by(|a, b| {
                if a < b {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            })
            .unwrap()
    }

    pub fn render_test(&mut self, gpu: &mut Gpu) {
        let mesh = match self.mesh.as_mut() {
            Some(m) => m,
            None => {
                let positions = vec![
                    Vec2::new(0.0, 0.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(0.0, 1.0),
                ];

                let colors = vec![
                    Vec4::new(0.0, 1.0, 0.0, 1.0),
                    Vec4::new(1.0, 0.0, 0.0, 1.0),
                    Vec4::new(0.0, 0.0, 1.0, 0.0),
                ];
                let mesh = Mesh::new_2d(&positions, Some(&colors), Some((0, &positions)), gpu);
                self.mesh = Some(mesh);
                self.mesh.as_mut().unwrap()
            }
        };

        // TODO: don't use the egui texture for the render test. use an independent one. It's also pretty hacky to have a pub texture field.
        match self.egui_to_gpu_tex_id.get(&0) {
            Some(t) => {
                mesh.texture = *t;
            }
            None => return,
        };

        gpu.render_mesh(&mesh, &Mat4::IDENTITY, None);
    }

    pub fn update(&mut self, events: &mut VecDeque<Event>, dt: f32, gpu: &Gpu) {
        events.retain(|event| {
            match event {
                Event::LeftClickPressed(pos) => {
                    let mouse_egui = transform_2d(pos, &self.matrix.inverse());
                    let mouse_egui = egui::Pos2::new(mouse_egui.x, mouse_egui.y);
                    self.input.events.push(egui::Event::PointerButton {
                        pos: mouse_egui,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::default(),
                    });
                }
                Event::LeftClickReleased(pos) => {
                    let mouse_egui = transform_2d(pos, &self.matrix.inverse());
                    let mouse_egui = egui::Pos2::new(mouse_egui.x, mouse_egui.y);
                    self.input.events.push(egui::Event::PointerButton {
                        pos: mouse_egui,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::default(),
                    });
                }
                Event::MousePos(pos) => {
                    let mouse_egui = transform_2d(pos, &self.matrix.inverse());
                    let mouse_egui = egui::Pos2::new(mouse_egui.x, mouse_egui.y);
                    self.input
                        .events
                        .push(egui::Event::PointerMoved(mouse_egui));
                }
                _ => (),
            }

            // Remove pointer events (return false) if the egui context wants them.
            !self.ctx.wants_pointer_input()
        });

        self.ctx.set_pixels_per_point(2.0); // TODO: customise this based on window height?

        self.matrix = {
            gpu.window_to_normalized_transform()
                * Mat4::from_scale(Vec3::new(
                    self.ctx.pixels_per_point(),
                    self.ctx.pixels_per_point(),
                    1.0,
                ))
        };

        self.full_output = self.ctx.run(std::mem::take(&mut self.input), |ctx| {
            egui::TopBottomPanel::top("top panel").show(&ctx, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    // TODO: Update the displayed time every second instead of every 60 frames.
                    // TODO: also, switch to processing time.
                    self.delta_times.push_back(dt);
                    if self.delta_times.len() > 60 {
                        self.delta_times.pop_front();
                    }

                    let max_dt = Self::max_dt(&self.delta_times);
                    ui.label(format!("Worst frame: {:.1}ms", max_dt * 1000.0));
                });
            });
            egui::Window::new("Editor").show(&ctx, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    let radio_atom = self.editor_state.current_atom;
                    ui.radio_value(
                        &mut self.editor_state.current_atom,
                        if let Atom::Gas(p) = radio_atom {
                            Atom::Gas(p)
                        } else {
                            Atom::Gas(0.0)
                        },
                        "Gas",
                    );
                    ui.radio_value(&mut self.editor_state.current_atom, Atom::Solid, "Solid");
                    ui.radio_value(&mut self.editor_state.current_atom, Atom::Liquid, "Liquid");
                });
                if let Atom::Gas(pressure) = &mut self.editor_state.current_atom {
                    ui.add(egui::Slider::new(pressure, -100.0..=100.0).text("Pressure"));
                }

                self.editor_state.should_reload = ui.button("Reload").clicked();

                if ui
                    .button(if self.editor_state.is_playing {
                        "Pause"
                    } else {
                        "Play"
                    })
                    .clicked()
                {
                    self.editor_state.is_playing = !self.editor_state.is_playing;
                }

                self.editor_state.should_step = ui.button("Step").clicked();
            });
        });
    }

    pub fn render(&mut self, gpu: &mut Gpu) {
        gpu.depth_test(false);

        if !self.full_output.textures_delta.set.is_empty() {
            assert_eq!(self.full_output.textures_delta.set.len(), 1);
            let (egui_tex_id, delta) = &self.full_output.textures_delta.set[0];
            assert_eq!(delta.options.magnification, TextureFilter::Linear);
            assert_eq!(delta.options.minification, TextureFilter::Linear);
            assert_eq!(delta.options.wrap_mode, TextureWrapMode::ClampToEdge);
            assert_eq!(delta.pos, None);
            let font_image = match &delta.image {
                ImageData::Color(_) => panic!(),
                ImageData::Font(f) => f,
            };

            let gpu_tex_id = gpu.create_texture(font_image.size[0], font_image.size[1], true);
            let srgba_pixels = font_image.srgba_pixels(None);
            let mut pixel_bytes = Vec::with_capacity(srgba_pixels.len() * 4);
            for pixel in srgba_pixels {
                pixel_bytes.push(pixel.r());
                pixel_bytes.push(pixel.g());
                pixel_bytes.push(pixel.b());
                pixel_bytes.push(pixel.a());
            }
            gpu.write_rgba_texture(gpu_tex_id, &pixel_bytes);

            let egui_tex_id = match egui_tex_id {
                egui::TextureId::Managed(id) => *id,
                _ => panic!(),
            };
            assert!(egui_tex_id == 0);

            self.egui_to_gpu_tex_id.insert(egui_tex_id, gpu_tex_id);
        }
        assert!(self.full_output.textures_delta.free.is_empty());

        let shapes = std::mem::take(&mut self.full_output.shapes);
        for prim in self
            .ctx
            .tessellate(shapes, self.full_output.pixels_per_point)
        {
            let mesh = match prim.primitive {
                egui::epaint::Primitive::Mesh(m) => m,
                _ => panic!(),
            };

            let mut vert_positions = Vec::with_capacity(mesh.indices.len());
            let mut vert_colors = Vec::with_capacity(mesh.indices.len() * 4);
            let mut vert_uvs = Vec::with_capacity(mesh.indices.len());
            for index in mesh.indices {
                let vert = mesh.vertices[index as usize];
                vert_positions.push(Vec2::new(vert.pos.x, vert.pos.y));
                let rgba = vert.color.to_array(); // TODO: this is premultiplied
                vert_colors.extend_from_slice(&rgba);
                vert_uvs.push(Vec2::new(vert.uv.x, vert.uv.y));
            }

            let vert_colors = {
                let mut colors_vec4s = Vec::with_capacity(vert_colors.len() / 4);
                for i in (0..vert_colors.len()).step_by(4) {
                    let v = Vec4::new(
                        vert_colors[i] as f32 / 255.0,
                        vert_colors[i + 1] as f32 / 255.0,
                        vert_colors[i + 2] as f32 / 255.0,
                        vert_colors[i + 3] as f32 / 255.0,
                    );
                    colors_vec4s.push(v);
                }
                colors_vec4s
            };

            let egui_tex_id = match mesh.texture_id {
                egui::TextureId::Managed(id) => id,
                _ => panic!(),
            };

            let gpu_tex_id = *self.egui_to_gpu_tex_id.get(&egui_tex_id).unwrap();
            assert!(gpu_tex_id != 0);

            let mesh = Mesh::new_2d(
                &vert_positions,
                Some(&vert_colors),
                Some((gpu_tex_id, &vert_uvs)),
                gpu,
            );
            gpu.render_mesh(&mesh, &self.matrix, None);
        }
    }
}
