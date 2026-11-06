use dsh_libadwaita::ui::controller::SessionController;
use dsh_libadwaita::ui::desktop::configure_system_style;
use dsh_libadwaita::ui::settings::show_settings_dialog;
use dsh_libadwaita::ui::window::MainWindow;
use dsh_libadwaita::APP_ID;
use gtk4::gio::prelude::*;
use gtk4::prelude::*;
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;

fn main() -> adw::glib::ExitCode {
    configure_system_style();

    let application = adw::Application::builder().application_id(APP_ID).build();

    application.connect_startup(|_| configure_system_style());

    application.set_accels_for_action("app.settings", &["<Primary>comma"]);
    application.set_accels_for_action("win.toggle-sidebar", &["<Primary>b"]);
    application.set_accels_for_action("win.new-session", &["<Primary>n"]);
    application.set_accels_for_action("window.close", &["<Primary>w"]);
    application.set_accels_for_action(
        "win.show-help-overlay",
        &["<Primary>question", "<Primary><Shift>slash"],
    );

    if application.lookup_action("activate").is_none() {
        let app_weak_activate = application.downgrade();
        let activate_action = gtk4::gio::SimpleAction::new("activate", None);
        activate_action.connect_activate(move |_, _| {
            if let Some(app) = app_weak_activate.upgrade() {
                ApplicationExt::activate(&app);
            }
        });
        application.add_action(&activate_action);
    }

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

    let main_window_store: Rc<RefCell<Option<MainWindow>>> = Rc::new(RefCell::new(None));

    application.connect_activate(move |application| {
        let existing_window = {
            main_window_store
                .borrow()
                .as_ref()
                .map(|w| w.window().clone())
        };

        if let Some(window) = existing_window {
            window.set_visible(true);
            window.present();
            return;
        }

        let main_window = MainWindow::new(application);
        let controller = SessionController::new(
            &main_window,
            main_window.sidebar().clone(),
            main_window.conversation_view_rc().clone(),
        );
        main_window.set_controller(controller);
        main_window.present();
        *main_window_store.borrow_mut() = Some(main_window);
    });

    application.run()
}
