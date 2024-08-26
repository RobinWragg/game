use crate::common_types::*;
use crate::gpu::Gpu;
use egui;
use egui::epaint::{image::ImageData, textures::*};
use std::collections::HashMap;

// TODO: I'm not clipping the primitives as instructed.

#[derive(Default)]
pub struct Debugger {
    ctx: egui::Context,
    egui_to_gpu_tex_id: HashMap<u64, usize>,
}

impl Debugger {
    pub fn render_test(&mut self, gpu: &mut Gpu) {
        let texture = *match self.egui_to_gpu_tex_id.get(&0) {
            Some(t) => t,
            None => return,
        };

        let mut matrix = Mat4::IDENTITY;
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
        gpu.render_triangles(&positions, None, Some((texture, &positions)), matrix);
        matrix.w_axis.x += 0.2;
        gpu.render_triangles(&positions, Some(&colors), None, matrix);
        matrix.w_axis.x += 0.2;
        gpu.render_triangles(&positions, None, None, matrix);
        matrix.w_axis.x += 0.2;
        gpu.render_triangles(
            &positions,
            Some(&colors),
            Some((texture, &positions)),
            matrix,
        );
    }

    pub fn render(&mut self, gpu: &mut Gpu, dt: f64) {
        let raw_input = egui::RawInput::default();
        self.ctx.set_pixels_per_point(2.0); // TODO: customise this based on window height?
        let full_output = self.ctx.run(raw_input, |ctx| {
            egui::TopBottomPanel::top("top panel").show(&ctx, |ui| {
                ui.label(format!("{:.1}ms", dt * 1000.0));
            });
            egui::Window::new("window!").show(&ctx, |ui| {
                ui.label("Hello world!");
                let mut wat = false;
                ui.checkbox(&mut wat, "checkbox");
                let _ = ui.button("button");
                let mut slider_value = 30.0;
                ui.add(egui::Slider::new(&mut slider_value, 0.0..=100.0).text("My value"));
            });
        });

        if !full_output.textures_delta.set.is_empty() {
            assert_eq!(full_output.textures_delta.set.len(), 1);
            let (egui_tex_id, delta) = &full_output.textures_delta.set[0];
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
        assert!(full_output.textures_delta.free.is_empty());

        dbg!(full_output.shapes.len());
        for prim in self
            .ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point)
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

            let scale_x = (full_output.pixels_per_point * 2.0) / gpu.width() as f32;
            let scale_y = (full_output.pixels_per_point * 2.0) / gpu.height() as f32;
            let trans_matrix = Mat4::from_translation(Vec3::new(-1.0, 1.0, 0.0));
            let scale_matrix = Mat4::from_scale(Vec3::new(scale_x, -scale_y, 1.0));
            gpu.render_triangles(
                &vert_positions,
                Some(&vert_colors),
                Some((gpu_tex_id, &vert_uvs)),
                trans_matrix * scale_matrix,
            );
        }
    }
}
