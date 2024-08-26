use std::io::{self, Cursor, Read};
extern crate libz_sys as zlib;
use std::ptr;

const GZIP_MAGIC: u32 = 0x00088B1F;

pub struct BinaryDataReader {
    cursor: Cursor<Vec<u8>>,
}

impl BinaryDataReader {
    pub fn new(data: Vec<u8>) -> Self {
        BinaryDataReader {
            cursor: Cursor::new(data),
        }
    }

    pub fn new_possibly_gzip(data: Vec<u8>) -> Result<Self, io::Error> {
        if data.len() >= 4 {
            let magic = (data[3] as u32) << 24
                | (data[2] as u32) << 16
                | (data[1] as u32) << 8
                | data[0] as u32;
            if magic == GZIP_MAGIC {
                // this is gzip archive
                return Ok(Self::new(decompress(&data)?));
            }
        }
        Ok(Self::new(data))
    }

    // Read a single u8
    pub fn read_u8(&mut self) -> io::Result<u8> {
        let mut buffer = [0u8; 1];
        self.cursor.read_exact(&mut buffer)?;
        Ok(buffer[0])
    }

    pub fn read_i8(&mut self) -> io::Result<i8> {
        Ok(self.read_u8()? as i8)
    }

    pub fn read_bool(&mut self) -> io::Result<bool> {
        Ok(self.read_u8()? != 0)
    }

    // Read a single u16 assuming little-endian byte order
    pub fn read_u16_le(&mut self) -> io::Result<u16> {
        let mut buffer = [0u8; 2];
        self.cursor.read_exact(&mut buffer)?;
        Ok((buffer[1] as u16) << 8 | buffer[0] as u16)
    }

    // Read a single u32 assuming little-endian byte order
    pub fn read_u32_le(&mut self) -> io::Result<u32> {
        let mut buffer = [0u8; 4];
        self.cursor.read_exact(&mut buffer)?;
        Ok((buffer[3] as u32) << 24
            | (buffer[2] as u32) << 16
            | (buffer[1] as u32) << 8
            | buffer[0] as u32)
    }

    pub fn read_i32_le(&mut self) -> io::Result<i32> {
        Ok(self.read_u32_le()? as i32)
    }

    // Read a single f32 assuming little-endian byte order
    // pub fn read_f32_le(&mut self) -> io::Result<f32> {
    //     let value = self.read_u32_le()?;
    //     Ok(f32::from_bits(value))
    // }

    // Read a single u64 assuming little-endian byte order
    // pub fn read_u64_le(&mut self) -> io::Result<u64> {
    //     let mut buffer = [0u8; 8];
    //     self.cursor.read_exact(&mut buffer)?;
    //     Ok((buffer[7] as u64) << 56
    //         | (buffer[6] as u64) << 48
    //         | (buffer[5] as u64) << 40
    //         | (buffer[4] as u64) << 32
    //         | (buffer[3] as u64) << 24
    //         | (buffer[2] as u64) << 16
    //         | (buffer[1] as u64) << 8
    //         | buffer[0] as u64)
    // }

    pub fn read_byte_array(&mut self, len: usize) -> io::Result<Vec<u8>> {
        let mut buffer = vec![0u8; len];
        self.cursor.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    pub fn read_string_le(&mut self) -> io::Result<String> {
        let str_len = self.read_u32_le()?;
        let str_buf = self.read_byte_array(str_len as usize)?;
        // TODO: proper text encoding needed. For H3 maps, apparently this is not utf8
        // maybe ISO-8859-2 (Latin-2) or Windows-1250?
        Ok(String::from_utf8_lossy(&str_buf).into_owned())
        // match String::from_utf8(str_buf.clone()) {
        //     Ok(s) => Ok(s),
        //     Err(e) => Err(io::Error::new(
        //         io::ErrorKind::Other,
        //         format!("converting {str_buf:?} to utf8 string error: {e}"),
        //     )),
        // }
    }

    pub fn skip_n(&mut self, n: usize) {
        self.cursor.set_position(self.cursor.position() + n as u64)
    }

    pub fn dump_hex(&mut self, before: usize, after: usize) -> io::Result<()> {
        let original_position = self.cursor.position();
        // Calculate start and end positions
        let start = original_position.saturating_sub(before as u64);
        let end = (original_position + after as u64).min(self.cursor.get_ref().len() as u64);
        // Seek to the start position
        self.cursor.set_position(start);

        // Read the necessary bytes
        let mut buffer = vec![0; (end - start) as usize];
        self.cursor.read_exact(&mut buffer)?;

        // Print out the hex dump
        let mut start_indx = 0;
        for (i, byte) in buffer.iter().enumerate() {
            if i % 16 == 0 {
                print!("\n{:08x}: ", start + i as u64); // Print the address offset
                start_indx = i;
            }
            print!("{:02x} ", byte); // Print the byte in hex format

            // For better readability, print the ASCII representation at the end of each line
            if i % 16 == 15 || i == buffer.len() - 1 {
                let remaining = 15 - (i % 16);
                for _ in 0..remaining {
                    print!("   "); // Align the ASCII representation
                }
                print!("| ");
                for j in start_indx..=i {
                    let c = buffer[j] as char;
                    if c.is_ascii_graphic() || c == ' ' {
                        // if ['\n', '\t', '\r'].contains(&c) {
                            // print!(".");
                        // } else {
                            print!("{}", c);
                        // }
                    } else {
                        print!(".");
                    }
                }
                print!(" |");
            }
        }
        println!("");

        // restore original position
        self.cursor.set_position(original_position);
        Ok(())
    }
}

fn decompress(data: &[u8]) -> Result<Vec<u8>, io::Error> {
    unsafe {
        // Initialize z_stream
        let mut stream = zlib::z_stream {
            next_in: data.as_ptr() as *mut u8, // Input data pointer
            avail_in: data.len() as u32,       // Available input data size
            next_out: ptr::null_mut(),         // Will be set later for output
            avail_out: 0,                      // Available output size
            // ..zlib::z_stream::default()        // Fill remaining fields with default values
            adler: 0,
            data_type: 0,
            msg: std::ptr::null_mut(),
            opaque: std::ptr::null_mut(),
            reserved: 0,
            state: std::ptr::null_mut(),
            total_in: 0,
            total_out: 0,
            // zalloc: std::mem::uninitialized(),
            #[allow(invalid_value)]
            zalloc: std::mem::MaybeUninit::zeroed().assume_init(),
            #[allow(invalid_value)]
            zfree: std::mem::MaybeUninit::zeroed().assume_init(),
        };

        // Allocate an initial buffer for the output
        let mut output: Vec<u8> = Vec::with_capacity(1024);

        let mut wbits = 15;
        wbits += 16; // if zlib
        let ret = zlib::inflateInit2_(
            &mut stream,
            wbits,
            libz_sys::zlibVersion(),
            std::mem::size_of::<zlib::z_stream>() as i32,
        );

        if ret != zlib::Z_OK {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to initialize zlib",
            ));
        }

        // Decompression loop
        loop {
            // Extend the output buffer when needed
            output.reserve(1024);
            let output_ptr = output.as_mut_ptr().add(output.len());
            let available_output = output.capacity() - output.len();

            // Set the output buffer in the z_stream
            stream.next_out = output_ptr;
            stream.avail_out = available_output as u32;

            // Perform the inflation (decompression)
            let ret = zlib::inflate(&mut stream, zlib::Z_NO_FLUSH);

            // Update the length of the output based on how much data was written
            let written = available_output - stream.avail_out as usize;
            output.set_len(output.len() + written);

            // Check the return value of `inflate` to see if we're done
            match ret {
                zlib::Z_OK | zlib::Z_BUF_ERROR => continue, // Keep decompressing
                zlib::Z_STREAM_END => break,                // Done
                zlib::Z_DATA_ERROR => {
                    // If we hit an error, clean up and return an error
                    zlib::inflateEnd(&mut stream);
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Decompression failed: Z_DATA_ERROR",
                    ));
                }

                _ => {
                    // If we hit an error, clean up and return an error
                    zlib::inflateEnd(&mut stream);
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Decompression failed: {ret}").as_str(),
                    ));
                }
            }
        }

        // Clean up the decompression state
        zlib::inflateEnd(&mut stream);

        Ok(output)
    }
}
