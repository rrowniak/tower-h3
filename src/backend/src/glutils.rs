use gl::{types::*, *};

pub fn check_gl_err() {
    let err = unsafe { gl::GetError() };
    if err == gl::NO_ERROR {
        return;
    }
    panic!("error: {:?}", err);
}

pub fn print_opengl_info() {
    let mut mtu: i32 = 0;
    unsafe { gl::GetIntegerv(MAX_TEXTURE_IMAGE_UNITS, &mut mtu) };
    println!("MAX_TEXTURE_IMAGE_UNITS = {}", mtu);

    unsafe { gl::GetIntegerv(MAX_COMBINED_TEXTURE_IMAGE_UNITS, &mut mtu) };
    println!("MAX_COMBINED_TEXTURE_IMAGE_UNITS = {}", mtu);
}

pub fn gl_buffer_data_arr_stat<T: Sized>(buffer: &[T]) {
    unsafe {
        gl::BufferData(
            ARRAY_BUFFER,
            std::mem::size_of_val(buffer) as isize,
            buffer.as_ptr().cast(),
            STATIC_DRAW,
        )
    };
}

pub fn gl_buffer_data_element_stat<T: Sized>(buffer: &[T]) {
    unsafe {
        gl::BufferData(
            ELEMENT_ARRAY_BUFFER,
            std::mem::size_of_val(buffer) as isize,
            buffer.as_ptr().cast(),
            STATIC_DRAW,
        )
    };
}

pub fn gl_vertex_attrib_ptr_enab(index: u32, size: u32, stride: u32, pointer: usize) {
    unsafe {
        gl::VertexAttribPointer(
            index,
            size as i32,
            FLOAT,
            1, //FALSE,
            (stride as usize * std::mem::size_of::<f32>()) as i32,
            (pointer * std::mem::size_of::<f32>()) as *const _,
        )
    };
    unsafe { gl::EnableVertexAttribArray(index) };
}

pub fn load_texture(filename: &str) -> Result<u32, String> {
    let params = [
        (TEXTURE_2D, TEXTURE_WRAP_S, REPEAT),
        (TEXTURE_2D, TEXTURE_WRAP_T, REPEAT),
        (TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR),
        (TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR),
    ];
    load_texture_params(filename, &params)
}

pub fn load_texture_params(
    filename: &str,
    params: &[(GLenum, GLenum, GLenum)],
) -> Result<u32, String> {
    let mut texture = 0;
    unsafe { gl::GenTextures(1, &mut texture) };
    unsafe { gl::BindTexture(TEXTURE_2D, texture) };

    for (t, n, p) in params {
        unsafe {
            gl::TexParameteri(
                *t, *n, *p as i32,
                // 0x2901,
            )
        };
    }
    unsafe {
        stb_image::stb_image::stbi_set_flip_vertically_on_load(1);
    }
    let img = match stb_image::image::load(filename) {
        stb_image::image::LoadResult::ImageF32(_) => {
            return Err("32-bit images not supported here".to_string());
            // img
        }
        stb_image::image::LoadResult::ImageU8(img) => img,
        stb_image::image::LoadResult::Error(e) => {
            return Err(format!("loading image {} error: {}", filename, e))
        }
    };

    let mut format = RGB;
    if img.depth == 1 {
        format = RED;
    } else if img.depth == 3 {
        format = RGB;
    } else if img.depth == 4 {
        format = RGBA;
    }

    unsafe {
        gl::TexImage2D(
            TEXTURE_2D,
            0,
            RGBA as i32,
            img.width as i32,
            img.height as i32,
            0,
            format,
            UNSIGNED_BYTE,
            img.data.as_ptr().cast(),
        )
    };
    check_gl_err();
    unsafe { gl::GenerateMipmap(TEXTURE_2D) };

    Ok(texture)
}

/// Creates a cube map texture
///
/// # Arguments
///
/// * `gl` - OpenGl context
/// * `filenames` - An array of image filenames according to the following
///   orientation: [right, left, top, bottom, back, front]
///
pub fn load_cube_map_texture(filenames: &[&str]) -> Result<u32, String> {
    let params = [
        (TEXTURE_CUBE_MAP, TEXTURE_WRAP_S, CLAMP_TO_EDGE),
        (TEXTURE_CUBE_MAP, TEXTURE_WRAP_T, CLAMP_TO_EDGE),
        (TEXTURE_CUBE_MAP, TEXTURE_WRAP_R, CLAMP_TO_EDGE),
        (TEXTURE_CUBE_MAP, TEXTURE_MIN_FILTER, LINEAR),
        (TEXTURE_CUBE_MAP, TEXTURE_MAG_FILTER, LINEAR),
    ];
    load_cube_map_texture_params(filenames, &params)
}

pub fn load_cube_map_texture_params(
    filenames: &[&str],
    params: &[(GLenum, GLenum, GLenum)],
) -> Result<u32, String> {
    let mut texture = 0;
    unsafe { gl::GenTextures(1, &mut texture) };
    unsafe { gl::BindTexture(TEXTURE_CUBE_MAP, texture) };
    check_gl_err();

    for (t, n, p) in params {
        unsafe {
            gl::TexParameteri(
                *t, *n, *p as i32,
                // 0x2901,
            )
        };
        check_gl_err();
    }
    unsafe {
        // stb_image::stb_image::bindgen::stbi_set_flip_vertically_on_load(1);
        stb_image::stb_image::stbi_set_flip_vertically_on_load(0);
    }
    let targets = [
        TEXTURE_CUBE_MAP_POSITIVE_X, // right
        TEXTURE_CUBE_MAP_NEGATIVE_X, // left
        TEXTURE_CUBE_MAP_POSITIVE_Y, // top
        TEXTURE_CUBE_MAP_NEGATIVE_Y, // bottom
        TEXTURE_CUBE_MAP_POSITIVE_Z, // back
        TEXTURE_CUBE_MAP_NEGATIVE_Z, // front
    ];
    for (target, filename) in targets.iter().zip(filenames) {
        let img = match stb_image::image::load(filename) {
            stb_image::image::LoadResult::ImageF32(_) => {
                return Err("32-bit images not supported here".to_string());
                // img
            }
            stb_image::image::LoadResult::ImageU8(img) => img,
            stb_image::image::LoadResult::Error(e) => {
                return Err(format!("loading image {} error: {}", filename, e))
            }
        };

        let mut format = RGB;
        if img.depth == 1 {
            format = RED;
        } else if img.depth == 3 {
            format = RGB;
        } else if img.depth == 4 {
            format = RGBA;
        }

        unsafe {
            gl::TexImage2D(
                *target,
                0,
                RGBA as i32,
                img.width as i32,
                img.height as i32,
                0,
                format,
                UNSIGNED_BYTE,
                img.data.as_ptr().cast(),
            )
        };
        check_gl_err();
    }

    Ok(texture)
}
