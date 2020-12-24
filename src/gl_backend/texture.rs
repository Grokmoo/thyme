pub struct GLTexture {
    texture_handle: u32,
    data: Vec<u8>,
}

impl GLTexture {
    pub fn new(
        image_data: &[u8],
        dimensions: (u32, u32),
        filter: u32,
        wrap: u32,
        format: u32,
        internal_format: u32,
    ) -> GLTexture {
        let mut texture = GLTexture {
            texture_handle: 0,
            data: image_data.to_vec(),
        };
        
        unsafe {
            gl::GenTextures(1, &mut texture.texture_handle);
            gl::BindTexture(gl::TEXTURE_2D, texture.texture_handle);

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, wrap as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, wrap as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_R, wrap as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, filter as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, filter as _);

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_BASE_LEVEL, 0);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAX_LEVEL, 0);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            
            gl::TexStorage2D(
                gl::TEXTURE_2D,
                1,
                internal_format as _,
                dimensions.0 as _,
                dimensions.1 as _,
            );
            
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                dimensions.0 as _,
                dimensions.1 as _,
                format,
                gl::UNSIGNED_BYTE,
                texture.data.as_ptr() as _,
            );

            gl::GenerateMipmap(gl::TEXTURE_2D);
        }

        texture
    }

    pub fn bind(&self, idx: i32) {
        let bind_location = match idx {
            0 => gl::TEXTURE0,
            1 => gl::TEXTURE1,
            2 => gl::TEXTURE2,
            3 => gl::TEXTURE3,
            4 => gl::TEXTURE4,
            5 => gl::TEXTURE5,
            6 => gl::TEXTURE6,
            _ => panic!("invalid idx"),
        };

        unsafe {
            gl::ActiveTexture(bind_location);
            gl::BindTexture(gl::TEXTURE_2D, self.texture_handle);
        }
    }
}

impl Drop for GLTexture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.texture_handle);
        }
    }
}
