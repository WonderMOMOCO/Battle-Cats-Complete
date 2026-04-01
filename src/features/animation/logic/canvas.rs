use eframe::egui;
use eframe::glow::{self, HasContext};
use std::sync::{Arc, Mutex};
use crate::global::formats::imgcut::SpriteSheet;
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
    uniform int u_is_glow;
    varying vec2 v_texcoord;

    void main() {
        vec4 tex_color = texture2D(u_texture, v_texcoord);
        
        if (u_is_glow == 1) {
            float brightness = max(tex_color.r, max(tex_color.g, tex_color.b));
            gl_FragColor = vec4(tex_color.rgb, brightness) * u_opacity;
        } else {
            gl_FragColor = tex_color * u_opacity;
        }
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
            let vertex_array = gl_context.create_vertex_array().expect("Failed to create VAO - GL Context invalid");
            let vbo = gl_context.create_buffer().expect("Failed to create VBO - GL Context invalid");
            let tbo = gl_context.create_buffer().expect("Failed to create TBO - GL Context invalid");

            gl_context.bind_vertex_array(Some(vertex_array));
            
            gl_context.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let position_location = gl_context.get_attrib_location(program, "a_position").unwrap_or(0);
            gl_context.enable_vertex_attrib_array(position_location);
            gl_context.vertex_attrib_pointer_f32(position_location, 2, glow::FLOAT, false, 0, 0);

            gl_context.bind_buffer(glow::ARRAY_BUFFER, Some(tbo));
            let texture_location = gl_context.get_attrib_location(program, "a_texcoord").unwrap_or(1);
            gl_context.enable_vertex_attrib_array(texture_location);
            gl_context.vertex_attrib_pointer_f32(texture_location, 2, glow::FLOAT, false, 0, 0);

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
            if !allow_update && self.texture.is_some() { 
                return; 
            }

            // Data Check
            let Some(image) = &sheet.image_data else {
                return; 
            };

            // Texture Recycling
            let texture_id = if let Some(existing_texture) = self.texture {
                // Bind the existing texture ID
                gl_context.bind_texture(glow::TEXTURE_2D, Some(existing_texture));
                existing_texture
            } else {
                // First time only
                let new_texture = gl_context.create_texture().expect("Failed to allocate texture on GPU");
                gl_context.bind_texture(glow::TEXTURE_2D, Some(new_texture));
                
                // Set parameters only on creation
                gl_context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
                gl_context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
                gl_context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
                gl_context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
                
                new_texture
            };

            // Convert & Upload Pixels
            let pixels = &image.pixels;
            let mut data: Vec<u8> = Vec::with_capacity(pixels.len() * 4);
            
            let gamma_value: f32 = 1.883;
            let inverse_gamma = 1.0 / gamma_value;
            let to_linear = |byte_value: u8| -> f32 { (byte_value as f32 / 255.0).powf(gamma_value) };
            let to_monitor = |value: f32| -> u8 { (value.powf(inverse_gamma) * 255.0 + 0.5).clamp(0.0, 255.0) as u8 };

            for pixel in pixels {
                let alpha_byte = pixel.a();
                
                if alpha_byte == 0 {
                    data.extend_from_slice(&[0, 0, 0, 0]);
                    continue;
                }
                
                let red_linear = to_linear(pixel.r());
                let green_linear = to_linear(pixel.g());
                let blue_linear = to_linear(pixel.b());
                let alpha_linear = alpha_byte as f32 / 255.0; 

                let red_premultiplied = red_linear * alpha_linear;
                let green_premultiplied = green_linear * alpha_linear;
                let blue_premultiplied = blue_linear * alpha_linear;

                data.push(to_monitor(red_premultiplied));
                data.push(to_monitor(green_premultiplied));
                data.push(to_monitor(blue_premultiplied));
                data.push(alpha_byte);
            }

            gl_context.tex_image_2d(
                glow::TEXTURE_2D, 0, glow::RGBA as i32,
                image.width() as i32, image.height() as i32, 0,
                glow::RGBA, glow::UNSIGNED_BYTE, Some(&data),
            );

            self.texture = Some(texture_id);
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

            // State resets to ensure stable layering
            gl_context.disable(glow::DEPTH_TEST);
            gl_context.depth_mask(false);
            gl_context.disable(glow::CULL_FACE);

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
            let u_is_glow = gl_context.get_uniform_location(self.program, "u_is_glow");
            
            gl_context.uniform_1_i32(u_texture.as_ref(), 0);

            gl_context.enable(glow::BLEND);

            for part in parts {
                if part.hidden || part.opacity < 0.005 { continue; }

                // Set uniform and handle blending
                gl_context.uniform_1_i32(u_is_glow.as_ref(), if part.glow > 0 { 1 } else { 0 });

                if part.glow > 0 {
                    // Fixes black boxes on export while keeping Z-order look
                    gl_context.blend_func_separate(glow::ONE, glow::ONE, glow::ONE, glow::ONE);
                } else {
                    gl_context.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
                }

                let Some(cut) = sheet.cuts_map.get(&part.sprite_index) else { continue; };
                
                let sprite_width = cut.original_size.x;
                let sprite_height = cut.original_size.y;
                let pivot_x = part.pivot.x;
                let pivot_y = part.pivot.y;

                let final_matrix = multiply_mat3(&view_matrix, &part.matrix);
                
                gl_context.uniform_matrix_3_f32_slice(u_transform.as_ref(), false, &final_matrix);
                gl_context.uniform_1_f32(u_opacity.as_ref(), part.opacity);

                let vertices: [f32; 12] = [
                    -pivot_x,               -pivot_y,          
                    sprite_width - pivot_x, -pivot_y,          
                    -pivot_x,               sprite_height - pivot_y,      
                    
                    -pivot_x,               sprite_height - pivot_y,      
                    sprite_width - pivot_x, -pivot_y,          
                    sprite_width - pivot_x, sprite_height - pivot_y,      
                ];
                
                gl_context.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
                gl_context.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytemuck::cast_slice(&vertices), glow::DYNAMIC_DRAW);

                let uv_coordinates = cut.uv_coordinates;
                let texture_coordinates: [f32; 12] = [
                    uv_coordinates.min.x, uv_coordinates.min.y, 
                    uv_coordinates.max.x, uv_coordinates.min.y, 
                    uv_coordinates.min.x, uv_coordinates.max.y, 
                    
                    uv_coordinates.min.x, uv_coordinates.max.y, 
                    uv_coordinates.max.x, uv_coordinates.min.y, 
                    uv_coordinates.max.x, uv_coordinates.max.y, 
                ];

                gl_context.bind_buffer(glow::ARRAY_BUFFER, Some(self.tbo));
                gl_context.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytemuck::cast_slice(&texture_coordinates), glow::DYNAMIC_DRAW);

                gl_context.draw_arrays(glow::TRIANGLES, 0, 6);
            }
            
            gl_context.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
        }
    }
}

fn multiply_mat3(matrix_a: &[f32; 9], matrix_b: &[f32; 9]) -> [f32; 9] {
    [
        matrix_a[0]*matrix_b[0] + matrix_a[3]*matrix_b[1] + matrix_a[6]*matrix_b[2],
        matrix_a[1]*matrix_b[0] + matrix_a[4]*matrix_b[1] + matrix_a[7]*matrix_b[2],
        matrix_a[2]*matrix_b[0] + matrix_a[5]*matrix_b[1] + matrix_a[8]*matrix_b[2],

        matrix_a[0]*matrix_b[3] + matrix_a[3]*matrix_b[4] + matrix_a[6]*matrix_b[5],
        matrix_a[1]*matrix_b[3] + matrix_a[4]*matrix_b[4] + matrix_a[7]*matrix_b[5],
        matrix_a[2]*matrix_b[3] + matrix_a[5]*matrix_b[4] + matrix_a[8]*matrix_b[5],

        matrix_a[0]*matrix_b[6] + matrix_a[3]*matrix_b[7] + matrix_a[6]*matrix_b[8],
        matrix_a[1]*matrix_b[6] + matrix_a[4]*matrix_b[7] + matrix_a[7]*matrix_b[8],
        matrix_a[2]*matrix_b[6] + matrix_a[5]*matrix_b[7] + matrix_a[8]*matrix_b[8],
    ]
}

unsafe fn compile_program(gl_context: &glow::Context, vertex_shader_source: &str, fragment_shader_source: &str) -> glow::Program {
    unsafe {
        let program = gl_context.create_program().expect("Cannot create OpenGL program");
        
        let vertex_shader = gl_context.create_shader(glow::VERTEX_SHADER).expect("Cannot create vertex shader");
        gl_context.shader_source(vertex_shader, vertex_shader_source);
        gl_context.compile_shader(vertex_shader);
        if !gl_context.get_shader_compile_status(vertex_shader) {
            panic!("{}", gl_context.get_shader_info_log(vertex_shader));
        }
        gl_context.attach_shader(program, vertex_shader);

        let fragment_shader = gl_context.create_shader(glow::FRAGMENT_SHADER).expect("Cannot create fragment shader");
        gl_context.shader_source(fragment_shader, fragment_shader_source);
        gl_context.compile_shader(fragment_shader);
        if !gl_context.get_shader_compile_status(fragment_shader) {
            panic!("{}", gl_context.get_shader_info_log(fragment_shader));
        }
        gl_context.attach_shader(program, fragment_shader);

        gl_context.link_program(program);
        if !gl_context.get_program_link_status(program) {
            panic!("{}", gl_context.get_program_info_log(program));
        }
        
        gl_context.delete_shader(vertex_shader);
        gl_context.delete_shader(fragment_shader);

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
            let Ok(mut renderer_lock) = renderer_ref.lock() else { return; };
            
            if renderer_lock.is_none() {
                *renderer_lock = Some(GlowRenderer::new(painter.gl()));
            }

            let Some(renderer) = renderer_lock.as_mut() else { return; };
            renderer.paint(painter.gl(), info.viewport, &parts, &sheet, pan, zoom, allow_update);
        })),
    };

    ui.painter().add(callback);
}