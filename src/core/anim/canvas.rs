use eframe::egui;
use eframe::glow::{self, HasContext};
use std::sync::{Arc, Mutex};
use crate::data::global::imgcut::SpriteSheet;
use super::transform::WorldTransform;

// Shaders
const VERTEX_SHADER_SOURCE: &str = r#"
    precision lowp float;
    attribute vec2 a_position;
    attribute vec2 a_texcoord;
    uniform mat3 u_transform;
    varying vec2 v_texcoord;

    void main() {
        vec3 pos = u_transform * vec3(a_position, 1.0);
        gl_Position = vec4(pos.xy, 0.0, 1.0);
        v_texcoord = a_texcoord;
    }
"#;

const FRAGMENT_SHADER_SOURCE: &str = r#"
    precision lowp float;
    uniform sampler2D u_texture;
    uniform float u_opacity;
    varying vec2 v_texcoord;

    void main() {
        gl_FragColor = texture2D(u_texture, v_texcoord) * u_opacity;
    }
"#;

// Renderer
pub struct GlowRenderer {
    program: glow::Program,
    vertex_array: glow::VertexArray,
    vbo: glow::Buffer, 
    tbo: glow::Buffer,
    texture: Option<glow::Texture>,
    last_sheet_name: String,
}

impl GlowRenderer {
    pub fn new(gl_context: &glow::Context) -> Self {
        unsafe {
            let program = compile_program(gl_context, VERTEX_SHADER_SOURCE, FRAGMENT_SHADER_SOURCE);
            let vertex_array = gl_context.create_vertex_array().expect("Failed to create VAO");
            let vbo = gl_context.create_buffer().expect("Failed to create VBO");
            let tbo = gl_context.create_buffer().expect("Failed to create TBO");

            gl_context.bind_vertex_array(Some(vertex_array));
            
            gl_context.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let pos_loc = gl_context.get_attrib_location(program, "a_position").unwrap_or(0);
            gl_context.enable_vertex_attrib_array(pos_loc);
            gl_context.vertex_attrib_pointer_f32(pos_loc, 2, glow::FLOAT, false, 0, 0);

            gl_context.bind_buffer(glow::ARRAY_BUFFER, Some(tbo));
            let tex_loc = gl_context.get_attrib_location(program, "a_texcoord").unwrap_or(1);
            gl_context.enable_vertex_attrib_array(tex_loc);
            gl_context.vertex_attrib_pointer_f32(tex_loc, 2, glow::FLOAT, false, 0, 0);

            gl_context.bind_vertex_array(None);

            Self {
                program,
                vertex_array,
                vbo,
                tbo,
                texture: None,
                last_sheet_name: String::new(),
            }
        }
    }

    fn upload_texture(&mut self, gl_context: &glow::Context, sheet: &SpriteSheet, allow_update: bool) {
        unsafe {
            // Cache Check
            if self.last_sheet_name == sheet.sheet_name && self.texture.is_some() {
                return;
            }

            // Safety Lock
            if !allow_update {
                if self.texture.is_some() { return; }
            }

            // Data Check
            let img = match &sheet.image_data {
                Some(data) => data,
                None => {
                    if self.texture.is_some() { return; }
                    return; 
                },
            };

            // Texture Recycling
            let tex_id = if let Some(existing_tex) = self.texture {
                // Bind the existing texture ID
                gl_context.bind_texture(glow::TEXTURE_2D, Some(existing_tex));
                existing_tex
            } else {
                // First time only
                let new_tex = gl_context.create_texture().expect("Failed to create texture");
                gl_context.bind_texture(glow::TEXTURE_2D, Some(new_tex));
                
                // Set parameters only on creation
                gl_context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
                gl_context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
                gl_context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
                gl_context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
                
                new_tex
            };

            // Convert & Upload Pixels
            let pixels = &img.pixels;
            let mut data: Vec<u8> = Vec::with_capacity(pixels.len() * 4);
            
            let gamma: f32 = 1.883;
            let inv_gamma = 1.0 / gamma;
            let to_linear = |byte_val: u8| -> f32 { (byte_val as f32 / 255.0).powf(gamma) };
            let to_monitor = |val: f32| -> u8 { (val.powf(inv_gamma) * 255.0 + 0.5).clamp(0.0, 255.0) as u8 };

            for pixel in pixels {
                let a_byte = pixel.a();
                if a_byte == 0 {
                    data.extend_from_slice(&[0, 0, 0, 0]);
                } else {
                    let r_lin = to_linear(pixel.r());
                    let g_lin = to_linear(pixel.g());
                    let b_lin = to_linear(pixel.b());
                    let a_lin = a_byte as f32 / 255.0; 

                    let r_pre = r_lin * a_lin;
                    let g_pre = g_lin * a_lin;
                    let b_pre = b_lin * a_lin;

                    data.push(to_monitor(r_pre));
                    data.push(to_monitor(g_pre));
                    data.push(to_monitor(b_pre));
                    data.push(a_byte);
                }
            }

            gl_context.tex_image_2d(
                glow::TEXTURE_2D, 0, glow::RGBA as i32,
                img.width() as i32, img.height() as i32, 0,
                glow::RGBA, glow::UNSIGNED_BYTE, Some(&data),
            );

            self.texture = Some(tex_id);
            self.last_sheet_name = sheet.sheet_name.clone();
        }
    }

    pub fn paint(
        &mut self, 
        gl_context: &glow::Context, 
        viewport: egui::Rect, 
        parts: &[WorldTransform], 
        sheet: &SpriteSheet, 
        pan: egui::Vec2, 
        zoom: f32,
        allow_update: bool
    ) {
        unsafe {
            self.upload_texture(gl_context, sheet, allow_update);
            
            if self.texture.is_none() { return; }

            gl_context.use_program(Some(self.program));
            gl_context.bind_vertex_array(Some(self.vertex_array));
            gl_context.active_texture(glow::TEXTURE0);
            gl_context.bind_texture(glow::TEXTURE_2D, self.texture);

            let w = viewport.width();
            let h = viewport.height();
            
            let projection = [
                2.0 / w, 0.0, 0.0,
                0.0, -2.0 / h, 0.0, 
                -1.0, 1.0, 1.0,
            ];

            let center_x = w / 2.0;
            let center_y = h / 2.0;
            
            let camera = [
                zoom, 0.0, 0.0,
                0.0, zoom, 0.0,
                center_x + pan.x * zoom, center_y + pan.y * zoom, 1.0
            ];

            let view_matrix = multiply_mat3(&projection, &camera);

            let u_transform = gl_context.get_uniform_location(self.program, "u_transform");
            let u_opacity = gl_context.get_uniform_location(self.program, "u_opacity");
            let u_texture = gl_context.get_uniform_location(self.program, "u_texture");
            gl_context.uniform_1_i32(u_texture.as_ref(), 0);

            gl_context.enable(glow::BLEND);

            for part in parts {
                if part.hidden || part.opacity < 0.005 { continue; }

                if part.glow > 0 {
                    gl_context.blend_func(glow::ONE, glow::ONE);
                } else {
                    gl_context.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
                }

                if let Some(cut) = sheet.cuts_map.get(&part.sprite_index) {
                    let sprite_w = cut.original_size.x;
                    let sprite_h = cut.original_size.y;
                    let pivot_x = part.pivot.x;
                    let pivot_y = part.pivot.y;

                    let final_matrix = multiply_mat3(&view_matrix, &part.matrix);
                    
                    gl_context.uniform_matrix_3_f32_slice(u_transform.as_ref(), false, &final_matrix);
                    gl_context.uniform_1_f32(u_opacity.as_ref(), part.opacity);

                    let vertices: [f32; 12] = [
                        -pivot_x,            -pivot_y,          
                        sprite_w - pivot_x,  -pivot_y,          
                        -pivot_x,            sprite_h - pivot_y,      
                        
                        -pivot_x,            sprite_h - pivot_y,      
                        sprite_w - pivot_x,  -pivot_y,          
                        sprite_w - pivot_x,  sprite_h - pivot_y,      
                    ];
                    
                    gl_context.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
                    gl_context.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytemuck::cast_slice(&vertices), glow::DYNAMIC_DRAW);

                    let uv = cut.uv_coordinates;
                    let tex_coords: [f32; 12] = [
                        uv.min.x, uv.min.y, 
                        uv.max.x, uv.min.y, 
                        uv.min.x, uv.max.y, 
                        
                        uv.min.x, uv.max.y, 
                        uv.max.x, uv.min.y, 
                        uv.max.x, uv.max.y, 
                    ];

                    gl_context.bind_buffer(glow::ARRAY_BUFFER, Some(self.tbo));
                    gl_context.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytemuck::cast_slice(&tex_coords), glow::DYNAMIC_DRAW);

                    gl_context.draw_arrays(glow::TRIANGLES, 0, 6);
                }
            }
            
            gl_context.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
        }
    }
}

fn multiply_mat3(a: &[f32; 9], b: &[f32; 9]) -> [f32; 9] {
    [
        a[0]*b[0] + a[3]*b[1] + a[6]*b[2],
        a[1]*b[0] + a[4]*b[1] + a[7]*b[2],
        a[2]*b[0] + a[5]*b[1] + a[8]*b[2],

        a[0]*b[3] + a[3]*b[4] + a[6]*b[5],
        a[1]*b[3] + a[4]*b[4] + a[7]*b[5],
        a[2]*b[3] + a[5]*b[4] + a[8]*b[5],

        a[0]*b[6] + a[3]*b[7] + a[6]*b[8],
        a[1]*b[6] + a[4]*b[7] + a[7]*b[8],
        a[2]*b[6] + a[5]*b[7] + a[8]*b[8],
    ]
}

unsafe fn compile_program(gl_context: &glow::Context, vs_source: &str, fs_source: &str) -> glow::Program {
    unsafe {
        let program = gl_context.create_program().expect("Cannot create program");
        
        let vert_shader = gl_context.create_shader(glow::VERTEX_SHADER).expect("cannot create vertex shader");
        gl_context.shader_source(vert_shader, vs_source);
        gl_context.compile_shader(vert_shader);
        if !gl_context.get_shader_compile_status(vert_shader) {
            panic!("{}", gl_context.get_shader_info_log(vert_shader));
        }
        gl_context.attach_shader(program, vert_shader);

        let frag_shader = gl_context.create_shader(glow::FRAGMENT_SHADER).expect("cannot create fragment shader");
        gl_context.shader_source(frag_shader, fs_source);
        gl_context.compile_shader(frag_shader);
        if !gl_context.get_shader_compile_status(frag_shader) {
            panic!("{}", gl_context.get_shader_info_log(frag_shader));
        }
        gl_context.attach_shader(program, frag_shader);

        gl_context.link_program(program);
        if !gl_context.get_program_link_status(program) {
            panic!("{}", gl_context.get_program_info_log(program));
        }
        
        gl_context.delete_shader(vert_shader);
        gl_context.delete_shader(frag_shader);

        program
    }
}

pub fn paint(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    renderer_ref: Arc<Mutex<Option<GlowRenderer>>>,
    sheet: Arc<SpriteSheet>,
    parts: Vec<WorldTransform>,
    pan: egui::Vec2,
    zoom: f32,
    allow_update: bool
) {
    let callback = egui::PaintCallback {
        rect,
        callback: Arc::new(eframe::egui_glow::CallbackFn::new(move |info, painter| {
            let mut renderer_lock = renderer_ref.lock().unwrap();
            
            if renderer_lock.is_none() {
                *renderer_lock = Some(GlowRenderer::new(painter.gl()));
            }

            if let Some(renderer) = renderer_lock.as_mut() {
                renderer.paint(painter.gl(), info.viewport, &parts, &sheet, pan, zoom, allow_update);
            }
        })),
    };

    ui.painter().add(callback);
}