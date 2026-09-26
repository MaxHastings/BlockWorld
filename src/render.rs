use std::sync::mpsc;
use std::sync::Arc;
use std::time::Instant;

use bytemuck::Zeroable;
use wgpu::util::DeviceExt;
use winit::window::Window;

mod chunks;
mod geometry;
mod materials;
mod scene;
mod shadows;
mod sky;

use crate::game::Game;
use crate::terrain::material::{Material, ShadowPolicy, WIND_AMPLITUDE};
use crate::terrain::Terrain;
use chunks::{vertex_layout, ChunkCache};
use scene::{SceneFrame, Uniforms};
use shadows::{CASCADE_COUNT, SHADOW_MAP_SIZE};

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    terrain_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    sky_pipeline: wgpu::RenderPipeline,
    terrain_bind_group: wgpu::BindGroup,
    sky_bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    shadow_map: ShadowMap,
    depth_view: wgpu::TextureView,
    chunks: ChunkCache,
    gpu_queries: Option<wgpu::QuerySet>,
    gpu_pending: Option<GpuPending>,
    timestamp_inside_encoders: bool,
    profile_frames: u64,
    shadow_resident_radius: f32,
    previous_frame_time: Instant,
}

struct ShadowMap {
    _texture: wgpu::Texture,
    layer_views: [wgpu::TextureView; CASCADE_COUNT],
    caster_bind_group: wgpu::BindGroup,
}

struct GpuPending {
    buffer: wgpu::Buffer,
    receiver: mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
    shadow_profiled: bool,
}

impl Renderer {
    pub async fn new(window: Arc<Window>, profile: bool) -> Result<Self, String> {
        let size = window.inner_size();
        #[cfg(target_os = "windows")]
        let backends = wgpu::Backends::DX12;
        #[cfg(not(target_os = "windows"))]
        let backends = wgpu::Backends::PRIMARY;
        // Avoid the secondary OpenGL backend. Windows uses its native D3D12
        // backend; other platforms retain wgpu's primary backend set.
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });
        let surface = instance.create_surface(window).map_err(|e| e.to_string())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| "Could not find a graphics adapter".to_string())?;
        let timestamp_feature = wgpu::Features::TIMESTAMP_QUERY;
        let adapter_features = adapter.features();
        let timestamp_supported = profile && adapter_features.contains(timestamp_feature);
        let timestamp_inside_encoders = timestamp_supported
            && adapter_features.contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS);
        let descriptor = wgpu::DeviceDescriptor {
            required_features: if timestamp_supported {
                timestamp_feature
                    | if timestamp_inside_encoders {
                        wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
                    } else {
                        wgpu::Features::empty()
                    }
            } else {
                wgpu::Features::empty()
            },
            ..Default::default()
        };
        let (device, queue) = adapter
            .request_device(&descriptor, None)
            .await
            .map_err(|e| format!("Could not create a graphics device: {e}"))?;
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .unwrap_or(capabilities.formats[0]);
        let present_mode = if capabilities
            .present_modes
            .contains(&wgpu::PresentMode::AutoVsync)
        {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::Fifo
        };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let (texture_view, atlas_sampler) = materials::load(&device, &queue)?;
        let (cloud_view, cloud_sampler) = sky::cloud_noise(&device, &queue);
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("directional shadow cascades"),
            size: wgpu::Extent3d {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
                depth_or_array_layers: CASCADE_COUNT as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_array_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("sampled directional shadow cascades"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let shadow_layer_views = std::array::from_fn(|layer| {
            shadow_texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("directional shadow cascade layer"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: layer as u32,
                array_layer_count: Some(1),
                ..Default::default()
            })
        });
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("directional shadow comparison sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera sky and lighting"),
            contents: bytemuck::bytes_of(&Uniforms::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let terrain_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terrain bindings"),
            entries: &[
                uniform_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });
        let shadow_caster_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("shadow caster bindings"),
                entries: &[
                    uniform_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT),
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let sky_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sky bindings"),
            entries: &[
                uniform_entry(0, wgpu::ShaderStages::FRAGMENT),
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let terrain_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terrain resources"),
            layout: &terrain_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&shadow_array_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
        });
        let shadow_map = ShadowMap {
            _texture: shadow_texture,
            layer_views: shadow_layer_views,
            caster_bind_group: device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shadow caster resources"),
                layout: &shadow_caster_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                    },
                ],
            }),
        };
        let sky_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sky resources"),
            layout: &sky_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&cloud_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&cloud_sampler),
                },
            ],
        });
        let terrain_shader_source = terrain_shader_source();
        let terrain_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("terrain shader"),
            source: wgpu::ShaderSource::Wgsl(terrain_shader_source.into()),
        });
        let sky_shader_source = format!(
            "{}\n{}",
            include_str!("render/scene.wgsl"),
            include_str!("render/sky.wgsl")
        );
        let sky_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sky shader"),
            source: wgpu::ShaderSource::Wgsl(sky_shader_source.into()),
        });
        let terrain_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("terrain pipeline"),
            layout: Some(&pipeline_layout(
                &device,
                "terrain pipeline layout",
                &terrain_layout,
            )),
            vertex: wgpu::VertexState {
                module: &terrain_shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &terrain_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("geometry directional shadow pipeline"),
            layout: Some(&pipeline_layout(
                &device,
                "shadow pipeline layout",
                &shadow_caster_layout,
            )),
            vertex: wgpu::VertexState {
                module: &terrain_shader,
                entry_point: Some("vs_shadow"),
                buffers: &[vertex_layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &terrain_shader,
                entry_point: Some("fs_shadow"),
                targets: &[],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                // A small clamped slope bias covers raster/shader depth
                // disagreement. Receiver-plane correction handles PCF taps.
                bias: wgpu::DepthBiasState {
                    constant: 1,
                    slope_scale: 1.0,
                    // Normalized depth; caps extreme slopes on near-edge-on faces.
                    clamp: 0.00005,
                },
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let sky_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky pipeline"),
            layout: Some(&pipeline_layout(
                &device,
                "sky pipeline layout",
                &sky_layout,
            )),
            vertex: wgpu::VertexState {
                module: &sky_shader,
                entry_point: Some("vs_sky"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &sky_shader,
                entry_point: Some("fs_sky"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let depth_view = create_depth_target(&device, &config);
        let gpu_queries = timestamp_supported.then(|| {
            device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("profile full GPU frame"),
                ty: wgpu::QueryType::Timestamp,
                count: 6,
            })
        });
        if profile && !timestamp_supported {
            eprintln!("profile GPU timestamps unavailable on this adapter");
        }
        Ok(Self {
            device,
            queue,
            surface,
            config,
            terrain_pipeline,
            shadow_pipeline,
            sky_pipeline,
            terrain_bind_group,
            sky_bind_group,
            uniform_buffer,
            shadow_map,
            depth_view,
            chunks: ChunkCache::new(profile)
                .map_err(|e| format!("Could not start terrain mesher: {e}"))?,
            gpu_queries,
            gpu_pending: None,
            timestamp_inside_encoders,
            profile_frames: 0,
            shadow_resident_radius: 0.0,
            previous_frame_time: Instant::now(),
        })
    }

    pub fn update_chunks(&mut self, terrain: &Terrain, x: f32, z: f32) {
        self.chunks.update(&self.device, terrain, x, z);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.depth_view = create_depth_target(&self.device, &self.config);
    }

    pub fn draw(&mut self, game: &Game) -> Result<(), wgpu::SurfaceError> {
        if let Some(pending) = &self.gpu_pending {
            self.device.poll(wgpu::Maintain::Poll);
            if let Ok(result) = pending.receiver.try_recv() {
                if result.is_ok() {
                    let data = pending.buffer.slice(..).get_mapped_range();
                    let ticks = bytemuck::cast_slice::<u8, u64>(&data);
                    let frame_end = if self.timestamp_inside_encoders {
                        ticks[1]
                    } else {
                        ticks[5]
                    };
                    let milliseconds = frame_end.saturating_sub(ticks[0]) as f64
                        * self.queue.get_timestamp_period() as f64
                        / 1_000_000.0;
                    let scope = if self.timestamp_inside_encoders {
                        "frame including shadows"
                    } else {
                        "scene passes (shadow time separate)"
                    };
                    eprintln!("profile GPU {scope}: {milliseconds:.2} ms");
                    let terrain_ms = ticks[5].saturating_sub(ticks[4]) as f64
                        * self.queue.get_timestamp_period() as f64
                        / 1_000_000.0;
                    eprintln!("profile GPU terrain and shadow sampling: {terrain_ms:.2} ms");
                    if pending.shadow_profiled {
                        let shadow_ms = ticks[3].saturating_sub(ticks[2]) as f64
                            * self.queue.get_timestamp_period() as f64
                            / 1_000_000.0;
                        eprintln!("profile GPU shadow generation: {shadow_ms:.2} ms");
                    }
                    drop(data);
                    pending.buffer.unmap();
                }
                self.gpu_pending = None;
            }
        }
        self.profile_frames += 1;
        let profile_gpu = self.profile_frames.is_multiple_of(30) && self.gpu_pending.is_none();
        let aspect = self.config.width as f32 / self.config.height as f32;
        let now = Instant::now();
        let elapsed = now.duration_since(self.previous_frame_time).as_secs_f32();
        self.previous_frame_time = now;
        // Reveal newly streamed coverage over time; never exceed actual residency.
        // One fade-width takes 0.25 seconds. Teleports/regeneration may shrink it
        // immediately, since retaining unavailable casters would be incorrect.
        self.shadow_resident_radius = self
            .chunks
            .resident_radius(game.position)
            .min(self.shadow_resident_radius + elapsed * shadows::DISTANCE_FADE / 0.25);
        let caster_bounds: Vec<_> = self
            .chunks
            .meshes()
            .filter(|mesh| mesh.casts_shadow)
            .map(|mesh| mesh.bounds)
            .collect();
        let scene = SceneFrame::new(
            game.position,
            game.forward(),
            game.day_seconds,
            game.sky_seconds,
            aspect,
            &caster_bounds,
            self.shadow_resident_radius,
        );
        let draw_shadows = scene.shadows.shadow_strength > 0.0 && scene.shadows.distance > 0.0;
        let profile_shadow_gpu = profile_gpu && draw_shadows;
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&scene.uniforms));
        let frame = self.surface.get_current_texture()?;
        let surface_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("terrain frame"),
            });
        if profile_gpu && self.timestamp_inside_encoders {
            if let Some(queries) = &self.gpu_queries {
                encoder.write_timestamp(queries, 0);
            }
        }
        for cascade in 0..if draw_shadows { CASCADE_COUNT } else { 0 } {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("directional geometry shadow pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_map.layer_views[cascade],
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: self
                    .gpu_queries
                    .as_ref()
                    .filter(|_| profile_shadow_gpu)
                    .map(|query_set| wgpu::RenderPassTimestampWrites {
                        query_set,
                        beginning_of_pass_write_index: (cascade == 0).then_some(2),
                        end_of_pass_write_index: (cascade + 1 == CASCADE_COUNT).then_some(3),
                    }),
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.shadow_pipeline);
            pass.set_bind_group(0, &self.shadow_map.caster_bind_group, &[]);
            for mesh in self.chunks.meshes() {
                if mesh.casts_shadow
                    && mesh
                        .bounds
                        .visible(scene.shadows.world_view_projection[cascade])
                {
                    pass.set_vertex_buffer(0, mesh.buffer.slice(..));
                    pass.draw(0..mesh.vertex_count, cascade as u32..cascade as u32 + 1);
                }
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sky pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: self
                    .gpu_queries
                    .as_ref()
                    .filter(|_| profile_gpu && !self.timestamp_inside_encoders)
                    .map(|query_set| wgpu::RenderPassTimestampWrites {
                        query_set,
                        beginning_of_pass_write_index: Some(0),
                        end_of_pass_write_index: None,
                    }),
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.sky_pipeline);
            pass.set_bind_group(0, &self.sky_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terrain pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: self.gpu_queries.as_ref().filter(|_| profile_gpu).map(
                    |query_set| wgpu::RenderPassTimestampWrites {
                        query_set,
                        beginning_of_pass_write_index: Some(4),
                        end_of_pass_write_index: Some(5),
                    },
                ),
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.terrain_pipeline);
            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
            for mesh in self.chunks.meshes() {
                if mesh.in_visible_range(game.position)
                    && mesh.bounds.visible(scene.view_projection)
                {
                    pass.set_vertex_buffer(0, mesh.buffer.slice(..));
                    pass.draw(0..mesh.vertex_count, 0..1);
                }
            }
        }
        if profile_gpu && self.timestamp_inside_encoders {
            if let Some(queries) = &self.gpu_queries {
                encoder.write_timestamp(queries, 1);
            }
        }
        let readback = if profile_gpu {
            self.gpu_queries.as_ref().map(|queries| {
                let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("profile GPU query resolve"),
                    size: 48,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                });
                let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("profile GPU timestamps"),
                    size: 48,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                encoder.resolve_query_set(queries, 0..6, &buffer, 0);
                encoder.copy_buffer_to_buffer(&buffer, 0, &readback, 0, 48);
                readback
            })
        } else {
            None
        };
        self.queue.submit(Some(encoder.finish()));
        if let Some(buffer) = readback {
            let (sender, receiver) = mpsc::channel();
            buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    let _ = sender.send(result);
                });
            self.gpu_pending = Some(GpuPending {
                buffer,
                receiver,
                shadow_profiled: profile_shadow_gpu,
            });
        }
        frame.present();
        Ok(())
    }
}

fn terrain_shader_source() -> String {
    let properties = Material::ALL
        .map(|material| {
            format!(
                "vec3<u32>({}u, {}u, {}u)",
                u32::from(material.alpha_test()),
                u32::from(material.shadow_policy() != ShadowPolicy::None),
                u32::from(material.wind()),
            )
        })
        .join(",\n");
    format!(
        "const MATERIAL_PROPERTIES = array<vec3<u32>, {}>({properties});\n\
         const WIND_AMPLITUDE = vec2<f32>({}, {});\n\
         const MAT_SPRUCE_LEAVES: u32 = {}u;\n{}\n{}",
        Material::ALL.len(),
        WIND_AMPLITUDE[0],
        WIND_AMPLITUDE[1],
        Material::SpruceLeaves as u32,
        include_str!("render/scene.wgsl"),
        include_str!("terrain.wgsl"),
    )
}

fn uniform_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

#[cfg(test)]
mod shader_tests {
    use super::*;

    #[test]
    fn combined_terrain_shadow_shader_is_valid_wgsl() {
        let source = terrain_shader_source();
        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("terrain and shadow WGSL should validate");
    }
}

fn pipeline_layout(
    device: &wgpu::Device,
    label: &str,
    bind_layout: &wgpu::BindGroupLayout,
) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &[bind_layout],
        push_constant_ranges: &[],
    })
}

fn create_depth_target(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("terrain depth"),
            size: wgpu::Extent3d {
                width: config.width.max(1),
                height: config.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}
