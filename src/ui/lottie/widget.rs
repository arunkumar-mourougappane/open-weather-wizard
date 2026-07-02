//! # Animated Lottie Widget
//!
//! Renders a single frame of a `velato::Composition` (a parsed Lottie
//! animation) directly into iced's own `wgpu` render target, with no CPU pixel
//! readback. Proven viable by `examples/lottie_spike.rs`: depending on
//! `velato` alone (not a separately-versioned `vello`) makes `velato::vello`'s
//! `wgpu` requirement unify with iced's own `wgpu` dependency into a single
//! crate instance, which is what allows iced's `shader::Program`/`Primitive`
//! hooks to hand `vello::Renderer` iced's actual `wgpu::Device`/`Queue`.
//!
//! Rendering strategy: `vello::Renderer::render_to_texture` needs device+queue
//! together, which only `Primitive::prepare()` has, so each frame is rendered
//! into an offscreen `Rgba8Unorm` storage texture there (`vello`'s internal
//! compute pipeline requires that exact format, regardless of the window's
//! actual swapchain format). `Primitive::draw()` then composites that texture
//! into iced's target via a textured full-screen-triangle blit inside the
//! render pass iced already provides, scoped to this widget's bounds.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use iced::widget::shader::{self, Pipeline, Primitive, Shader};
use iced::{Element, Rectangle, mouse, wgpu};

use velato::vello;

/// Identifies one animated icon's cached offscreen texture: iced's renderer
/// calls `prepare()` on *every* primitive in a frame before calling `draw()`
/// on any of them, and every `LottiePrimitive` shares one `LottiePipeline`
/// instance (iced's `Storage` keys pipelines by primitive *type*, not by
/// instance) -- so a single shared texture slot gets overwritten by each
/// icon in turn before any of them are actually drawn, leaving every icon
/// showing blank or the wrong frame. Keying the cache by composition
/// identity + pixel size gives each on-screen icon its own texture.
type CacheKey = (usize, u32, u32);

fn cache_key(composition: &Arc<velato::Composition>, bounds: &Rectangle) -> CacheKey {
    (
        Arc::as_ptr(composition) as usize,
        bounds.width.max(1.0) as u32,
        bounds.height.max(1.0) as u32,
    )
}

/// Renders one frame of `composition` at the given (fractional) `frame`
/// number, at `size` logical pixels square. Callers drive animation by
/// recomputing `frame` from elapsed time on each redraw (see
/// `lottie::frame_at`).
pub fn lottie<'a, Message>(
    composition: Arc<velato::Composition>,
    frame: f64,
    size: f32,
) -> Element<'a, Message>
where
    Message: 'a,
{
    Shader::new(LottieProgram { composition, frame })
        .width(size)
        .height(size)
        .into()
}

struct LottieProgram {
    composition: Arc<velato::Composition>,
    frame: f64,
}

impl<Message> shader::Program<Message> for LottieProgram {
    type State = ();
    type Primitive = LottiePrimitive;

    fn draw(&self, _state: &(), _cursor: mouse::Cursor, bounds: Rectangle) -> LottiePrimitive {
        LottiePrimitive {
            composition: self.composition.clone(),
            frame: self.frame,
            bounds,
        }
    }
}

#[derive(Debug)]
struct LottiePrimitive {
    composition: Arc<velato::Composition>,
    frame: f64,
    bounds: Rectangle,
}

/// WGSL for a full-screen-triangle blit: samples the offscreen texture vello
/// rendered into and writes it into iced's target via a render pass already
/// scoped to this primitive's bounds (per the `Primitive::draw` contract).
const BLIT_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var out: VertexOutput;
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    out.uv = uv;
    out.position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    out.position.y = -out.position.y;
    return out;
}

@group(0) @binding(0) var t_offscreen: texture_2d<f32>;
@group(0) @binding(1) var s_offscreen: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_offscreen, s_offscreen, in.uv);
}
"#;

/// One icon's cached GPU resources: the offscreen texture vello renders each
/// frame into, and the bind group the blit pipeline samples it through.
struct CachedIcon {
    // Never read directly, but must outlive `view`/`bind_group`, which are
    // handles into it -- kept here so the cache entry owns it.
    #[allow(dead_code)]
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}

/// Shared across every `LottiePrimitive` (keyed by `TypeId` in iced's
/// `Storage`, so one `Pipeline` instance serves all animated icons at once):
/// owns the `vello::Renderer`, the blit pipeline that composites rendered
/// frames into iced's actual target, and a per-icon texture cache (see
/// `CacheKey` for why this can't just be a single shared texture).
///
/// `vello::Renderer` contains an internal `RefCell`-based buffer cache and so
/// is not `Sync`, while iced's `Pipeline` trait requires it (its `Storage`
/// type-erases pipelines behind `Box<dyn Pipeline>`, even on native targets
/// where nothing is actually cross-thread) -- the `Mutex` wrapper exists
/// purely to satisfy that bound; access is always single-threaded in practice.
struct LottiePipeline {
    renderer: Mutex<vello::Renderer>,
    icons: HashMap<CacheKey, CachedIcon>,
    blit_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl Pipeline for LottiePipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let renderer = vello::Renderer::new(device, vello::RendererOptions::default())
            .expect("failed to create vello::Renderer sharing iced's wgpu::Device");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lottie-blit-shader"),
            source: wgpu::ShaderSource::Wgsl(BLIT_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lottie-blit-bind-group-layout"),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lottie-blit-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lottie-blit-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("lottie-blit-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            renderer: Mutex::new(renderer),
            icons: HashMap::new(),
            blit_pipeline,
            bind_group_layout,
            sampler,
        }
    }
}

impl Primitive for LottiePrimitive {
    type Pipeline = LottiePipeline;

    fn prepare(
        &self,
        pipeline: &mut LottiePipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        _viewport: &shader::Viewport,
    ) {
        let key = cache_key(&self.composition, bounds);
        let (width, height) = (key.1, key.2);

        if !pipeline.icons.contains_key(&key) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("lottie-offscreen"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                // vello's internal compute pipeline writes to this as a storage
                // texture, which constrains the format to Rgba8Unorm regardless
                // of iced's actual swapchain/target format -- the blit shader's
                // fragment stage converts on sample, so that mismatch is fine.
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("lottie-blit-bind-group"),
                layout: &pipeline.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
                    },
                ],
            });

            pipeline.icons.insert(
                key,
                CachedIcon {
                    texture,
                    view,
                    bind_group,
                },
            );
        }

        // The composition's own coordinate space (e.g. 100x100 units, set by
        // its `w`/`h` fields) rarely matches the target texture's pixel size,
        // and `Renderer::append` does not scale to fit on its own -- without
        // this, a composition larger than the target just gets cropped to
        // its top-left corner instead of scaled down.
        let scale_x = f64::from(width) / self.composition.width as f64;
        let scale_y = f64::from(height) / self.composition.height as f64;
        let transform = vello::kurbo::Affine::scale_non_uniform(scale_x, scale_y);

        let mut scene = vello::Scene::new();
        let mut lottie_renderer = velato::Renderer::new();
        lottie_renderer.append(&self.composition, self.frame, transform, 1.0, &mut scene);

        let icon = &pipeline.icons[&key];
        pipeline
            .renderer
            .lock()
            .expect("lottie renderer mutex poisoned")
            .render_to_texture(
                device,
                queue,
                &scene,
                &icon.view,
                &vello::RenderParams {
                    base_color: vello::peniko::Color::TRANSPARENT,
                    width,
                    height,
                    antialiasing_method: vello::AaConfig::Area,
                },
            )
            .expect("vello render_to_texture failed");
    }

    fn draw(&self, pipeline: &LottiePipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        let key = cache_key(&self.composition, &self.bounds);
        let Some(icon) = pipeline.icons.get(&key) else {
            return true;
        };

        render_pass.set_pipeline(&pipeline.blit_pipeline);
        render_pass.set_bind_group(0, &icon.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
        true
    }
}
