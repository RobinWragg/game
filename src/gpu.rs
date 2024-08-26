use crate::common_types::*;
use bytemuck;
use pollster;
use std::sync::Arc;
use wgpu;
use winit::window::Window;

// TODO: I wonder if I can resize the buffers on the fly.
const VERTEX_BUFFERS_SIZE: u64 = 30000;
const WHITE_TEXTURE_ID: usize = 0;

struct Texture {
    texture: wgpu::Texture,
    size: wgpu::Extent3d,
    bindgroup: wgpu::BindGroup,
}

pub struct Gpu<'a> {
    surface: wgpu::Surface<'a>,
    surface_texture: Option<wgpu::SurfaceTexture>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    uniforms_bindgroup: wgpu::BindGroup,
    texture_bindgroup_layout: wgpu::BindGroupLayout,
    textures: Vec<Texture>,
    matrix_buffer: wgpu::Buffer,
    vertpos_buffer: wgpu::Buffer,
    vertcolor_buffer: wgpu::Buffer,
    uv_buffer: wgpu::Buffer,
    width: usize,
    height: usize,
}

impl<'a> Gpu<'a> {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn new(window: &Arc<Window>) -> Gpu<'a> {
        let (surface, adapter) = {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                ..Default::default()
            });
            let surface = instance.create_surface(window.clone()).unwrap();
            let adapter =
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                }))
                .unwrap();
            (surface, adapter)
        };

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .unwrap();

        let size = window.inner_size(); // Size in physical pixels
        let mut surface_config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();
        surface_config.present_mode = wgpu::PresentMode::Fifo;
        surface.configure(&device, &surface_config);

        let matrix_buffer = {
            let desc = wgpu::BufferDescriptor {
                label: None,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                size: std::mem::size_of::<Mat4>() as u64,
                mapped_at_creation: false,
            };
            device.create_buffer(&desc)
        };

        let vertpos_buffer = {
            let desc = wgpu::BufferDescriptor {
                label: None,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                size: VERTEX_BUFFERS_SIZE,
                mapped_at_creation: false,
            };
            device.create_buffer(&desc)
        };
        let vertcolor_buffer = {
            let desc = wgpu::BufferDescriptor {
                label: None,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                size: VERTEX_BUFFERS_SIZE,
                mapped_at_creation: false,
            };
            device.create_buffer(&desc)
        };
        let uv_buffer = {
            let desc = wgpu::BufferDescriptor {
                label: None,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                size: VERTEX_BUFFERS_SIZE,
                mapped_at_creation: false,
            };
            device.create_buffer(&desc)
        };

        let uniforms_bindgroup_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: None,
            });
        let uniforms_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniforms_bindgroup_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: matrix_buffer.as_entire_binding(),
            }],
            label: None,
        });

        let texture_bindgroup_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: None,
            });

        let pipeline = Self::create_pipeline(
            &device,
            &surface_config,
            &[&uniforms_bindgroup_layout, &texture_bindgroup_layout],
        );

        let mut gpu = Self {
            width: window.inner_size().width as usize,
            height: window.inner_size().height as usize,
            surface,
            surface_texture: None,
            device,
            queue,
            pipeline,
            uniforms_bindgroup,
            texture_bindgroup_layout,
            matrix_buffer,
            vertpos_buffer,
            vertcolor_buffer,
            uv_buffer,
            textures: vec![],
        };

        // The white texture is used when the user doesn't want texturing; the vertex
        // colors get multiplied with white (255u8), allowing the texturing pipeline to
        // handle non-textured meshes.
        let white_texture = gpu.create_texture(1, 1, false);
        gpu.write_monochrome_texture(white_texture, &[255u8; 1]);
        assert_eq!(white_texture, WHITE_TEXTURE_ID);

        gpu
    }

    fn create_pipeline(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/default.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts,
            push_constant_ranges: &[],
        });
        let vertpos_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            }],
        };
        let vertcolor_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x4,
            }],
        };
        let uv_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x2,
            }],
        };
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[vertpos_layout, vertcolor_layout, uv_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING), // TODO: not premultiplied
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        })
    }

    pub fn create_texture(&mut self, width: usize, height: usize, linear_filtering: bool) -> usize {
        let size = wgpu::Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("default gb texture"),
            view_formats: &[],
        });
        let filter = if linear_filtering {
            wgpu::FilterMode::Linear
        } else {
            wgpu::FilterMode::Nearest
        };
        let bindgroup = {
            let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
                // rwtodo: what are the defaults?
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.texture_bindgroup_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
                label: Some("default gb texture bind group"),
            })
        };

        self.textures.push(Texture {
            texture,
            size,
            bindgroup,
        });
        self.textures.len() - 1
    }

    pub fn begin_frame(&mut self) {
        self.surface_texture = Some(self.surface.get_current_texture().unwrap());
        // rwtodo: clear.
    }

    pub fn finish_frame(&mut self) {
        let surface_texture = std::mem::replace(&mut self.surface_texture, None);
        surface_texture.unwrap().present();
    }

    pub fn write_monochrome_texture(&self, texture_id: usize, pixels: &[u8]) {
        let texture = &self.textures[texture_id];
        assert_eq!(
            pixels.len(),
            (texture.size.width * texture.size.height) as usize,
            "expected 8bit single-channel pixel data"
        );

        let mut rgba_pixel_bytes = Vec::with_capacity(pixels.len() * 4);
        for pixel in pixels {
            rgba_pixel_bytes.push(*pixel);
            rgba_pixel_bytes.push(*pixel);
            rgba_pixel_bytes.push(*pixel);
            rgba_pixel_bytes.push(0xff);
        }

        self.write_rgba_texture(texture_id, &rgba_pixel_bytes);
    }

    pub fn write_rgba_texture(&self, texture_id: usize, pixel_bytes: &[u8]) {
        let texture = &self.textures[texture_id];
        assert_eq!(
            pixel_bytes.len(),
            (texture.size.width * texture.size.height * 4) as usize,
            "expected 8bit RGBA pixel data"
        );
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixel_bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(texture.size.width * 4),
                rows_per_image: Some(texture.size.height),
            },
            texture.size,
        );
    }

    fn write_vec2_slice_to_buffer(&self, buffer: &wgpu::Buffer, slice: &[Vec2]) {
        let mut floats: Vec<f32> = Vec::with_capacity(slice.len() * 2); // Assume Vec2 or bigger.
        for i in 0..slice.len() {
            let a = slice[i].to_array();
            floats.extend_from_slice(&a);
        }
        let bytes = bytemuck::cast_slice(&floats);
        self.queue.write_buffer(buffer, 0, bytes);
    }

    fn write_vec4_slice_to_buffer(&self, buffer: &wgpu::Buffer, slice: &[Vec4]) {
        let mut floats: Vec<f32> = Vec::with_capacity(slice.len() * 2); // Assume Vec2 or bigger.
        for i in 0..slice.len() {
            let a = slice[i].to_array();
            floats.extend_from_slice(&a);
        }
        let bytes = bytemuck::cast_slice(&floats);
        self.queue.write_buffer(buffer, 0, bytes);
    }

    pub fn render_triangles(
        &self,
        verts: &[Vec2],
        colors: Option<&[Vec4]>,
        texture_id_and_uvs: Option<(usize, &[Vec2])>,
        matrix: Mat4,
    ) {
        self.write_vec2_slice_to_buffer(&self.vertpos_buffer, verts);

        if let Some(colors) = colors {
            self.write_vec4_slice_to_buffer(&self.vertcolor_buffer, colors);
        } else {
            // Disable vertex colors by just multiplying the texture with white.
            let white = Vec4::new(1.0, 1.0, 1.0, 1.0);
            self.write_vec4_slice_to_buffer(&self.vertcolor_buffer, &vec![white; verts.len()]);
        }

        let texture_id = match texture_id_and_uvs {
            Some((id, uvs)) => {
                self.write_vec2_slice_to_buffer(&self.uv_buffer, uvs);
                id
            }
            // Disable texturing by just multiplying vertex colors with white.
            None => WHITE_TEXTURE_ID,
        };

        // Write the matrix to its wgpu buffer
        {
            let matrix_floats = matrix.to_cols_array();
            let matrix_bytes = bytemuck::bytes_of(&matrix_floats);
            self.queue
                .write_buffer(&self.matrix_buffer, 0, matrix_bytes);
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let view = self
                .surface_texture
                .as_ref()
                .expect("Did you forget to call gpu.begin_frame()?")
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertpos_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.vertcolor_buffer.slice(..));
            render_pass.set_vertex_buffer(2, self.uv_buffer.slice(..));
            render_pass.set_bind_group(0, &self.uniforms_bindgroup, &[]);

            let texture_bindgroup = &self.textures[texture_id].bindgroup;
            render_pass.set_bind_group(1, texture_bindgroup, &[]);

            render_pass.draw(0..verts.len() as u32, 0..1);
        } // We're dropping render_pass here to unborrow the encoder.

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn render_textured_quad(&mut self, texture_id: usize, matrix: Mat4) {
        let positions = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
        ];
        let uvs = vec![
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 0.0),
        ];
        self.render_triangles(&positions, None, Some((texture_id, &uvs)), matrix);
    }
}
