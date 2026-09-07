use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

use iced::futures::executor::block_on;
use iced::wgpu;
use iced::Point;
use image::RgbaImage;
use tracing::{error, warn};

use nyanko::graphics::animate::{resolve_frame, FrameData};
use nyanko::graphics::rig::{Animation, Rig};
use nyanko::graphics::tools::part;

use kore::systems::animation::export::process::calculate_export_time;
use kore::systems::animation::export::{EncoderMessage, ExportMode, FrameTiming, ShowcaseLengths};
use kore::domains::settings::Scope;
use kore::systems::animation::{multiply_mat3, Role};

use crate::systems::animation;

use super::diagnostics;
use super::pipeline::{build_vertices, Painted, Pipeline};

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub region_x: f32,
    pub region_y: f32,
    pub zoom: f32,
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: Pipeline,
    target: wgpu::Texture,
    readback: wgpu::Buffer,
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
}

impl Renderer {
    pub fn new(region_w: f32, region_h: f32) -> Result<Self, String> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .map_err(|e| format!("No GPU adapter available for export rendering: {e}"))
            .inspect_err(|message| error!("{message}"))?;

        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .map_err(|e| format!("Failed to create GPU device for export rendering: {e}"))
            .inspect_err(|message| error!("{message}"))?;

        let max_dim = device.limits().max_texture_dimension_2d;
        let width = (region_w.round() as u32).clamp(1, max_dim);
        let height = (region_h.round() as u32).clamp(1, max_dim);

        let pipeline = Pipeline::create(&device, wgpu::TextureFormat::Rgba8Unorm);

        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("animation_export_target"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let padded_bytes_per_row = align_bytes_per_row(width);

        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("animation_export_readback"),
            size: padded_bytes_per_row as u64 * height as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            device,
            queue,
            pipeline,
            target,
            readback,
            width,
            height,
            padded_bytes_per_row,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    fn render_frame(
        &mut self,
        unit: &Rig,
        animation: Option<&Animation>,
        frame_time: f32,
        take: Take<'_>,
    ) -> Result<Vec<u8>, String> {
        let Take { camera, background, offset, debug } = take;
        let at = frame_time.floor() as i32;
        let whole = || resolve_frame(unit, animation, at, offset);

        let mapped = debug.and_then(|_| {
            part::resolve(unit, animation, at, offset)
                .ok()
                .map(|held| held.into_iter().map(|entry| (entry.part, entry.frame)).collect::<Vec<_>>())
        });

        let mut parts = match debug {
            Some(shot) => scoped(unit, shot, mapped.as_deref(), &whole),
            None => whole(),
        };

        remap_glow(&mut parts);

        let mut pixels =
            self.render_parts(unit.sheet.image_data.as_ref(), &parts, camera, background)?;

        if let Some(shot) = debug {
            let marked: Vec<(Option<usize>, FrameData)> = match mapped {
                Some(held) => held.into_iter().map(|(part, frame)| (Some(part), frame)).collect(),
                None => whole().into_iter().map(|frame| (None, frame)).collect(),
            };

            let to_screen = |x: f32, y: f32| {
                Point::new((x - camera.region_x) * camera.zoom, (y - camera.region_y) * camera.zoom)
            };

            diagnostics::paint(&mut pixels, self.width, self.height, unit, &marked, to_screen, shot);
        }

        Ok(pixels)
    }

    fn render_parts(
        &mut self,
        image: Option<&Arc<RgbaImage>>,
        parts: &[FrameData],
        camera: Camera,
        background: [u8; 4],
    ) -> Result<Vec<u8>, String> {
        self.pipeline.batches.clear();

        let view_proj = export_view_proj(self.width as f32, self.height as f32, camera);

        let mut can_draw = false;
        if let Some(image) = image
            && !parts.is_empty() {
            self.pipeline.upload_atlas(&self.device, &self.queue, image);

            let plain: Vec<Painted> = parts.iter().cloned().map(Painted::plain).collect();
            let (vertex_data, index_data, batches) = build_vertices(&plain, &view_proj);
            self.pipeline.batches = batches;
            self.pipeline.upload_vertices(&self.device, &self.queue, &vertex_data);
            self.pipeline.upload_indices(&self.device, &self.queue, &index_data);
            can_draw = true;
        }

        let clear_color = wgpu::Color {
            r: background[0] as f64 / 255.0,
            g: background[1] as f64 / 255.0,
            b: background[2] as f64 / 255.0,
            a: background[3] as f64 / 255.0,
        };

        let view = self.target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("animation_export"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("animation_export"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if can_draw
                && let (Some(atlas), Some(vertices), Some(indices)) =
                    (&self.pipeline.atlas, &self.pipeline.vertices, &self.pipeline.indices) {
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, vertices.slice(..));
                pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);

                for batch in &self.pipeline.batches {
                    pass.set_pipeline(&self.pipeline.pipelines[batch.variant]);
                    pass.draw_indexed(batch.range.clone(), 0, 0..1);
                }
            }
        }

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit([encoder.finish()]);

        let slice = self.readback.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

        if let Err(e) = self.device.poll(wgpu::PollType::wait_indefinitely()) {
            let message = format!("GPU poll failed during export readback: {e}");
            error!("{message}");
            return Err(message);
        }

        match receiver.recv() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                let message = format!("Export readback mapping failed: {e}");
                error!("{message}");
                return Err(message);
            }
            Err(_) => {
                let message = "Export readback mapping was never signalled".to_string();
                error!("{message}");
                return Err(message);
            }
        }

        let pixels = {
            let data = slice.get_mapped_range();
            unpad_rows_bottom_up(&data, self.width, self.height, self.padded_bytes_per_row)
        };
        self.readback.unmap();

        Ok(pixels)
    }
}

fn scoped(
    unit: &Rig,
    shot: &diagnostics::Shot,
    mapped: Option<&[(usize, FrameData)]>,
    whole: &impl Fn() -> Vec<FrameData>,
) -> Vec<FrameData> {
    if shot.scope == Scope::Rig {
        return whole();
    }

    if shot.scope == Scope::None {
        return Vec::new();
    }

    let Some(mapped) = mapped else {
        return whole();
    };

    mapped
        .iter()
        .filter(|(part, _)| animation::shows(&unit.model, shot.scope, shot.picked, *part))
        .map(|(_, frame)| frame.clone())
        .collect()
}

struct Take<'a> {
    camera: Camera,
    background: [u8; 4],
    offset: Option<usize>,
    debug: Option<&'a diagnostics::Shot>,
}

pub struct Job {
    pub unit: Arc<Rig>,
    pub animation: Option<Arc<Animation>>,
    pub role_paths: Vec<(Role, PathBuf)>,
    pub offset: Option<usize>,
    pub timing: FrameTiming,
    pub lengths: ShowcaseLengths,
    pub camera: Camera,
    pub region_w: f32,
    pub region_h: f32,
    pub fps: i32,
    pub background: bool,
    pub debug: Option<diagnostics::Shot>,
    pub tx: mpsc::Sender<EncoderMessage>,
    pub abort: Arc<AtomicBool>,
    pub progress: Arc<AtomicI32>,
}

pub fn spawn(job: Job) {
    thread::spawn(move || run(job));
}

fn run(job: Job) {
    let Ok(mut renderer) = Renderer::new(job.region_w, job.region_h)
        .inspect_err(|e| error!("Export render thread could not start: {e}"))
    else {
        return;
    };

    let clips = if job.timing.mode == ExportMode::Showcase {
        Some(ShowcaseClips::load(&job.role_paths))
    } else {
        None
    };

    let frame_count = (job.timing.frame_end - job.timing.frame_start).abs() + 1;
    let delay_ms = (1000.0 / job.fps as f32) as u32;
    let background = if job.background { [80, 80, 80, 255] } else { [0, 0, 0, 0] };

    for progress in 0..frame_count {
        if job.abort.load(Ordering::Relaxed) {
            warn!("Export render loop aborted at frame {progress}/{frame_count}");
            return;
        }

        let (animation, local_time) = match &clips {
            Some(clips) => {
                let (role, time) = showcase_segment(job.lengths, progress);
                let animation = clips.get(role);
                let time = match animation {
                    Some(anim) if anim.declared_frames() > 1 => time,
                    _ => 0.0,
                };
                (animation, time)
            }
            None => (job.animation.as_deref(), 0.0),
        };

        let frame_time = calculate_export_time(&job.timing, animation, local_time, progress);

        match renderer.render_frame(
            &job.unit,
            animation,
            frame_time,
            Take {
                camera: job.camera,
                background,
                offset: job.offset,
                debug: job.debug.as_ref(),
            },
        ) {
            Ok(pixels) => {
                let frame = EncoderMessage::Frame(pixels, renderer.width(), renderer.height(), delay_ms);
                if job.tx.send(frame).is_err() {
                    warn!("Encoder channel closed mid-export; stopping render loop");
                    return;
                }
            }
            Err(e) => error!("Export rendering failed at frame {progress}: {e}"),
        }

        job.progress.store(progress + 1, Ordering::Relaxed);
    }

    if job.tx.send(EncoderMessage::Finish).is_err() {
        warn!("Encoder channel closed before Finish could be sent");
    }
}

struct ShowcaseClips {
    walk: Option<Animation>,
    idle: Option<Animation>,
    attack: Option<Animation>,
    kb: Option<Animation>,
}

impl ShowcaseClips {
    fn load(role_paths: &[(Role, PathBuf)]) -> Self {
        let parse = |role: Role| -> Option<Animation> {
            let (_, path) = role_paths.iter().find(|(known, _)| *known == role)?;
            let bytes = fs::read(path).ok()?;
            Animation::parse(&bytes).ok()
        };

        Self {
            walk: parse(Role::Walk),
            idle: parse(Role::Idle),
            attack: parse(Role::Attack),
            kb: parse(Role::Knockback),
        }
    }

    fn get(&self, role: Role) -> Option<&Animation> {
        match role {
            Role::Walk => self.walk.as_ref(),
            Role::Idle => self.idle.as_ref(),
            Role::Attack => self.attack.as_ref(),
            Role::Knockback => self.kb.as_ref(),
        }
    }
}

fn showcase_segment(lengths: ShowcaseLengths, progress: i32) -> (Role, f32) {
    let walk = lengths.walk;
    let idle = lengths.idle;
    let attack = lengths.attack;

    if progress < walk {
        (Role::Walk, (progress % walk.max(1)) as f32)
    } else if progress < walk + idle {
        (Role::Idle, ((progress - walk) % idle.max(1)) as f32)
    } else if progress < walk + idle + attack {
        (Role::Attack, (progress - (walk + idle)) as f32)
    } else {
        let relative = progress - (walk + idle + attack);
        (Role::Knockback, (relative % lengths.kb.max(1)) as f32)
    }
}

fn align_bytes_per_row(width: u32) -> u32 {
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    (width * 4).div_ceil(align) * align
}

fn remap_glow(parts: &mut [FrameData]) {
    for part in parts {
        match part.glow {
            1 | 3 => part.glow = 1,
            2 => part.glow = 0,
            _ => {}
        }
    }
}

fn unpad_rows_bottom_up(data: &[u8], width: u32, height: u32, padded_bytes_per_row: u32) -> Vec<u8> {
    let row_bytes = (width * 4) as usize;
    let padded = padded_bytes_per_row as usize;
    let mut pixels = Vec::with_capacity(row_bytes * height as usize);

    for row in (0..height as usize).rev() {
        let start = row * padded;
        pixels.extend_from_slice(&data[start..start + row_bytes]);
    }

    pixels
}

fn export_view_proj(width: f32, height: f32, camera: Camera) -> [f32; 9] {
    let projection = [
        2.0 / width, 0.0, 0.0,
        0.0, -2.0 / height, 0.0,
        -1.0, 1.0, 1.0,
    ];

    let zoom = camera.zoom;
    let pan_x = -camera.region_x - width / (2.0 * zoom);
    let pan_y = -camera.region_y - height / (2.0 * zoom);

    let view = [
        zoom, 0.0, 0.0,
        0.0, zoom, 0.0,
        (width / 2.0) + pan_x * zoom,
        (height / 2.0) + pan_y * zoom,
        1.0,
    ];

    multiply_mat3(&projection, &view)
}

