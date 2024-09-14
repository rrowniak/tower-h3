use super::glutils::*;
use super::math::*;
use gl::*;
use std::ffi::{CStr, CString};
use std::fs;

#[derive(Default, Clone, Copy)]
pub struct Shaders {
    program_id: u32,
}

impl Shaders {
    pub fn from_files(vertex_file: &str, fragment_file: &str) -> Result<Shaders, String> {
        Self::from_files_full(vertex_file, fragment_file, "")
    }

    pub fn from_files_full(
        vertex_file: &str,
        fragment_file: &str,
        geometry_file: &str,
    ) -> Result<Shaders, String> {
        let vertex_code = match fs::read_to_string(vertex_file) {
            Ok(v) => v,
            Err(e) => return Err(format!("error reading {}: {}", vertex_file, e)),
        };

        let fragment_code = match fs::read_to_string(fragment_file) {
            Ok(v) => v,
            Err(e) => return Err(format!("error reading {}: {}", fragment_file, e)),
        };

        let geometry_code = match fs::read_to_string(geometry_file) {
            Ok(v) => v,
            Err(_) => "".to_string(),
        };

        Shaders::from_str_full(
            vertex_code.as_str(),
            fragment_code.as_str(),
            geometry_code.as_str(),
        )
    }

    pub fn from_str(vertex_code: &str, fragment_code: &str) -> Result<Shaders, String> {
        Self::from_str_full(vertex_code, fragment_code, "")
    }

    pub fn from_str_full(
        vertex_code: &str,
        fragment_code: &str,
        geometry_code: &str,
    ) -> Result<Shaders, String> {
        // create vertex shader
        let vertex_shader = unsafe { gl::CreateShader(VERTEX_SHADER) };
        if vertex_shader == 0 {
            return Err("gl::createShader(VERTEX_SHADER) failed".to_string());
        }

        if let Err(e) = Self::compile(vertex_shader, vertex_code) {
            return Err(format!("vertex shader compilation error: {}", e));
        }

        // create fragment shader
        let fragment_shader = unsafe { gl::CreateShader(FRAGMENT_SHADER) };
        if fragment_shader == 0 {
            return Err("gl::createShader(FRAGMENT_SHADER) failed".to_string());
        }

        if let Err(e) = Self::compile(fragment_shader, fragment_code) {
            return Err(format!("fragment shader compilation error: {}", e));
        }

        // create program and link shaders
        let shader_program = unsafe { gl::CreateProgram() };
        unsafe { gl::AttachShader(shader_program, vertex_shader) };
        unsafe { gl::AttachShader(shader_program, fragment_shader) };

        let mut geometry_shader = 0;

        if !geometry_code.is_empty() {
            geometry_shader = unsafe { gl::CreateShader(GEOMETRY_SHADER) };
            if geometry_shader == 0 {
                return Err("gl::createShader(GEOMETRY_SHADER) failed".to_string());
            }

            if let Err(e) = Self::compile(geometry_shader, geometry_code) {
                return Err(format!("geometry shader compilation error: {}", e));
            }

            unsafe { gl::AttachShader(shader_program, geometry_shader) };
        }

        unsafe { gl::LinkProgram(shader_program) };

        let mut success = 0;
        unsafe {
            gl::GetProgramiv(shader_program, LINK_STATUS, &mut success);
        }
        if success == 0 {
            let mut v: Vec<u8> = Vec::with_capacity(1024);
            let mut log_len = 0_i32;
            unsafe {
                gl::GetProgramInfoLog(shader_program, 1024, &mut log_len, v.as_mut_ptr().cast());
                v.set_len(log_len.try_into().unwrap());
            }
            return Err(format!(
                "program link error: {}",
                String::from_utf8_lossy(&v)
            ));
        }

        // not needed anymore
        unsafe { gl::DeleteShader(vertex_shader) };
        unsafe { gl::DeleteShader(fragment_shader) };
        if geometry_shader != 0 {
            unsafe { gl::DeleteShader(geometry_shader) };
        }

        Ok(Shaders {
            program_id: shader_program,
        })
    }
    fn compile(shader_id: u32, shader_code: &str) -> Result<(), String> {
        unsafe {
            gl::ShaderSource(
                shader_id,
                1,
                &(shader_code.as_bytes().as_ptr().cast()),
                &(shader_code.len().try_into().unwrap()),
            );
        }

        unsafe { gl::CompileShader(shader_id) };

        // check if there are compilation errors
        let mut success = 0;
        unsafe {
            gl::GetShaderiv(shader_id, COMPILE_STATUS, &mut success);
        }

        if success == 0 {
            let mut v: Vec<u8> = Vec::with_capacity(1024);
            let mut log_len = 0_i32;
            unsafe {
                gl::GetShaderInfoLog(shader_id, 1024, &mut log_len, v.as_mut_ptr().cast());
                v.set_len(log_len.try_into().unwrap());
            }

            return Err(String::from_utf8_lossy(&v).to_string());
        }
        Ok(())
    }

    fn get_uniform_location(&self, name: &str) -> i32 {
        let c_name = std::ffi::CString::new(name).unwrap_or_else(|_| {
            panic!("get_uniform_location: CString::new failed for '{}'", name);
        });

        self.get_uniform_location_cstr(&c_name)
    }

    fn get_uniform_location_cstr(&self, c_name: &CStr) -> i32 {
        let location = unsafe { gl::GetUniformLocation(self.program_id, c_name.as_ptr().cast()) };
        check_gl_err();
        if location == -1 {
            let name = c_name.to_str().unwrap_or("<cstring decoding error>");
            panic!(
                "program({}): location '{}' does not correspond to an active uniform variable in program",
                self.program_id,
                name
            );
        }
        location
    }

    pub fn use_program(&self) {
        unsafe { gl::UseProgram(self.program_id) };
        check_gl_err();
    }

    pub fn set_bool(&self, name: &str, value: bool) {
        unsafe { gl::Uniform1i(self.get_uniform_location(name), i32::from(value)) };
    }

    pub fn set_bool_cstr(&self, name: &CStr, value: bool) {
        unsafe { gl::Uniform1i(self.get_uniform_location_cstr(name), i32::from(value)) };
    }

    pub fn set_i32(&self, name: &str, value: i32) {
        unsafe { gl::Uniform1i(self.get_uniform_location(name), value) };
    }

    pub fn set_i32_cstr(&self, name: &CString, value: i32) {
        unsafe { gl::Uniform1i(self.get_uniform_location_cstr(name), value) };
    }

    pub fn try_set_i32(&self, name: &str, value: i32) {
        let c_name = std::ffi::CString::new(name).unwrap();
        let location = unsafe { gl::GetUniformLocation(self.program_id, c_name.as_ptr().cast()) };

        if location == -1 {
            return;
        }

        unsafe { gl::Uniform1i(location, value) };
    }

    pub fn set_f32(&self, name: &str, value: f32) {
        let c_name = std::ffi::CString::new(name).unwrap_or_else(|_| {
            panic!("get_uniform_location: CString::new failed for '{}'", name);
        });

        self.get_uniform_location_cstr(&c_name);
        unsafe { gl::Uniform1f(self.get_uniform_location(name), value) };
    }

    pub fn set_f32_cstr(&self, name: &CStr, value: f32) {
        unsafe { gl::Uniform1f(self.get_uniform_location_cstr(name), value) };
    }

    pub fn set_vec3(&self, name: &str, v0: f32, v1: f32, v2: f32) {
        unsafe { gl::Uniform3f(self.get_uniform_location(name), v0, v1, v2) };
    }

    pub fn set_vec3_cstr(&self, name: &CStr, v0: f32, v1: f32, v2: f32) {
        unsafe { gl::Uniform3f(self.get_uniform_location_cstr(name), v0, v1, v2) };
    }

    pub fn set_vec4(&self, name: &str, v0: f32, v1: f32, v2: f32, v3: f32) {
        unsafe { gl::Uniform4f(self.get_uniform_location(name), v0, v1, v2, v3) };
    }

    pub fn set_vec4_cstr(&self, name: &CString, v0: f32, v1: f32, v2: f32, v3: f32) {
        unsafe { gl::Uniform4f(self.get_uniform_location_cstr(name), v0, v1, v2, v3) };
    }

    pub fn set_mat4fv(&self, name: &str, mat: &Mat4x4) {
        let location = self.get_uniform_location(name);
        let arr: [f32; 16] = [
            mat.c0[0], mat.c0[1], mat.c0[2], mat.c0[3], mat.c1[0], mat.c1[1], mat.c1[2], mat.c1[3],
            mat.c2[0], mat.c2[1], mat.c2[2], mat.c2[3], mat.c3[0], mat.c3[1], mat.c3[2], mat.c3[3],
        ];
        unsafe { gl::UniformMatrix4fv(location, 1, FALSE as u8, arr.as_ptr().cast()) };
    }

    // pub fn set_mat4fv_uv(&self, name: &str, mat: &Mat4x4) {
    //     let location = self.get_uniform_location(name);
    //         gl::UniformMatrix4fv(location, 1, FALSE as u8, mat.slice().as_ptr());
    // }
    //
    // pub fn try_set_mat4fv_uv(&self, gl:: &gl::ns, name: &str, mat: &Mat4) {
    //     unsafe {
    //         let c_name = std::ffi::CString::new(name).unwrap();
    //         let location = gl::GetUniformLocation(self.program_id, c_name.as_ptr().cast());
    //
    //         if location == -1 {
    //             return;
    //         }
    //
    //         gl::UniformMatrix4fv(location, 1, FALSE.0 as u8, mat.as_slice().as_ptr());
    //     }
    // }
    //
    // pub fn set_mat4fv_uv_cstr(&self, gl:: &gl::ns, name: &CString, mat: &Mat4) {
    //     let location = self.get_uniform_location_cstr(gl:: name);
    //     unsafe {
    //         gl::UniformMatrix4fv(location, 1, FALSE.0 as u8, mat.as_slice().as_ptr());
    //     }
    // }
}
