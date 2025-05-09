use crate::grid::Atom;
use crate::math::transform_2d;
use crate::prelude::*;
use egui;
use egui::epaint::{image::ImageData, textures::*};

// TODO: I'm not clipping the primitives as instructed.

#[derive(Default)]
pub struct Debugger {
    ctx: egui::Context,
    egui_to_gpu_tex_id: HashMap<u64, usize>,
    delta_times: HashMap<String, VecDeque<f32>>,
    input: egui::RawInput,
    matrix: Mat4,
    full_output: egui::FullOutput,
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

    pub fn profile<F: FnOnce() -> ()>(&mut self, name: &str, f: F) {
        let start = Instant::now();
        f();
        let dt = start.elapsed().as_secs_f32();

        if !self.delta_times.contains_key(name) {
            self.delta_times.insert(name.to_string(), VecDeque::new());
        }

        self.delta_times.get_mut(name).unwrap().push_back(dt);
    }

    pub fn update(&mut self, events: &mut VecDeque<Event>, dt: f32, gpu: &dyn Gpu) {
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
                    // Remove pointer events (return false) if the egui context wants them.
                    !self.ctx.wants_pointer_input()
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
                    // Remove pointer events (return false) if the egui context wants them.
                    !self.ctx.wants_pointer_input()
                }
                Event::MousePos(pos) => {
                    let mouse_egui = transform_2d(pos, &self.matrix.inverse());
                    let mouse_egui = egui::Pos2::new(mouse_egui.x, mouse_egui.y);
                    self.input
                        .events
                        .push(egui::Event::PointerMoved(mouse_egui));
                    // Remove pointer events (return false) if the egui context wants them.
                    !self.ctx.wants_pointer_input()
                }
                _ => true,
            }
        });

        if !self.delta_times.contains_key("total") {
            self.delta_times
                .insert("total".to_string(), VecDeque::new());
        }

        self.delta_times.get_mut("total").unwrap().push_back(dt);

        for v in self.delta_times.values_mut() {
            while v.len() > 60 {
                v.pop_front();
            }
        }

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
            let mut global = GLOBAL.lock().unwrap();

            egui::TopBottomPanel::top("top panel").show(&ctx, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    for key in self.delta_times.keys() {
                        let max_dt = Self::max_dt(&self.delta_times[key]);
                        ui.label(format!("{}: {:.1}ms", key, max_dt * 1000.0));
                    }
                });
            });
            egui::Window::new("Editor").show(&ctx, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    ui.radio_value(
                        &mut global.selected_atom_type,
                        Atom::Solid(Vec4::ZERO),
                        "Solid",
                    );
                    ui.radio_value(
                        &mut global.selected_atom_type,
                        Atom::Gas((0.0, Vec3::ZERO)),
                        "Gas",
                    );
                    ui.radio_value(
                        &mut global.selected_atom_type,
                        Atom::GasSource(Vec3::ZERO),
                        "GasSource",
                    );
                });

                ui.add(
                    egui::Slider::new(&mut global.spread_interval, 1..=16).text("Spread Interval"),
                );

                if ui
                    .button(if global.is_playing { "Pause" } else { "Play" })
                    .clicked()
                {
                    global.is_playing = !global.is_playing;
                }

                global.should_step = ui.button("Step").clicked();
            });
        });
    }

    pub fn render(&mut self, gpu: &mut dyn Gpu) {
        gpu.set_render_features(RenderFeatures::empty());

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
                vert_positions.push(Vec3::new(vert.pos.x, vert.pos.y, 0.0));
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

            let mesh = gpu.create_mesh(
                &vert_positions,
                Some(&vert_colors),
                Some((gpu_tex_id, &vert_uvs)),
            );
            gpu.render_mesh(&mesh, &self.matrix);
        }
    }
}
