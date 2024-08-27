use backend;

fn main() {
    let mut system = match backend::system::System::new(800, 600) {
        Ok(s) => s,
        Err(msg) => panic!("Game initialization failure: {msg}"),
    };
    loop {
    if !system.process_io_events() {
            break;
        }
        // game logic
        system.clear_screen(0.6, 0.0, 0.7);
        // game gfx render logic
        system.draw_to_screen();
    }
}
