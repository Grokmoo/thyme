use gl;
pub struct Program {
    program_handle: u32,
}

impl Program {
    pub fn new(vertex_shader: &str, geom_shader: &str, fragment_shader: &str) -> Program {
        let program_handle = unsafe { gl::CreateProgram() };

        let vertex_shader = unsafe { create_shader(gl::VERTEX_SHADER, vertex_shader) };
        let geom_shader = unsafe { create_shader(gl::GEOMETRY_SHADER, geom_shader) };
        let fragment_shader = unsafe { create_shader(gl::FRAGMENT_SHADER, fragment_shader) };

        unsafe {
            gl::AttachShader(program_handle, vertex_shader);
            gl::AttachShader(program_handle, geom_shader);
            gl::AttachShader(program_handle, fragment_shader);

            gl::LinkProgram(program_handle);

            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(geom_shader);
            gl::DeleteShader(fragment_shader);
        }

        Program { program_handle }
    }

    pub fn uniform_matrix4fv(
        &self,
        uniform_location: i32,
        transposed: bool,
        matrix: &[[f32; 4]; 4],
    ) {
        let transposed = if transposed { 1 } else { 0 };
        unsafe {
            gl::UniformMatrix4fv(uniform_location, 1, transposed, matrix.as_ptr() as _);
        }
    }

    pub fn uniform1i(&self, uniform_location: i32, value: i32) {
        unsafe {
            gl::Uniform1i(uniform_location, value);
        }
    }

    pub fn get_uniform_location(&self, name: &str) -> i32 {
        let name = std::ffi::CString::new(name).unwrap();
        unsafe { gl::GetUniformLocation(self.program_handle, name.as_ptr() as _) }
    }

    pub fn use_program(&self) {
        unsafe {
            gl::UseProgram(self.program_handle);
        }
    }
}

unsafe fn create_shader(shader_type: u32, src: &str) -> u32 {
    let shader_str = std::ffi::CString::new(src).unwrap();
    
    let gl_handle = gl::CreateShader(shader_type);
    gl::ShaderSource(gl_handle, 1, &shader_str.as_ptr() as _, std::ptr::null());
    gl::CompileShader(gl_handle);

    let mut success = gl::FALSE as gl::types::GLint;
    gl::GetShaderiv(gl_handle, gl::COMPILE_STATUS, &mut success);
    if success != gl::TRUE as gl::types::GLint {
        let info_log = [0u8; 513];
        let mut error_size = 0i32;
        gl::GetShaderInfoLog(gl_handle, 512, &mut error_size, info_log.as_ptr() as _);
        let info_log = std::str::from_utf8(&info_log[..error_size as usize]);
        panic!(
            "Error compile failed with error: {:?} for: {:?}",
            info_log, gl_handle
        );
    }

    gl_handle
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.program_handle);
        }
    }
}
