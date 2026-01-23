use adw::prelude::*;
use gtk::glib;

use crate::window::MainWindow;

pub struct NextLineApp {
    app: adw::Application,
}

impl NextLineApp {
    pub fn new() -> Self {
        let app = adw::Application::builder()
            .application_id("com.github.nextline")
            .build();

        app.connect_activate(|app| {
            let win = MainWindow::new(app);
            win.present();
            // Keep the MainWindow alive for the duration of the app
            // by leaking the Rc (the app will exit when the window closes)
            std::mem::forget(win);
        });

        // Apply custom CSS
        app.connect_startup(|_| {
            let provider = gtk::CssProvider::new();
            provider.load_from_data(include_str!("style.css"));

            gtk::style_context_add_provider_for_display(
                &gtk::gdk::Display::default().expect("Could not get default display"),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        });

        Self { app }
    }

    pub fn run(&self) -> glib::ExitCode {
        self.app.run()
    }
}
