use crate::math::transform_2d;
use crate::prelude::*;
use bytemuck;
use pollster;
use std::mem::size_of;
use std::sync::Arc;
use wgpu;
use winit::window::Window;

const WHITE_TEXTURE_ID: usize = 0;

struct Texture {
    texture: wgpu::Texture,
    size: wgpu::Extent3d,
    bindgroup: wgpu::BindGroup,
}

struct FrameObjects {
    surface_texture: wgpu::SurfaceTexture,
    command_encoder: wgpu::CommandEncoder,
    render_pass: Option<wgpu::RenderPass<'static>>,
}

pub struct Mesh {
    vert_count: usize,
    positions: wgpu::Buffer,
    normals: wgpu::Buffer,
    vert_colors: wgpu::Buffer,
    uvs: wgpu::Buffer,
    pub texture: usize, // TODO: this pub is smelly.
}

impl Mesh {
    pub fn new(
        positions: &[Vec3],
        vert_colors: Option<&[Vec4]>,
        texture_id_and_uvs: Option<(usize, &[Vec2])>,
        gpu: &Gpu,
    ) -> Self {
        let mut mesh = Self::allocate(positions.len(), gpu);
        mesh.write(positions, vert_colors, texture_id_and_uvs, gpu);
        mesh
    }

    pub fn new_2d(
        positions: &[Vec2],
        vert_colors: Option<&[Vec4]>,
        texture_id_and_uvs: Option<(usize, &[Vec2])>,
        gpu: &Gpu,
    ) -> Self {
        let mut positions_3d = Vec::with_capacity(positions.len());
        for pos in positions {
            positions_3d.push(Vec3::new(pos.x, pos.y, 0.0));
        }
        Self::new(&positions_3d, vert_colors, texture_id_and_uvs, gpu)
    }

    fn allocate(vert_count: usize, gpu: &Gpu) -> Self {
        let positions = Self::create_vertex_buffer(vert_count * size_of::<[f32; 3]>(), &gpu.device);
        let normals = Self::create_vertex_buffer(vert_count * size_of::<[f32; 3]>(), &gpu.device);
        let vert_colors =
            Self::create_vertex_buffer(vert_count * size_of::<[f32; 4]>(), &gpu.device);
        let uvs = Self::create_vertex_buffer(vert_count * size_of::<[f32; 2]>(), &gpu.device);

        Self {
            vert_count,
            positions,
            normals,
            vert_colors,
            uvs,
            texture: 0,
        }
    }

    fn write(
        &mut self,
        positions: &[Vec3],
        vert_colors: Option<&[Vec4]>,
        texture_id_and_uvs: Option<(usize, &[Vec2])>,
        gpu: &Gpu,
    ) {
        debug_assert_eq!(positions.len(), self.vert_count);
        Self::write_vec3_slice_to_buffer(&self.positions, positions, &gpu.queue);

        // Default normals for each triangle
        let mut normals = vec![Vec3::ZERO; self.vert_count];
        for i in (0..positions.len()).step_by(3) {
            let v0 = positions[i];
            let v1 = positions[i + 1];
            let v2 = positions[i + 2];
            let normal = (v1 - v0).cross(v2 - v0).normalize();

            normals[i] = normal;
            normals[i + 1] = normal;
            normals[i + 2] = normal;
        }
        Self::write_vec3_slice_to_buffer(&self.normals, &normals, &gpu.queue);

        if let Some(colors) = vert_colors {
            debug_assert_eq!(colors.len(), self.vert_count);
            Self::write_vec4_slice_to_buffer(&self.vert_colors, colors, &gpu.queue);
        } else {
            // Disable vertex colors by just multiplying the texture with white in the shader.
            let white = Vec4::new(1.0, 1.0, 1.0, 1.0);
            Self::write_vec4_slice_to_buffer(
                &self.vert_colors,
                &vec![white; positions.len()],
                &gpu.queue,
            );
        }

        if let Some((id, uvs)) = texture_id_and_uvs {
            self.texture = id;
            debug_assert_eq!(uvs.len(), self.vert_count);
            Self::write_vec2_slice_to_buffer(&self.uvs, uvs, &gpu.queue);
        } else {
            self.texture = WHITE_TEXTURE_ID;
        }
    }

    fn create_vertex_buffer(num_bytes: usize, device: &wgpu::Device) -> wgpu::Buffer {
        let desc = wgpu::BufferDescriptor {
            label: None,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            size: num_bytes as u64,
            mapped_at_creation: false,
        };
        device.create_buffer(&desc)
    }

    fn write_vec2_slice_to_buffer(buffer: &wgpu::Buffer, slice: &[Vec2], queue: &wgpu::Queue) {
        let mut floats: Vec<f32> = Vec::with_capacity(slice.len() * 2); // Assume Vec2 or bigger.
        for i in 0..slice.len() {
            let a = slice[i].to_array();
            floats.extend_from_slice(&a);
        }
        debug_assert!(floats.len() == slice.len() * 2);
        let bytes = bytemuck::cast_slice(&floats);
        queue.write_buffer(buffer, 0, bytes);
    }

    fn write_vec3_slice_to_buffer(buffer: &wgpu::Buffer, slice: &[Vec3], queue: &wgpu::Queue) {
        let mut floats: Vec<f32> = Vec::with_capacity(slice.len() * 3);
        for i in 0..slice.len() {
            let a = slice[i].to_array();
            floats.extend_from_slice(&a);
        }
        debug_assert!(floats.len() == slice.len() * 3);
        let bytes = bytemuck::cast_slice(&floats);
        queue.write_buffer(buffer, 0, bytes);
    }

    fn write_vec4_slice_to_buffer(buffer: &wgpu::Buffer, slice: &[Vec4], queue: &wgpu::Queue) {
        let mut floats: Vec<f32> = Vec::with_capacity(slice.len() * 4);
        for i in 0..slice.len() {
            let a = slice[i].to_array();
            floats.extend_from_slice(&a);
        }
        debug_assert!(floats.len() == slice.len() * 4);
        let bytes = bytemuck::cast_slice(&floats);
        queue.write_buffer(buffer, 0, bytes);
    }
}

struct Uniform {
    buffer: wgpu::Buffer,
    bindgroup: wgpu::BindGroup,
}

impl Uniform {
    fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let size = size_of::<Mat4>() + size_of::<Vec4>();
        debug_assert_eq!(size, 16 * 4 + 4 * 4);

        let buffer = {
            let desc = wgpu::BufferDescriptor {
                label: None,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                size: size as u64,
                mapped_at_creation: false,
            };
            device.create_buffer(&desc)
        };

        let bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: None,
        });

        Self { buffer, bindgroup }
    }

    fn as_bytes(&self, matrix: &Mat4, color: &Vec4) -> Vec<u8> {
        let matrix_floats = matrix.to_cols_array();
        let matrix_bytes = bytemuck::bytes_of(&matrix_floats);

        let color_floats = color.to_array();
        let color_bytes = bytemuck::bytes_of(&color_floats);

        let mut uniform_bytes = Vec::with_capacity(matrix_bytes.len() + color_bytes.len());
        uniform_bytes.extend_from_slice(matrix_bytes);
        uniform_bytes.extend_from_slice(color_bytes);
        uniform_bytes
    }
}

pub struct Gpu<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipelines: [wgpu::RenderPipeline; 4],
    depth_texture_view: wgpu::TextureView,
    uniform_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    textures: Vec<Texture>,
    frame_objects: Option<FrameObjects>,
    busy_uniforms: Vec<Uniform>,
    idle_uniforms: Vec<Uniform>,
    width: usize,
    height: usize,
    render_count: u32,
}

impl<'a> Gpu<'a> {
    // These bitflags are OR'd together to create an index into the pipelines array.
    pub const FEATURE_DEPTH: usize = 0b0001;
    pub const FEATURE_LIGHT: usize = 0b0010;

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.width() as f32 / self.height() as f32
    }

    pub fn window_to_normalized_transform(&self) -> Mat4 {
        let height = self.height() as f32;
        let pixels_to_normalized = Mat4::from_scale(Vec3::new(2.0 / height, -2.0 / height, 1.0));
        let translation = Mat4::from_translation(Vec3::new(-self.aspect_ratio(), 1.0, 0.0));
        translation * pixels_to_normalized
    }

    pub fn window_to_normalized(&self, window_pos: &Vec2) -> Vec2 {
        transform_2d(&window_pos, &self.window_to_normalized_transform())
    }

    pub fn normalized_to_window(&self, normalized_pos: &Vec2) -> Vec2 {
        transform_2d(
            &normalized_pos,
            &self.window_to_normalized_transform().inverse(),
        )
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
            let info = adapter.get_info();
            println!(
                "backend: {}\nDriver: {}\nInfo: {}",
                info.backend, info.driver, info.driver_info
            );
            let limits = adapter.limits();
            println!("2D texture limit: {}", limits.max_texture_dimension_2d);
            (surface, adapter)
        };

        let mut limits = wgpu::Limits::downlevel_defaults();
        limits.max_texture_dimension_2d = 2048;

        // Increase the texture size limit if it's smaller than the window.
        while limits.max_texture_dimension_2d < window.inner_size().width
            || limits.max_texture_dimension_2d < window.inner_size().height
        {
            limits.max_texture_dimension_2d *= 2;
        }
        println!(
            "Adjusted 2D texture limit: {}",
            limits.max_texture_dimension_2d
        );

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .unwrap();

        let size = window.inner_size(); // Size in physical pixels
        let surface_config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();
        // TODO: try surface_config.desired_maximum_frame_latency = 1;
        debug_assert_eq!(surface_config.present_mode, wgpu::PresentMode::Fifo);
        surface.configure(&device, &surface_config);

        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: None,
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let pipelines: [wgpu::RenderPipeline; 4] = {
            let mut pipelines = vec![];
            for flags in 0..4 {
                pipelines.push(Self::create_pipeline(
                    &device,
                    &surface_config,
                    &[&uniform_layout, &texture_layout],
                    flags,
                ));
            }
            pipelines.try_into().unwrap()
        };

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("depth texture"),
            view_formats: &[],
        });

        let mut gpu = Self {
            width: window.inner_size().width as usize,
            height: window.inner_size().height as usize,
            surface,
            device,
            queue,
            pipelines,
            depth_texture_view: depth_texture.create_view(&wgpu::TextureViewDescriptor::default()),
            uniform_layout,
            texture_layout,
            textures: vec![],
            frame_objects: None,
            busy_uniforms: vec![],
            idle_uniforms: vec![],
            render_count: 0,
        };

        // The white texture is used when the user doesn't want texturing; the vertex
        // colors get multiplied with white (255u8), allowing the texturing pipeline to
        // handle non-textured meshes.
        let white_texture = gpu.create_texture(1, 1, false);
        gpu.write_rgba_texture(white_texture, &[255u8; 4]);
        debug_assert_eq!(white_texture, WHITE_TEXTURE_ID);

        gpu
    }

    fn create_pipeline(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
        feature_flags: usize,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/default.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts,
            push_constant_ranges: &[],
        });
        let vertpos_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<[f32; 3]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            }],
        };
        let vertnormal_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<[f32; 3]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x3,
            }],
        };
        let vertcolor_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<[f32; 4]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x4,
            }],
        };
        let uv_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<[f32; 2]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 3,
                format: wgpu::VertexFormat::Float32x2,
            }],
        };

        let mut compilation_options: wgpu::PipelineCompilationOptions = Default::default();
        let mut constants_hash = HashMap::new();
        if feature_flags & Gpu::FEATURE_LIGHT != 0 {
            constants_hash.insert("LIGHTING_ENABLED".to_string(), 1.0);
        }
        compilation_options.constants = &constants_hash;

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    vertpos_layout,
                    vertnormal_layout,
                    vertcolor_layout,
                    uv_layout,
                ],
                compilation_options: compilation_options.clone(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING), // TODO: not premultiplied
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: compilation_options,
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: if feature_flags & Gpu::FEATURE_DEPTH != 0 {
                    wgpu::CompareFunction::Less
                } else {
                    wgpu::CompareFunction::Always
                },
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
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
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.texture_layout,
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

    pub fn write_monochrome_texture(&self, texture_id: usize, pixels: &[u8]) {
        let texture = &self.textures[texture_id];
        debug_assert_eq!(
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
        debug_assert_eq!(
            pixel_bytes.len(),
            (texture.size.width * texture.size.height * 4) as usize,
            "expected entire 8bit RGBA pixel data"
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

    pub fn set_render_features(&mut self, feature_flags: usize) {
        let pipeline = &self.pipelines[feature_flags];
        self.frame_objects
            .as_mut()
            .unwrap()
            .render_pass
            .as_mut()
            .unwrap()
            .set_pipeline(&pipeline);
    }

    pub fn begin_frame(&mut self) {
        let surface_texture = self.surface.get_current_texture().unwrap();

        let mut command_encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut render_pass = command_encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            })
            .forget_lifetime();

        // todo is it necessary to set the pipeline here?
        render_pass.set_pipeline(&self.pipelines[0]);

        self.frame_objects = Some(FrameObjects {
            surface_texture,
            command_encoder,
            render_pass: Some(render_pass),
        });

        self.render_count = 0;
    }

    pub fn finish_frame(&mut self) {
        let mut frame_objects = std::mem::take(&mut self.frame_objects).unwrap();
        frame_objects.render_pass = None; // Finish the render pass

        let finished_command_buffer = frame_objects.command_encoder.finish();
        self.queue.submit(std::iter::once(finished_command_buffer));

        std::mem::swap(&mut self.idle_uniforms, &mut self.busy_uniforms);

        frame_objects.surface_texture.present();
    }

    pub fn render_mesh(&mut self, mesh: &Mesh, matrix: &Mat4, color: Option<Vec4>) {
        let uniform = match self.idle_uniforms.pop() {
            Some(m) => m,
            None => Uniform::new(&self.device, &self.uniform_layout),
        };

        // Write the uniform to its wgpu buffer
        let color = match color {
            Some(c) => c,
            None => Vec4::new(1.0, 1.0, 1.0, 1.0),
        };
        let aspect_ratio_transform =
            Mat4::from_scale(Vec3::new(1.0 / self.aspect_ratio(), 1.0, 1.0));
        self.queue.write_buffer(
            &uniform.buffer,
            0,
            &uniform.as_bytes(&(aspect_ratio_transform * *matrix), &color),
        );

        let render_pass = self
            .frame_objects
            .as_mut()
            .unwrap()
            .render_pass
            .as_mut()
            .unwrap();

        render_pass.set_vertex_buffer(0, mesh.positions.slice(..));
        render_pass.set_vertex_buffer(1, mesh.normals.slice(..));
        render_pass.set_vertex_buffer(2, mesh.vert_colors.slice(..));
        render_pass.set_vertex_buffer(3, mesh.uvs.slice(..));
        render_pass.set_bind_group(0, &uniform.bindgroup, &[]);

        let texture_bindgroup = &self.textures[mesh.texture].bindgroup;
        render_pass.set_bind_group(1, texture_bindgroup, &[]);

        render_pass.draw(0..mesh.vert_count as u32, 0..1);

        self.busy_uniforms.push(uniform);
        self.render_count += 1;
    }
}
