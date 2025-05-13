use crate::math::transform_2d;
use crate::prelude::*;
use bitflags::bitflags;
use bytemuck;
use pollster;
use std::mem::size_of;
use std::sync::Arc;
use wgpu;
use winit::window::Window;

pub trait Gpu {
    // TODO: combine these
    fn create_texture(&mut self, width: usize, height: usize, linear_filtering: bool) -> usize;
    fn write_rgba_texture(&self, texture_id: usize, pixel_bytes: &[u8]);

    fn create_mesh(
        &self,
        positions: &[Vec3],
        colors: Option<&[Vec4]>,
        texture_id_and_uvs: Option<(usize, &[Vec2])>,
    ) -> Mesh;
    fn create_mesh_with_color(&self, positions: &[Vec3], color: &Vec4) -> Mesh {
        self.create_mesh(positions, Some(&vec![*color; positions.len()]), None)
    }
    fn render_mesh(&mut self, mesh: &Mesh, matrix: &Mat4);

    fn begin_frame(&mut self);
    fn set_camera(&mut self, matrix: &Mat4);
    fn set_render_features(&mut self, features: RenderFeatures);
    fn finish_frame(&mut self);

    fn width(&self) -> u32;
    fn height(&self) -> u32;

    fn aspect_ratio(&self) -> f32 {
        self.width() as f32 / self.height() as f32
    }

    fn window_to_normalized_transform(&self) -> Mat4 {
        let h = self.height() as f32;
        let pixels_to_normalized = Mat4::from_scale(Vec3::new(2.0 / h, -2.0 / h, 1.0));
        let translation = Mat4::from_translation(Vec3::new(-self.aspect_ratio(), 1.0, 0.0));
        translation * pixels_to_normalized
    }

    fn window_to_normalized(&self, window_pos: &Vec2) -> Vec2 {
        transform_2d(&window_pos, &self.window_to_normalized_transform())
    }
}

bitflags! {
    pub struct RenderFeatures: usize {
        const DEPTH = 0b0001;
        const LIGHT = 0b0010;
    }
}

const WHITE_TEXTURE_ID: usize = 0;
const MAX_SWAPCHAIN_SIZE: usize = 3;

struct Texture {
    texture: wgpu::Texture,
    size: wgpu::Extent3d,
    bindgroup: wgpu::BindGroup,
}

pub struct Mesh {
    vert_count: usize,
    positions: wgpu::Buffer,
    normals: wgpu::Buffer,
    colors: wgpu::Buffer,
    uvs: wgpu::Buffer,
    texture: usize,
}

struct Uniform {
    buffer: wgpu::Buffer,
    bindgroup: wgpu::BindGroup,
}

impl Uniform {
    fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let size = size_of::<Mat4>();
        debug_assert_eq!(size, 16 * 4);

        let buffer = {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                size: size as u64,
                mapped_at_creation: false,
            })
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
}

pub struct ImplGpu {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipelines: [wgpu::RenderPipeline; 4],
    depth_texture_view: wgpu::TextureView,
    texture_layout: wgpu::BindGroupLayout,
    textures: Vec<Texture>,
    uniform_layout: wgpu::BindGroupLayout,
    uniform_ring: Vec<Vec<Uniform>>,
    uniform_ring_index: usize,
    camera_uniform: Option<Uniform>,
    surface_texture: Option<wgpu::SurfaceTexture>,
    command_encoder: Option<wgpu::CommandEncoder>,
    render_pass: Option<wgpu::RenderPass<'static>>,
    width: u32,
    height: u32,
}

impl Gpu for ImplGpu {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn create_mesh(
        &self,
        positions: &[Vec3],
        colors: Option<&[Vec4]>,
        texture_id_and_uvs: Option<(usize, &[Vec2])>,
    ) -> Mesh {
        let v_count = positions.len();

        let pos_buf = self.create_vertex_buffer(v_count * size_of::<Vec3>());
        self.queue
            .write_buffer(&pos_buf, 0, bytemuck::cast_slice(positions));

        // Default normals for each triangle
        let normal_buf = self.create_vertex_buffer(v_count * size_of::<Vec3>());
        let mut normals = vec![Vec3::ZERO; v_count];
        for i in (0..v_count).step_by(3) {
            let v0 = positions[i];
            let v1 = positions[i + 1];
            let v2 = positions[i + 2];
            let normal = (v1 - v0).cross(v2 - v0).normalize();

            normals[i] = normal;
            normals[i + 1] = normal;
            normals[i + 2] = normal;
        }
        self.queue
            .write_buffer(&normal_buf, 0, bytemuck::cast_slice(&normals));

        let color_buf = self.create_vertex_buffer(v_count * size_of::<Vec4>());
        if let Some(colors) = colors {
            debug_assert_eq!(colors.len(), v_count);
            self.queue
                .write_buffer(&color_buf, 0, bytemuck::cast_slice(colors));
        } else {
            // Disable vertex colors by just multiplying the texture with white in the shader.
            let whites = vec![Vec4::splat(1.0); positions.len()];
            self.queue
                .write_buffer(&color_buf, 0, bytemuck::cast_slice(&whites));
        }

        let uv_buf = self.create_vertex_buffer(v_count * size_of::<Vec2>());
        let tex_id = if let Some((id, uvs)) = texture_id_and_uvs {
            debug_assert_eq!(uvs.len(), v_count);
            self.queue
                .write_buffer(&uv_buf, 0, bytemuck::cast_slice(uvs));
            id
        } else {
            WHITE_TEXTURE_ID
        };

        Mesh {
            vert_count: v_count,
            positions: pos_buf,
            normals: normal_buf,
            colors: color_buf,
            uvs: uv_buf,
            texture: tex_id,
        }
    }

    fn create_texture(&mut self, width: usize, height: usize, linear_filtering: bool) -> usize {
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
            label: None,
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

    fn write_rgba_texture(&self, texture_id: usize, pixel_bytes: &[u8]) {
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

    fn begin_frame(&mut self) {
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

        self.surface_texture = Some(surface_texture);
        self.command_encoder = Some(command_encoder);
        self.render_pass = Some(render_pass);
    }

    fn finish_frame(&mut self) {
        self.render_pass = None; // Finish the render pass

        let finished_command_buffer = self.command_encoder.take().unwrap().finish();
        self.queue.submit(std::iter::once(finished_command_buffer));

        self.uniform_ring_index += 1;
        while self.uniform_ring_index >= MAX_SWAPCHAIN_SIZE {
            self.uniform_ring_index -= MAX_SWAPCHAIN_SIZE;
        }

        self.surface_texture.take().unwrap().present();
    }

    fn set_render_features(&mut self, features: RenderFeatures) {
        let pipeline = &self.pipelines[features.bits()];
        self.render_pass.as_mut().unwrap().set_pipeline(&pipeline);
    }

    fn render_mesh(&mut self, mesh: &Mesh, matrix: &Mat4) {
        let model_uniform = self.pop_and_write_uniform(matrix);

        let render_pass = self.render_pass.as_mut().unwrap();

        render_pass.set_vertex_buffer(0, mesh.positions.slice(..));
        render_pass.set_vertex_buffer(1, mesh.normals.slice(..));
        render_pass.set_vertex_buffer(2, mesh.colors.slice(..));
        render_pass.set_vertex_buffer(3, mesh.uvs.slice(..));
        render_pass.set_bind_group(0, &self.camera_uniform.as_ref().unwrap().bindgroup, &[]);
        render_pass.set_bind_group(1, &model_uniform.bindgroup, &[]);

        let texture_bindgroup = &self.textures[mesh.texture].bindgroup;
        render_pass.set_bind_group(2, texture_bindgroup, &[]);

        render_pass.draw(0..mesh.vert_count as u32, 0..1);

        self.push_uniform(model_uniform);
    }

    fn set_camera(&mut self, matrix: &Mat4) {
        if let Some(cu) = self.camera_uniform.take() {
            self.push_uniform(cu);
        }

        let m = Mat4::from_scale(Vec3::new(1.0 / self.aspect_ratio(), 1.0, 1.0)) * *matrix;
        let u = self.pop_and_write_uniform(&m);

        self.camera_uniform = Some(u);
    }
}

impl ImplGpu {
    pub fn new(window: &Arc<Window>) -> Self {
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
            // For every feature e.g. DEPTH and LIGHT, we need:
            // A pipeline with depth+light
            // A pipeline with depth+nolight
            // A pipeline with light+nodepth
            // A pipeline with nodepth+nolight
            let feature_combinations_count = RenderFeatures::all()
                .iter()
                .map(|f| 2)
                .reduce(|a, b| a * b)
                .unwrap();

            let mut pipelines = vec![];
            for flags in 0..feature_combinations_count {
                pipelines.push(Self::create_pipeline(
                    &device,
                    &surface_config,
                    &[&uniform_layout, &uniform_layout, &texture_layout],
                    RenderFeatures::from_bits(flags).unwrap(),
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

        let uniform_ring = (0..MAX_SWAPCHAIN_SIZE).map(|_| Vec::new()).collect();

        let mut gpu = Self {
            width: window.inner_size().width,
            height: window.inner_size().height,
            surface,
            device,
            queue,
            pipelines,
            depth_texture_view: depth_texture.create_view(&wgpu::TextureViewDescriptor::default()),
            texture_layout,
            textures: vec![],
            surface_texture: None,
            command_encoder: None,
            render_pass: None,
            uniform_layout,
            uniform_ring,
            uniform_ring_index: 0,
            camera_uniform: None,
        };

        // The white texture is used when the user doesn't want texturing; the vertex
        // colors get multiplied with white (255u8), allowing the texturing pipeline to
        // handle non-textured meshes.
        let white_texture = gpu.create_texture(1, 1, false);
        gpu.write_rgba_texture(white_texture, &[255u8; 4]);
        debug_assert_eq!(white_texture, WHITE_TEXTURE_ID);

        gpu
    }

    fn pop_and_write_uniform(&mut self, matrix: &Mat4) -> Uniform {
        let u = if let Some(u_from_ring) = self.uniform_ring[self.uniform_ring_index].pop() {
            u_from_ring
        } else {
            Uniform::new(&self.device, &self.uniform_layout)
        };

        self.queue
            .write_buffer(&u.buffer, 0, bytemuck::bytes_of(matrix));
        u
    }

    fn push_uniform(&mut self, uniform: Uniform) {
        let mut end_of_ring = self.uniform_ring_index + MAX_SWAPCHAIN_SIZE - 1;
        while end_of_ring >= MAX_SWAPCHAIN_SIZE {
            end_of_ring -= MAX_SWAPCHAIN_SIZE;
        }

        self.uniform_ring[end_of_ring].push(uniform);
    }

    fn create_pipeline(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
        features: RenderFeatures,
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
        if features.contains(RenderFeatures::LIGHT) {
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
                depth_compare: if features.contains(RenderFeatures::DEPTH) {
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

    fn create_vertex_buffer(&self, num_bytes: usize) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            size: num_bytes as u64,
            mapped_at_creation: false,
        })
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
}
