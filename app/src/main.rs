use dsh_libadwaita::{APP_ID, APP_NAME};
use libadwaita as adw;

use adw::prelude::*;

fn main() -> adw::glib::ExitCode {
    let application = adw::Application::builder().application_id(APP_ID).build();

    application.connect_activate(|application| {
        let window = adw::ApplicationWindow::builder()
            .application(application)
            .title(APP_NAME)
            .build();

        window.present();
    });

    application.run()
}
