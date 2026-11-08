use dsh_libadwaita::ui::controller::SessionController;
use dsh_libadwaita::ui::desktop::configure_system_style;
use dsh_libadwaita::ui::window::MainWindow;
use dsh_libadwaita::APP_ID;
use gtk4::gio::prelude::*;
use gtk4::prelude::*;
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;

extern "C" fn on_terminate(_signal: libc::c_int) {
    std::process::exit(0);
}


fn main() -> adw::glib::ExitCode {
    let application = adw::Application::builder().application_id(APP_ID).build();

    application.connect_startup(|_| configure_system_style());
    unsafe {
        libc::signal(libc::SIGTERM, on_terminate as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, on_terminate as *const () as libc::sighandler_t);
    }

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

    let main_window_store: Rc<RefCell<Option<MainWindow>>> = Rc::new(RefCell::new(None));
    let pending_open_session: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let pending_for_action = pending_open_session.clone();
    let app_weak_open = application.downgrade();
    let open_session_action =
        gtk4::gio::SimpleAction::new("open-session", Some(&gtk4::glib::VariantTy::STRING));
    open_session_action.connect_activate(move |_, param| {
        if let Some(id) = param.and_then(|value| value.get::<String>()) {
            *pending_for_action.borrow_mut() = Some(id);
            if let Some(app) = app_weak_open.upgrade() {
                ApplicationExt::activate(&app);
            }
        }
    });
    application.add_action(&open_session_action);
    let main_window_store_for_settings = main_window_store.clone();
    let settings_action = gtk4::gio::SimpleAction::new("settings", None);
    settings_action.connect_activate(move |_, _| {
        if let Some(main_window) = main_window_store_for_settings.borrow().as_ref() {
            if let Some(controller) = main_window.controller() {
                controller.open_settings();
            }
        }
    });
    application.add_action(&settings_action);

    application.connect_activate(move |application| {
        let pending_for_activate = pending_open_session.clone();
        let existing_window = {
            main_window_store
                .borrow()
                .as_ref()
                .map(|w| w.window().clone())
        };

        if let Some(window) = existing_window {
            window.set_visible(true);
            window.present();
            if let Some(session_id) = pending_for_activate.borrow_mut().take() {
                if let Some(main_window) = main_window_store.borrow().as_ref() {
                    if let Some(controller) = main_window.controller() {
                        controller.follow_session(&session_id);
                    }
                }
            }
            return;
        }

        let main_window = MainWindow::new(application);
        let controller = SessionController::new(
            &main_window,
            main_window.sidebar().clone(),
            main_window.conversation_view_rc().clone(),
            main_window.right_dock().clone(),
        );
        main_window.set_controller(controller.clone());
        main_window.present();
        controller.bootstrap_supervisor();
        *main_window_store.borrow_mut() = Some(main_window);
        if let Some(session_id) = pending_for_activate.borrow_mut().take() {
            if let Some(main_window) = main_window_store.borrow().as_ref() {
                if let Some(controller) = main_window.controller() {
                    controller.follow_session(&session_id);
                }
            }
        }
    });

    application.run()
}
