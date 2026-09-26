use std::sync::mpsc;
use std::sync::Arc;

use bytemuck::Zeroable;
use wgpu::util::DeviceExt;
use winit::window::Window;

mod chunks;
mod heightfield;
mod materials;
mod scene;
mod sky;

use crate::game::Game;
use crate::terrain::material::Material;
use crate::terrain::Terrain;
use chunks::{vertex_layout, ChunkCache};
use heightfield::HeightField;
use scene::{SceneFrame, Uniforms};

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    terrain_pipeline: wgpu::RenderPipeline,
    sky_pipeline: wgpu::RenderPipeline,
    upscale_pipeline: wgpu::RenderPipeline,
    upscale_layout: wgpu::BindGroupLayout,
    upscale_sampler: wgpu::Sampler,
    terrain_bind_group: wgpu::BindGroup,
    sky_bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    render_target: RenderTarget,
    chunks: ChunkCache,
    heightfield: HeightField,
    gpu_queries: Option<wgpu::QuerySet>,
    gpu_pending: Option<(
        wgpu::Buffer,
        mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
    )>,
    profile_frames: u64,
}

struct RenderTarget {
    color_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    upscale_bind_group: wgpu::BindGroup,
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
        let timestamp_features = wgpu::Features::TIMESTAMP_QUERY;
        let timestamp_supported = profile && adapter.features().contains(timestamp_features);
        let descriptor = wgpu::DeviceDescriptor {
            required_features: if timestamp_supported {
                timestamp_features
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
        let heightfield = HeightField::new(&device, profile)
            .map_err(|e| format!("Could not start sun ray height worker: {e}"))?;
        let (cloud_view, cloud_sampler) = sky::cloud_noise(&device, &queue);
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
                        sample_type: wgpu::TextureSampleType::Sint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
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
                    resource: wgpu::BindingResource::TextureView(&heightfield.view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&heightfield.tree_view),
                },
            ],
        });
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
        let terrain_shader_source = format!(
            "const MAT_TALL_GRASS: u32 = {}u;\n\
             const MAT_DANDELION: u32 = {}u;\n\
             const MAT_OXEYE_DAISY: u32 = {}u;\n\
             const MAT_CORNFLOWER: u32 = {}u;\n\
             const MAT_SPRUCE_LOG: u32 = {}u;\n\
             const MAT_SPRUCE_LOG_TOP: u32 = {}u;\n\
             const MAT_SPRUCE_LEAVES: u32 = {}u;\n{}\n{}",
            Material::TallGrass as u32,
            Material::Dandelion as u32,
            Material::OxeyeDaisy as u32,
            Material::Cornflower as u32,
            Material::SpruceLog as u32,
            Material::SpruceLogTop as u32,
            Material::SpruceLeaves as u32,
            include_str!("render/scene.wgsl"),
            include_str!("terrain.wgsl"),
        );
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
        let upscale_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("upscale bindings"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let upscale_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("frame upscale sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let upscale_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("frame upscale shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("render/upscale.wgsl").into()),
        });
        let upscale_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("frame upscale pipeline"),
            layout: Some(&pipeline_layout(
                &device,
                "upscale pipeline layout",
                &upscale_layout,
            )),
            vertex: wgpu::VertexState {
                module: &upscale_shader,
                entry_point: Some("vs_upscale"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &upscale_shader,
                entry_point: Some("fs_upscale"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let render_target =
            create_render_target(&device, &config, &upscale_layout, &upscale_sampler);
        let gpu_queries = timestamp_supported.then(|| {
            device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("profile full GPU frame"),
                ty: wgpu::QueryType::Timestamp,
                count: 2,
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
            sky_pipeline,
            upscale_pipeline,
            upscale_layout,
            upscale_sampler,
            terrain_bind_group,
            sky_bind_group,
            uniform_buffer,
            render_target,
            chunks: ChunkCache::new(profile)
                .map_err(|e| format!("Could not start terrain mesher: {e}"))?,
            heightfield,
            gpu_queries,
            gpu_pending: None,
            profile_frames: 0,
        })
    }

    pub fn update_chunks(&mut self, terrain: &Terrain, x: f32, z: f32) {
        self.chunks.update(&self.device, terrain, x, z);
        self.heightfield.update(&self.queue, terrain, x, z);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.render_target = create_render_target(
            &self.device,
            &self.config,
            &self.upscale_layout,
            &self.upscale_sampler,
        );
    }

    pub fn draw(&mut self, game: &Game) -> Result<(), wgpu::SurfaceError> {
        if let Some((buffer, receiver)) = &self.gpu_pending {
            self.device.poll(wgpu::Maintain::Poll);
            if let Ok(result) = receiver.try_recv() {
                if result.is_ok() {
                    let data = buffer.slice(..).get_mapped_range();
                    let ticks = bytemuck::cast_slice::<u8, u64>(&data);
                    let milliseconds = ticks[1].saturating_sub(ticks[0]) as f64
                        * self.queue.get_timestamp_period() as f64
                        / 1_000_000.0;
                    eprintln!("profile GPU frame: {milliseconds:.2} ms");
                    drop(data);
                    buffer.unmap();
                }
                self.gpu_pending = None;
            }
        }
        self.profile_frames += 1;
        let profile_gpu = self.profile_frames.is_multiple_of(30) && self.gpu_pending.is_none();
        let aspect = self.config.width as f32 / self.config.height as f32;
        let scene = SceneFrame::new(
            game.position,
            game.forward(),
            game.day_seconds,
            game.sky_seconds,
            aspect,
        );
        let mut uniforms = scene.uniforms;
        uniforms.shadow_origin = [self.heightfield.origin[0], self.heightfield.origin[1], 0, 0];
        uniforms.shadow_params = [
            if self.heightfield.valid { 1.0 } else { 0.0 },
            self.heightfield.max_height,
            0.0,
            0.0,
        ];
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
        let frame = self.surface.get_current_texture()?;
        let surface_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("terrain frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sky pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.render_target.color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: self.gpu_queries.as_ref().filter(|_| profile_gpu).map(
                    |query_set| wgpu::RenderPassTimestampWrites {
                        query_set,
                        beginning_of_pass_write_index: Some(0),
                        end_of_pass_write_index: None,
                    },
                ),
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
                    view: &self.render_target.color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.render_target.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.terrain_pipeline);
            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
            for chunk in self.chunks.chunks.values() {
                if !chunk.visible(scene.view_projection) {
                    continue;
                }
                pass.set_vertex_buffer(0, chunk.terrain_buffer.slice(..));
                pass.draw(0..chunk.terrain_vertex_count, 0..1);
                if chunk.plants_near(game.position) {
                    if let Some(buffer) = &chunk.plant_buffer {
                        pass.set_vertex_buffer(0, buffer.slice(..));
                        pass.draw(0..chunk.plant_vertex_count, 0..1);
                    }
                }
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("upscale pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: self.gpu_queries.as_ref().filter(|_| profile_gpu).map(
                    |query_set| wgpu::RenderPassTimestampWrites {
                        query_set,
                        beginning_of_pass_write_index: None,
                        end_of_pass_write_index: Some(1),
                    },
                ),
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.upscale_pipeline);
            pass.set_bind_group(0, &self.render_target.upscale_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        let readback = if profile_gpu {
            self.gpu_queries.as_ref().map(|queries| {
                let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("profile GPU query resolve"),
                    size: 16,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                });
                let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("profile GPU timestamps"),
                    size: 16,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                encoder.resolve_query_set(queries, 0..2, &buffer, 0);
                encoder.copy_buffer_to_buffer(&buffer, 0, &readback, 0, 16);
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
            self.gpu_pending = Some((buffer, receiver));
        }
        frame.present();
        Ok(())
    }
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

fn create_render_target(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
) -> RenderTarget {
    const MAX_RENDER_PIXELS: f32 = 2560.0 * 1440.0;
    let surface_pixels = (config.width as f32) * (config.height as f32);
    let scale = (MAX_RENDER_PIXELS / surface_pixels).sqrt().min(1.0);
    let width = ((config.width as f32 * scale).round() as u32).max(1);
    let height = ((config.height as f32 * scale).round() as u32).max(1);
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("terrain color at render resolution"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: config.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());
    let depth_view = device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("terrain depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default());
    let upscale_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("frame upscale bindings"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&color_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    RenderTarget {
        color_view,
        depth_view,
        upscale_bind_group,
    }
}
