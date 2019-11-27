// render_system.rs
//
// Copyright (c) 2019, Univerisity of Minnesota
//
// Author: Bridger Herman (herma582@umn.edu)

//! Renders with WebGL, using wasm-bindgen and web-sys

use glam::Vec3;
use wasm_bindgen::JsCast;
use web_sys::{WebGl2RenderingContext, WebGlProgram};

use crate::entity::EntityId;
use crate::frame_buffer::FrameBuffer;
use crate::light::PointLight;
use crate::mesh::Mesh;
use crate::texture::{Texture, TextureId};
use crate::window::DEFAULT_WINDOW_SIZE;

pub struct WebGlContextWrapper {
    pub gl: WebGl2RenderingContext,
}

impl Default for WebGlContextWrapper {
    fn default() -> Self {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id("canvas").unwrap();
        let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("Unable to get canvas");

        let gl: WebGl2RenderingContext = canvas
            .get_context("webgl2")
            .unwrap()
            .unwrap()
            .dyn_into::<WebGl2RenderingContext>()
            .expect("Unable to get WebGL context");

        Self { gl }
    }
}

unsafe impl Send for WebGlContextWrapper {}

pub struct RenderSystem {
    lights: Vec<PointLight>,
    meshes: Vec<Mesh>,
    textures: Vec<Texture>,
    texture_paths: Vec<String>,
    shaders: Vec<WebGlProgram>,
    shader_names: Vec<String>,
    frame_buffer: FrameBuffer,
    ready: bool,
}

unsafe impl Send for RenderSystem {}

impl RenderSystem {
    pub fn render(&self) {
        if !self.ready {
            return;
        }

        self.frame_buffer.bind();

        wre_gl!().clear_color(0.0, 0.0, 0.0, 1.0);
        wre_gl!().clear(
            WebGl2RenderingContext::COLOR_BUFFER_BIT
                | WebGl2RenderingContext::DEPTH_BUFFER_BIT,
        );
        let (width, height) = DEFAULT_WINDOW_SIZE;
        wre_gl!().viewport(0, 0, width as i32, height as i32);

        // Save a texture image of rendered meshes to the frame buffer
        self.render_meshes();
        self.frame_buffer.unbind();

        // Splat that texture onto the viewport
        self.frame_buffer
            .render(self.get_shader_by_name("colorblur").unwrap());
    }

    /// Pass 1: Forward render all the meshes
    fn render_meshes(&self) {
        for mesh in &self.meshes {
            // Load the shader and VAO for this material and model
            let shader = &self.shaders[wre_entities!(mesh
                .attached_to
                .unwrap_or_default())
            .material()
            .shader_id];
            wre_gl!().use_program(Some(shader));
            wre_gl!().bind_vertex_array(Some(&mesh.vao));

            // Send the camera position for lighting information
            let camera_position: Vec<f32> =
                wre_camera!().transform().position().into();
            let camera_position_location =
                wre_gl!().get_uniform_location(shader, "uni_camera_position");
            wre_gl!().uniform3fv_with_f32_array(
                camera_position_location.as_ref(),
                &camera_position,
            );

            // Send the model matrix to the GPU
            let model_matrix =
                wre_entities!(mesh.attached_to.unwrap_or_default())
                    .transform()
                    .matrix();
            let model_uniform_location =
                wre_gl!().get_uniform_location(shader, "uni_model");
            wre_gl!().uniform_matrix4fv_with_f32_array(
                model_uniform_location.as_ref(),
                false,
                &model_matrix.to_flat_vec(),
            );

            // Send the normal matrix (inverse transpose of model matrix) to the
            // GPU for calculating transform of normals
            let normal_matrix = model_matrix.inverse().transpose();
            let normal_uniform_location =
                wre_gl!().get_uniform_location(shader, "uni_normal");
            wre_gl!().uniform_matrix4fv_with_f32_array(
                normal_uniform_location.as_ref(),
                false,
                &normal_matrix.to_flat_vec(),
            );

            // Send the camera's view/projection matrix to the GPU
            let view_matrix = wre_camera!().view_matrix();
            let projection_matrix = wre_camera!().projection_matrix();
            let projection_view = projection_matrix * view_matrix;
            let pv_uniform_location =
                wre_gl!().get_uniform_location(shader, "uni_projection_view");
            wre_gl!().uniform_matrix4fv_with_f32_array(
                pv_uniform_location.as_ref(),
                false,
                &projection_view.to_flat_vec(),
            );

            // Send all the lights over to the shader
            let num_light_location =
                wre_gl!().get_uniform_location(shader, "uni_num_lights");
            wre_gl!().uniform1i(
                num_light_location.as_ref(),
                self.lights.len() as i32,
            );

            let light_positions: Vec<f32> = self
                .lights
                .iter()
                .map(|light| -> Vec<f32> { light.position.into() })
                .flatten()
                .collect();
            let light_positions_location =
                wre_gl!().get_uniform_location(shader, "uni_light_positions");
            wre_gl!().uniform3fv_with_f32_array(
                light_positions_location.as_ref(),
                &light_positions,
            );

            let light_colors: Vec<f32> = self
                .lights
                .iter()
                .map(|light| -> Vec<f32> { light.color.into() })
                .flatten()
                .collect();
            let light_colors_location =
                wre_gl!().get_uniform_location(shader, "uni_light_colors");
            wre_gl!().uniform3fv_with_f32_array(
                light_colors_location.as_ref(),
                &light_colors,
            );

            // Send the material's color to the GPU
            let color: [f32; 4] =
                wre_entities!(mesh.attached_to.unwrap_or_default())
                    .material()
                    .color
                    .into();
            let color_uniform_location =
                wre_gl!().get_uniform_location(shader, "uni_color");
            wre_gl!().uniform4fv_with_f32_array(
                color_uniform_location.as_ref(),
                &color,
            );

            // Send the material's specularity to the GPU
            let specular: [f32; 4] =
                wre_entities!(mesh.attached_to.unwrap_or_default())
                    .material()
                    .specular
                    .into();
            let specular_uniform_location =
                wre_gl!().get_uniform_location(shader, "uni_specular");
            wre_gl!().uniform4fv_with_f32_array(
                specular_uniform_location.as_ref(),
                &specular,
            );

            // If there's a texture, send it to the GPU
            if let Some(texture_id) =
                wre_entities!(mesh.attached_to.unwrap_or_default())
                    .material()
                    .texture_id
            {
                wre_gl!().active_texture(WebGl2RenderingContext::TEXTURE0);
                wre_gl!().bind_texture(
                    WebGl2RenderingContext::TEXTURE_2D,
                    Some(&self.textures[texture_id].tex),
                );

                let tex_uniform_location =
                    wre_gl!().get_uniform_location(shader, "uni_texture");
                wre_gl!().uniform1i(tex_uniform_location.as_ref(), 0);
            }

            // Draw the geometry
            wre_gl!().draw_arrays(
                WebGl2RenderingContext::TRIANGLES,
                0,
                mesh.num_vertices,
            );

            wre_gl!().use_program(None);
        }
    }

    pub fn meshes(&self) -> &Vec<Mesh> {
        &self.meshes
    }

    pub fn make_ready(&mut self) {
        self.ready = true;
    }

    pub fn add_light(&mut self, position: Vec3, color: Vec3) {
        self.lights.push(PointLight::new(position, color));
    }

    pub fn add_shader(&mut self, name: &str, program: &WebGlProgram) -> usize {
        self.shaders.push(program.clone());
        self.shader_names.push(name.to_string());
        self.shaders.len() - 1
    }

    pub fn get_shader_by_name(&self, name: &str) -> Option<&WebGlProgram> {
        if let Some(index) = self.shader_names.iter().position(|n| n == name) {
            Some(&self.shaders[index])
        } else {
            None
        }
    }

    pub fn add_obj_mesh(&mut self, eid: EntityId, obj_text: &str) {
        let shader = &self.shaders[wre_entities!(eid).material().shader_id];
        wre_gl!().use_program(Some(shader));
        let mut mesh = Mesh::from_obj_str(obj_text);
        mesh.attached_to = Some(eid);
        self.meshes.push(mesh);
    }

    pub fn add_texture(&mut self, path: &str, png_bytes: &[u8]) -> TextureId {
        // Check to see if the texture is already in the list
        if let Some(index) = self.get_texture_id_by_path(path) {
            index
        } else {
            let tex = Texture::init_from_image(self.textures.len(), png_bytes);
            self.texture_paths.push(path.to_string());
            self.textures.push(tex);
            self.textures.len() - 1
        }
    }

    pub fn get_texture_id_by_path(&self, path: &str) -> Option<TextureId> {
        self.texture_paths.iter().position(|p| p == path)
    }
}

impl Default for RenderSystem {
    fn default() -> Self {
        // Enable depth testing for proper object occlusion
        wre_gl!().enable(WebGl2RenderingContext::DEPTH_TEST);

        Self {
            lights: Vec::default(),
            texture_paths: Vec::default(),
            textures: Vec::default(),
            meshes: Vec::default(),
            shaders: Vec::default(),
            shader_names: Vec::default(),
            frame_buffer: FrameBuffer::default(),
            ready: false,
        }
    }
}
