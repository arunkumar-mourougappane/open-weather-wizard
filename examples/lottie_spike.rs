//! # Lottie/iced GPU-integration spike (Phase C, throwaway prototype)
//!
//! Answers the open question from the migration plan: can `velato` (Lottie ->
//! `vello::Scene`) share iced's own `wgpu::Device`/`Queue` and composite directly
//! into iced's render target, instead of a CPU pixel-buffer round-trip?
//!
//! The key discovery driving this spike: don't add `vello` as a separate
//! top-level dependency (which pulled in a *second*, incompatible `wgpu` major
//! version and made GPU sharing a type-error). Instead depend on `velato` alone
//! and use its re-exported `velato::vello`, which pins the exact `vello` version
//! velato itself uses -- and that version's `wgpu` requirement turned out to
//! unify with iced's own `wgpu` dependency (both resolve to `wgpu 27.0.1`, a
//! single shared crate instance). That's what makes passing iced's `&wgpu::Device`
//! straight into `vello::Renderer::new()` type-check at all.
//!
//! Rendering strategy: `vello::Renderer::render_to_texture` wants device+queue,
//! which only `Primitive::prepare()` has -- so each frame is rendered into an
//! offscreen texture there. `Primitive::draw()` then composites that texture
//! into iced's actual target via a textured full-screen-triangle blit inside
//! the render pass iced already provides, scoped to this primitive's bounds --
//! fully on GPU, no CPU readback.
//!
//! Run with: `cargo run --example lottie_spike`

use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::mouse;
use iced::widget::shader::{self, Pipeline, Primitive, Shader};
use iced::widget::{center, column, text};
use iced::{Element, Fill, Rectangle, Size, Subscription};

use velato::vello;
use vello::kurbo;

/// A minimal hand-authored Lottie composition: a rotating filled circle.
/// Standing in for the real hand-authored sun/rain/snow/clouds icons that
/// Phase C.19 will produce, reusing the same rotate/opacity keyframe shape as
/// the existing CSS `@keyframes` in `assets/animated/*.svg`.
const SPIKE_LOTTIE_JSON: &str = r#"
{
  "v": "5.5.2", "fr": 30, "ip": 0, "op": 60, "w": 100, "h": 100,
  "nm": "spike", "ddd": 0, "assets": [],
  "layers": [
    {
      "ddd": 0, "ind": 1, "ty": 4, "nm": "circle", "sr": 1,
      "ks": {
        "o": { "a": 0, "k": 100 },
        "r": { "a": 1, "k": [
          { "t": 0, "s": [0] },
          { "t": 60, "s": [360] }
        ] },
        "p": { "a": 0, "k": [50, 50, 0] },
        "a": { "a": 0, "k": [0, 0, 0] },
        "s": { "a": 0, "k": [100, 100, 100] }
      },
      "ao": 0,
      "shapes": [
        {
          "ty": "gr",
          "it": [
            { "ty": "el", "p": { "a": 0, "k": [15, 0] }, "s": { "a": 0, "k": [30, 30] } },
            { "ty": "fl", "c": { "a": 0, "k": [1, 0.7, 0, 1] }, "o": { "a": 0, "k": 100 } },
            {
              "ty": "tr",
              "p": { "a": 0, "k": [0, 0] }, "a": { "a": 0, "k": [0, 0] },
              "s": { "a": 0, "k": [100, 100] }, "r": { "a": 0, "k": 0 }, "o": { "a": 0, "k": 100 }
            }
          ]
        }
      ],
      "ip": 0, "op": 60, "st": 0, "bm": 0
    }
  ]
}
"#;

struct State {
    composition: Arc<velato::Composition>,
    start: Instant,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Tick,
}

fn update(_state: &mut State, _message: Message) {}

fn view(state: &State) -> Element<'_, Message> {
    let elapsed = state.start.elapsed().as_secs_f64();
    let frame = (elapsed * state.composition.frame_rate) % state.composition.frames.end;

    column![
        text("Lottie/iced GPU-sharing spike -- rotating circle should animate below"),
        center(Shader::new(LottieProgram {
            composition: state.composition.clone(),
            frame,
        }))
        .width(Fill)
        .height(Fill),
    ]
    .spacing(12)
    .padding(12)
    .into()
}

fn subscription(_state: &State) -> Subscription<Message> {
    iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)
}

fn main() -> iced::Result {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    let composition = velato::Composition::from_slice(SPIKE_LOTTIE_JSON.as_bytes())
        .expect("spike Lottie JSON must parse");

    iced::application(
        move || State {
            composition: Arc::new(composition.clone()),
            start: Instant::now(),
        },
        update,
        view,
    )
    .subscription(subscription)
    .title("Lottie Spike")
    .run()
}

/// The `shader::Program` for a single frame of a Lottie composition.
struct LottieProgram {
    composition: Arc<velato::Composition>,
    frame: f64,
}

impl<Message> shader::Program<Message> for LottieProgram {
    type State = ();
    type Primitive = LottiePrimitive;

    fn draw(&self, _state: &(), _cursor: mouse::Cursor, _bounds: Rectangle) -> LottiePrimitive {
        LottiePrimitive {
            composition: self.composition.clone(),
            frame: self.frame,
        }
    }
}

#[derive(Debug)]
struct LottiePrimitive {
    composition: Arc<velato::Composition>,
    frame: f64,
}

/// WGSL for a full-screen-triangle blit: samples the offscreen texture vello
/// rendered into and writes it straight into iced's target via a render pass
/// already scoped to this primitive's bounds (by the `draw()` hook's contract).
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

/// Shared across every `LottiePrimitive` instance (keyed by `TypeId` in iced's
/// `Storage`): owns the `vello::Renderer`, a resizable offscreen texture that
/// each frame is rendered into, and a blit pipeline that composites it into
/// iced's actual target via a textured full-screen-triangle render pass.
struct LottiePipeline {
    renderer: std::sync::Mutex<vello::Renderer>,
    offscreen: Option<(wgpu::Texture, wgpu::TextureView, Size<u32>)>,
    blit_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    bind_group: Option<wgpu::BindGroup>,
}

impl Pipeline for LottiePipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let renderer = vello::Renderer::new(device, vello::RendererOptions::default())
            .expect("failed to create vello::Renderer sharing iced's wgpu::Device");
        log::info!("Lottie spike: vello::Renderer created against iced's own wgpu::Device");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lottie-spike-blit-shader"),
            source: wgpu::ShaderSource::Wgsl(BLIT_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lottie-spike-blit-bind-group-layout"),
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
            label: Some("lottie-spike-blit-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lottie-spike-blit-pipeline"),
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
            label: Some("lottie-spike-blit-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            renderer: std::sync::Mutex::new(renderer),
            offscreen: None,
            blit_pipeline,
            bind_group_layout,
            sampler,
            bind_group: None,
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
        let width = (bounds.width.max(1.0)) as u32;
        let height = (bounds.height.max(1.0)) as u32;

        let needs_new_texture = match &pipeline.offscreen {
            Some((_, _, size)) => size.width != width || size.height != height,
            None => true,
        };

        if needs_new_texture {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("lottie-spike-offscreen"),
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
                // of iced's actual swapchain/target format (Bgra8Unorm here) --
                // the blit shader's fragment stage converts on sample, so the
                // mismatch with the blit pipeline's own color target format
                // (set from the `format` passed into `Pipeline::new`) is fine.
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            pipeline.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("lottie-spike-blit-bind-group"),
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
            }));

            pipeline.offscreen = Some((texture, view, Size::new(width, height)));
        }

        let mut scene = vello::Scene::new();
        let mut lottie_renderer = velato::Renderer::new();
        lottie_renderer.append(
            &self.composition,
            self.frame,
            kurbo::Affine::IDENTITY,
            1.0,
            &mut scene,
        );

        let (_, offscreen_view, _) = pipeline.offscreen.as_ref().unwrap();
        pipeline
            .renderer
            .lock()
            .unwrap()
            .render_to_texture(
                device,
                queue,
                &scene,
                offscreen_view,
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
        // `render_pass` is already scoped to this primitive's bounds/clip (per
        // the `Primitive::draw` contract), so a textured full-screen-triangle
        // blit here composites the vello-rendered offscreen texture straight
        // into iced's target -- fully on GPU, no CPU readback. This is the
        // payoff of sharing iced's own wgpu::Device with vello: both the
        // offscreen texture and the target live in the same device.
        let Some(bind_group) = &pipeline.bind_group else {
            return true;
        };

        render_pass.set_pipeline(&pipeline.blit_pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);
        true
    }
}
