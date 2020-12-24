use super::GLVertex;
use gl;
use memoffset::offset_of;

pub struct VAO {
    vao_handle: u32,
    vbo_handle: u32,
}

impl VAO {
    pub(crate) fn new(vertices: &[GLVertex]) -> VAO {
        let mut vao_handle = 0;
        let mut vbo_handle = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut vao_handle);
           
            gl::GenBuffers(1, &mut vbo_handle);
            // bind the Vertex Array Object first, then bind and set vertex buffer(s), and then configure vertex attributes(s).
            gl::BindVertexArray(vao_handle);

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_handle);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * std::mem::size_of::<GLVertex>()) as _,
                vertices.as_ptr() as _,
                gl::STATIC_DRAW,
            );

            for idx in 0..=6 {
                gl::EnableVertexAttribArray(idx);    
            }
            
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<GLVertex>() as _,
                offset_of!(GLVertex, position) as _,
            );

            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<GLVertex>() as _,
                offset_of!(GLVertex, size) as _,
            );

            gl::VertexAttribPointer(
                2,
                2,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<GLVertex>() as _,
                offset_of!(GLVertex, tex0) as _,
            );

            gl::VertexAttribPointer(
                3,
                2,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<GLVertex>() as _,
                offset_of!(GLVertex, tex1) as _,
            );

            gl::VertexAttribPointer(
                4,
                3,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<GLVertex>() as _,
                offset_of!(GLVertex, color) as _,
            );

            gl::VertexAttribPointer(
                5,
                2,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<GLVertex>() as _,
                offset_of!(GLVertex, clip_pos) as _,
            );

            gl::VertexAttribPointer(
                6,
                2,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<GLVertex>() as _,
                offset_of!(GLVertex, clip_size) as _,
            );
            

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }

        VAO {
            vao_handle,
            vbo_handle,
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindVertexArray(self.vao_handle);
        }
    }
}

impl Drop for VAO {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.vbo_handle);
            gl::DeleteVertexArrays(1, &self.vao_handle);
        }
    }
}
