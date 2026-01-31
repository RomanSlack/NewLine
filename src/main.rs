mod application;
mod window;
mod document;
mod project_sidebar;
mod task;
mod sync;

use application::NextLineApp;

fn main() -> glib::ExitCode {
    // Initialize libadwaita
    adw::init().expect("Failed to initialize libadwaita");

    let app = NextLineApp::new();
    app.run()
}
