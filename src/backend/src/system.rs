use gl;
use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;

pub enum MouseButtonId {
    // x, y
    Left(i32, i32),   // button: 3
    Right(i32, i32),  // button: 1
    Middle(i32, i32), // button: 2
    Other(i32, i32),
}
pub enum IoEvents {
    Quit,
    // key code
    KeyDown(i32),
    // key code
    KeyUp(i32),
    ControllerAxisMotion(i32),
    ControllerButtonDown(i32),
    ControllerButtonUp(i32),
    // x, y, xrel, yrel
    MouseMotion(i32, i32, i32, i32),
    MouseButtonUp(MouseButtonId),
    MouseButtonDown(MouseButtonId),
    // dx, dy (usually -1 or 1 based on direction)
    MouseWheel(i32, i32),
}

pub struct System {
    pub w: usize,
    pub h: usize,
    pub sdl_context: sdl2::Sdl,
    pub video_subsystem: sdl2::VideoSubsystem,
    pub window: sdl2::video::Window,
    pub gl_ctx: sdl2::video::GLContext,
    pub events: Vec<IoEvents>,
}

impl System {
    pub fn new(w: usize, h: usize) -> Result<System, String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;

        let gl_attr = video_subsystem.gl_attr();
        gl_attr.set_context_profile(GLProfile::Core);
        gl_attr.set_context_version(3, 3);

        let window = match video_subsystem
            .window("Window", w as u32, h as u32)
            .opengl()
            .build()
        {
            Ok(w) => w,
            Err(e) => return Err(format!("Error while building OpenGL window: {e}")),
        };

        let gl_ctx = window.gl_create_context()?;
        gl::load_with(|name| video_subsystem.gl_get_proc_address(name) as *const _);

        debug_assert_eq!(gl_attr.context_profile(), GLProfile::Core);
        debug_assert_eq!(gl_attr.context_version(), (3, 3));

        Ok(System {
            w,
            h,
            sdl_context,
            window,
            video_subsystem,
            gl_ctx,
            events: Vec::new(),
        })
    }

    pub fn process_io_events(&mut self) -> bool {
        self.events.clear();
        let mut event_pump = self.sdl_context.event_pump().unwrap();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return false,
                _ => {}
            }
        }
        true
    }

    pub fn draw_to_screen(&mut self) {
        self.window.gl_swap_window();
        ::std::thread::sleep(::std::time::Duration::new(0, 1_000_000_000u32 / 60));
    }

    pub fn clear_screen(&mut self, r: f32, g: f32, b: f32) {
        unsafe {
            gl::ClearColor(r, g, b, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
    }
}
