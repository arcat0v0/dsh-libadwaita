use dsh_libadwaita::ui::controller::SessionController;
use dsh_libadwaita::ui::settings::show_settings_dialog;
use dsh_libadwaita::ui::window::MainWindow;
use dsh_libadwaita::APP_ID;
use gtk4::prelude::*;
use libadwaita as adw;

fn main() -> adw::glib::ExitCode {
    let application = adw::Application::builder().application_id(APP_ID).build();

    application.set_accels_for_action("app.settings", &["<Primary>comma"]);
    application.set_accels_for_action("win.toggle-sidebar", &["<Primary>b"]);
    application.set_accels_for_action("win.new-session", &["<Primary>n"]);

    let app_weak = application.downgrade();
    let settings_action = gtk4::gio::SimpleAction::new("settings", None);
    settings_action.connect_activate(move |_, _| {
        if let Some(app) = app_weak.upgrade() {
            if let Some(active_window) = app.active_window() {
                show_settings_dialog(&active_window, None, None);
            }
        }
    });
    application.add_action(&settings_action);

    application.connect_activate(|application| {
        let main_window = MainWindow::new(application);
        let controller = SessionController::new(
            &main_window,
            main_window.sidebar().clone(),
            main_window.conversation_view_rc().clone(),
        );
        main_window.set_controller(controller);
        main_window.present();
    });

    application.run()
}
