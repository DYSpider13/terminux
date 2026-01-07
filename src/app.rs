use crate::storage::Database;
use crate::window::TerminuxWindow;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{gio, glib};
use libadwaita as adw;
use libadwaita::subclass::prelude::*;
use std::cell::OnceCell;
use std::rc::Rc;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct TerminuxApplication {
        pub database: OnceCell<Rc<Database>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TerminuxApplication {
        const NAME: &'static str = "TerminuxApplication";
        type Type = super::TerminuxApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for TerminuxApplication {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_actions();
            obj.setup_accels();
        }
    }

    impl ApplicationImpl for TerminuxApplication {
        fn activate(&self) {
            log::debug!("Application activated");
            let app = self.obj();

            // Get or create the main window
            let window = if let Some(window) = app.active_window() {
                window.downcast::<TerminuxWindow>().unwrap()
            } else {
                let window = TerminuxWindow::new(&app);
                window.upcast()
            };

            window.present();
        }

        fn startup(&self) {
            self.parent_startup();
            log::debug!("Application startup");

            // Initialize database
            match Database::new() {
                Ok(db) => {
                    log::info!("Database initialized successfully");
                    let _ = self.database.set(Rc::new(db));
                }
                Err(e) => {
                    log::error!("Failed to initialize database: {}", e);
                }
            }

            // Load CSS styles
            let css_provider = gtk4::CssProvider::new();
            css_provider.load_from_string(include_str!("style.css"));

            gtk4::style_context_add_provider_for_display(
                &gtk4::gdk::Display::default().expect("Could not get default display"),
                &css_provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    impl GtkApplicationImpl for TerminuxApplication {}
    impl AdwApplicationImpl for TerminuxApplication {}
}

glib::wrapper! {
    pub struct TerminuxApplication(ObjectSubclass<imp::TerminuxApplication>)
        @extends gio::Application, gtk4::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl TerminuxApplication {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("application-id", "org.terminux.Terminux")
            .property("flags", gio::ApplicationFlags::FLAGS_NONE)
            .build()
    }

    pub fn database(&self) -> Option<Rc<Database>> {
        self.imp().database.get().cloned()
    }

    fn setup_actions(&self) {
        // Quit action
        let action_quit = gio::ActionEntry::builder("quit")
            .activate(|app: &Self, _, _| {
                app.quit();
            })
            .build();

        // About action
        let action_about = gio::ActionEntry::builder("about")
            .activate(|app: &Self, _, _| {
                app.show_about_dialog();
            })
            .build();

        // New session action
        let action_new_session = gio::ActionEntry::builder("new-session")
            .activate(|app: &Self, _, _| {
                if let Some(window) = app.active_window() {
                    if let Some(win) = window.downcast_ref::<TerminuxWindow>() {
                        win.show_new_session_dialog();
                    }
                }
            })
            .build();

        // New tab action
        let action_new_tab = gio::ActionEntry::builder("new-tab")
            .activate(|app: &Self, _, _| {
                if let Some(window) = app.active_window() {
                    if let Some(win) = window.downcast_ref::<TerminuxWindow>() {
                        win.add_local_terminal_tab();
                    }
                }
            })
            .build();

        self.add_action_entries([action_quit, action_about, action_new_session, action_new_tab]);
    }

    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<Control>q"]);
        self.set_accels_for_action("app.new-session", &["<Control><Shift>n"]);
        self.set_accels_for_action("app.new-tab", &["<Control>t"]);
        self.set_accels_for_action("win.close-tab", &["<Control>w"]);
    }

    fn show_about_dialog(&self) {
        let dialog = adw::AboutWindow::builder()
            .application_name("Terminux")
            .application_icon("org.terminux.Terminux")
            .developer_name("Younes Khadraoui")
            .version(env!("CARGO_PKG_VERSION"))
            .website("https://github.com/DYspider13/terminux")
            .issue_url("https://github.com/DYspider13/terminux/issues")
            .license_type(gtk4::License::Gpl30)
            .comments("A Linux SSH terminal manager inspired by MobaXterm")
            .developers(["Younes Khadraoui <https://github.com/DYspider13>"])
            .copyright("© 2025 Younes Khadraoui")
            .modal(true)
            .build();

        if let Some(window) = self.active_window() {
            dialog.set_transient_for(Some(&window));
        }
        dialog.present();
    }
}

impl Default for TerminuxApplication {
    fn default() -> Self {
        Self::new()
    }
}
