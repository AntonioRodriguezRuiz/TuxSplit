// mod api;
mod config;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::time::Duration;

// use api::api::{create, reset, split, start};

use livesplit_core::{Run, Segment, Timer, TimerPhase};
use tracing_subscriber;

use adw::prelude::*;
use adw::{Application, ApplicationWindow};
use glib::ControlFlow::Continue;
use gtk4::prelude::*;
use gtk4::{gdk::Display, Box as GtkBox, Builder, Button, CssProvider, Label, Orientation};

fn main() {
    adw::init().expect("Failed to initialize libadwaita");

    let app = Application::builder()
        .application_id("org.UnixSplit.unixplit-beta")
        .build();

    let app_state = Rc::new(RefCell::new(UnixSplit::new()));

    app.connect_activate(move |app| {
        app_state.borrow_mut().build_ui(app);
    });
    app.run();
}

#[derive(Clone, Debug)]
pub struct UnixSplit {
    pub timer: Rc<RefCell<Timer>>,
}

impl UnixSplit {
    pub fn new() -> Self {
        let mut run = Run::new();
        run.push_segment(Segment::new(""));

        let timer = Timer::new(run).expect("");

        Self {
            timer: Rc::new(RefCell::new(timer)),
        }
    }

    fn load_css() {
        let provider = CssProvider::new();
        provider.load_from_path("data/css/livesplit-gtk.css");

        let display = Display::default().expect("Could not connect to a display");
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    fn build_ui(&mut self, app: &Application) {
        Self::load_css();

        let builder = Builder::from_file("data/ui/livesplit-gtk.ui");

        let clamp: adw::Clamp = builder
            .object("livesplit-gtk")
            .expect("Couldn't get main widget");

        let window = ApplicationWindow::builder()
            .application(app)
            .title("LiveSplit GTK")
            .content(&clamp)
            .default_width(400)
            .default_height(600)
            .build();

        window.present();
    }
}
