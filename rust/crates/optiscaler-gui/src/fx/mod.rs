//! GPU background effects: a single fullscreen-triangle wgpu pass painted
//! behind the UI. Repaint pacing lives in App::update — when effects are
//! off, unfocused, or reduced motion is on, this module draws nothing and
//! costs nothing.

use eframe::egui_wgpu::{self, wgpu};

/// Selectable background styles: (config value, display label). Labels are
/// proper names shared across languages.
pub const STYLES: [(&str, &str); 4] = [
    ("orbits", "Orbits"),
    ("aurora", "Aurora"),
    ("synthwave", "Synthwave"),
    ("nebula", "Nebula"),
];

/// Map a config style value to the WGSL style branch index.
pub fn style_index(name: &str) -> f32 {
    match name {
        "aurora" => 1.0,
        "synthwave" => 2.0,
        "nebula" => 3.0,
        _ => 0.0,
    }
}

pub struct EffectsRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Uniforms {
    time: f32,
    aspect: f32,
    intensity: f32,
    dark: f32,
    style: f32,
    _pad: [f32; 3],
}

impl EffectsRenderer {
    /// Create the pipeline once and stash it in egui_wgpu's callback
    /// resources. Call from App::new.
    pub fn register(render_state: &egui_wgpu::RenderState) {
        let device = &render_state.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fx_background"),
            source: wgpu::ShaderSource::Wgsl(include_str!("background.wgsl").into()),
        });
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fx_uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fx_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fx_bg"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fx_pl"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("fx_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(render_state.target_format.into())],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        render_state
            .renderer
            .write()
            .callback_resources
            .insert(Self {
                pipeline,
                bind_group,
                uniform_buffer,
            });
    }
}

/// Per-frame paint callback carrying the uniform values.
pub struct EffectsCallback {
    pub time: f32,
    pub aspect: f32,
    pub intensity: f32,
    pub dark: f32,
    /// Matches the WGSL style branch: 0 orbits, 1 aurora, 2 synthwave, 3 nebula.
    pub style: f32,
}

impl egui_wgpu::CallbackTrait for EffectsCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(renderer) = callback_resources.get::<EffectsRenderer>() {
            let uniforms = Uniforms {
                time: self.time,
                aspect: self.aspect,
                intensity: self.intensity,
                dark: self.dark,
                style: self.style,
                _pad: [0.0; 3],
            };
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    (&uniforms as *const Uniforms) as *const u8,
                    std::mem::size_of::<Uniforms>(),
                )
            };
            queue.write_buffer(&renderer.uniform_buffer, 0, bytes);
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: eframe::epaint::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        if let Some(renderer) = callback_resources.get::<EffectsRenderer>() {
            render_pass.set_pipeline(&renderer.pipeline);
            render_pass.set_bind_group(0, &renderer.bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    }
}

/// Windows "client area animations" accessibility setting — when the user
/// has disabled animations system-wide, effects default off. Declared
/// directly against user32 to avoid pulling in the full windows crate.
#[cfg(windows)]
pub fn reduced_motion() -> bool {
    const SPI_GETCLIENTAREAANIMATION: u32 = 0x1042;
    #[link(name = "user32")]
    unsafe extern "system" {
        fn SystemParametersInfoW(
            ui_action: u32,
            ui_param: u32,
            pv_param: *mut core::ffi::c_void,
            f_win_ini: u32,
        ) -> i32;
    }
    let mut animations_enabled: i32 = 1;
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            (&mut animations_enabled as *mut i32).cast(),
            0,
        )
    };
    ok != 0 && animations_enabled == 0
}

#[cfg(not(windows))]
pub fn reduced_motion() -> bool {
    false
}
