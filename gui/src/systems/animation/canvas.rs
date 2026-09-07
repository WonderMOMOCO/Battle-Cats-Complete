use std::sync::Arc;

use iced::mouse;
use iced::wgpu;
use iced::widget::shader::{self, Shader};
use iced::{Element, Event, Length, Point, Rectangle, Vector};
use image::RgbaImage;

use nyanko::graphics::animate::{resolve_frame, FrameData};

use kore::domains::settings::{Scope, StudioSettings};

use kore::systems::animation::multiply_mat3;

use crate::widget::LINE_PIXELS;

use super::data;
use super::pipeline::{build_vertices, Painted, Pipeline};

pub(super) const ZOOM_MIN: f32 = 0.1;
pub(super) const ZOOM_MAX: f32 = 10.0;
const ZOOM_RATE_PER_PIXEL: f32 = 0.0016;
const FRAME_ADVANCE_PER_TICK: f32 = 0.5;

#[derive(Debug, Clone)]
pub enum Message {
    Panned(Vector),
    Zoomed(f32),
    Tick,
}

fn zoom_delta(delta: &mouse::ScrollDelta) -> f32 {
    match delta {
        mouse::ScrollDelta::Lines { y, .. } => *y * LINE_PIXELS,
        mouse::ScrollDelta::Pixels { y, .. } => *y,
    }
}

pub struct State {
    pub pan: Vector,
    pub zoom: f32,
    pub current_frame: f32,
    pub is_playing: bool,
    pub loop_start: Option<f32>,
    pub loop_end: Option<f32>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            pan: Vector::new(0.0, 0.0),
            zoom: 1.0,
            current_frame: 0.0,
            is_playing: true,
            loop_start: None,
            loop_end: None,
        }
    }
}

impl State {
    pub fn update(&mut self, message: Message, data: &data::State) {
        match message {
            Message::Panned(delta) => {
                self.pan += Vector::new(delta.x / self.zoom, delta.y / self.zoom);
            }
            Message::Zoomed(pixels) => {
                let factor = (pixels * ZOOM_RATE_PER_PIXEL).exp();
                self.zoom = (self.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
            }
            Message::Tick => {
                if self.is_playing && data.held_unit.is_some() {
                    self.current_frame += FRAME_ADVANCE_PER_TICK;

                    let range_start = self.loop_start.unwrap_or(0.0);
                    let range_end = self.loop_end.or_else(|| data.loop_bound().map(|v| v.max(0) as f32));

                    if let Some(end) = range_end
                        && self.current_frame > end {
                        self.current_frame = range_start;
                    }
                }
            }
        }
    }

    pub fn view<'a>(
        &'a self,
        data: &'a data::State,
        anim: Option<&'a StudioSettings>,
        picked: Option<usize>,
    ) -> Element<'a, Message> {
        Shader::new(Viewport { state: self, data, anim, picked })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

struct Viewport<'a> {
    state: &'a State,
    data: &'a data::State,
    anim: Option<&'a StudioSettings>,
    picked: Option<usize>,
}

#[derive(Default)]
struct Interaction {
    drag_origin: Option<Point>,
}

impl Viewport<'_> {
    fn scoped(&self, unit: &nyanko::graphics::rig::Rig, frame: i32) -> Vec<FrameData> {
        let clip = self.data.current_anim.as_deref();
        let offset = self.data.offset();
        let entity = self.anim.map_or(Scope::Rig, |held| held.entity);

        if entity == Scope::Rig {
            return resolve_frame(unit, clip, frame, offset);
        }

        if entity == Scope::None {
            return Vec::new();
        }

        let Some(mapped) = self.data.mapped(frame) else {
            return resolve_frame(unit, clip, frame, offset);
        };

        mapped
            .into_iter()
            .filter(|entry| super::shows(&unit.model, entity, self.picked, entry.part))
            .map(|entry| entry.frame)
            .collect()
    }

    fn seated(&self, at: f32) -> i32 {
        self.data.playback_frame(at).floor() as i32
    }

    fn trailed(&self, at: f32) -> Option<i32> {
        self.data.trailed(at).map(|frame| frame.floor() as i32)
    }

    fn drawn(&self, unit: &nyanko::graphics::rig::Rig) -> Vec<Painted> {
        let now = self.state.current_frame;
        let live: Vec<Painted> =
            self.scoped(unit, self.seated(now)).into_iter().map(Painted::plain).collect();

        let Some(anim) = self.anim.filter(|held| held.onion_on()) else {
            return live;
        };

        let Some(gap) = anim.onion_step() else {
            return live;
        };

        let (behind, ahead) = (anim.onion_behind().unwrap_or(0), anim.onion_ahead().unwrap_or(0));
        let reach = (anim.onion_skins(behind) + anim.onion_skins(ahead)) as usize;
        let gap = gap as f32;
        let grid = (now / gap).floor() * gap;
        let seat = self.seated(now);

        let mut layered = Vec::with_capacity(live.len() * (reach + 1));

        for (life, way, first, tint) in [
            (behind, -1.0, 0, anim.onion_before_wash()),
            (ahead, 1.0, 1, anim.onion_after_wash()),
        ] {
            for step in (first..first + anim.onion_skins(life)).rev() {
                let laid = grid + way * (step as f32) * gap;
                let fade = anim.onion_fade((laid - now) * way, life);

                if fade <= 0.0 {
                    continue;
                }

                let Some(at) = self.trailed(laid).filter(|at| *at != seat) else {
                    continue;
                };

                layered.extend(self.scoped(unit, at).into_iter().filter_map(|mut ghost| {
                    ghost.opacity *= fade;

                    (ghost.opacity > 0.0).then_some(Painted { frame: ghost, tint })
                }));
            }
        }

        layered.extend(live);
        layered
    }
}

impl<'a> shader::Program<Message> for Viewport<'a> {
    type State = Interaction;
    type Primitive = Scene;

    fn update(
        &self,
        interaction: &mut Interaction,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<shader::Action<Message>> {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let position = cursor.position_in(bounds)?;
                interaction.drag_origin = Some(position);
                Some(shader::Action::capture())
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if interaction.drag_origin.take().is_some() {
                    Some(shader::Action::capture())
                } else {
                    None
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let origin = interaction.drag_origin?;
                let position = Point::new(position.x - bounds.x, position.y - bounds.y);
                interaction.drag_origin = Some(position);
                let delta = position - origin;
                Some(shader::Action::publish(Message::Panned(delta)).and_capture())
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if !cursor.is_over(bounds) {
                    return None;
                }

                let pixels = zoom_delta(delta);

                if pixels == 0.0 {
                    return None;
                }

                Some(shader::Action::publish(Message::Zoomed(pixels)).and_capture())
            }
            _ => None,
        }
    }

    fn draw(&self, _interaction: &Interaction, _cursor: mouse::Cursor, _bounds: Rectangle) -> Scene {
        let (parts, image) = self.data.held_unit.as_ref().map_or((Vec::new(), None), |unit| {
            (self.drawn(unit), unit.sheet.image_data.clone())
        });

        Scene {
            image,
            parts,
            pan: self.state.pan,
            zoom: self.state.zoom,
        }
    }

    fn mouse_interaction(&self, interaction: &Interaction, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if interaction.drag_origin.is_some() {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}

struct Scene {
    image: Option<Arc<RgbaImage>>,
    parts: Vec<Painted>,
    pan: Vector,
    zoom: f32,
}

impl std::fmt::Debug for Scene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scene")
            .field("parts", &self.parts.len())
            .field("pan", &self.pan)
            .field("zoom", &self.zoom)
            .finish()
    }
}

impl shader::Primitive for Scene {
    type Pipeline = Pipeline;

    fn prepare(
        &self,
        pipeline: &mut Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &shader::Viewport,
    ) {
        pipeline.batches.clear();

        let Some(image) = &self.image else {
            return;
        };

        if self.parts.is_empty() {
            return;
        }

        let physical = viewport.physical_size();
        let window_width = physical.width as f32;
        let window_height = physical.height as f32;

        if window_width <= 0.0 || window_height <= 0.0 {
            return;
        }

        pipeline.upload_atlas(device, queue, image);

        let scale = viewport.scale_factor();
        let zoom = self.zoom * scale;

        let projection = [
            2.0 / window_width, 0.0, 0.0,
            0.0, -2.0 / window_height, 0.0,
            -1.0, 1.0, 1.0,
        ];

        let camera = [
            zoom, 0.0, 0.0,
            0.0, zoom, 0.0,
            (bounds.x + bounds.width / 2.0) * scale + self.pan.x * zoom,
            (bounds.y + bounds.height / 2.0) * scale + self.pan.y * zoom,
            1.0,
        ];

        let view_proj = multiply_mat3(&projection, &camera);

        let (vertex_data, index_data, batches) = build_vertices(&self.parts, &view_proj);
        pipeline.batches = batches;
        pipeline.upload_vertices(device, queue, &vertex_data);
        pipeline.upload_indices(device, queue, &index_data);
    }

    fn render(
        &self,
        pipeline: &Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        if pipeline.batches.is_empty() || clip_bounds.width == 0 || clip_bounds.height == 0 {
            return;
        }

        let Some(atlas) = &pipeline.atlas else {
            return;
        };

        let Some(vertices) = &pipeline.vertices else {
            return;
        };

        let Some(indices) = &pipeline.indices else {
            return;
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("animation_viewer"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_scissor_rect(clip_bounds.x, clip_bounds.y, clip_bounds.width, clip_bounds.height);
        pass.set_bind_group(0, &atlas.bind_group, &[]);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);

        for batch in &pipeline.batches {
            pass.set_pipeline(&pipeline.pipelines[batch.variant]);
            pass.draw_indexed(batch.range.clone(), 0, 0..1);
        }
    }
}

