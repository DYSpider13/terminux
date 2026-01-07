use crate::storage::{Database, Session};
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::glib;
use std::cell::RefCell;
use std::rc::Rc;

mod imp {
    use super::*;

    pub struct SessionList {
        pub list_box: gtk4::ListBox,
        pub sessions: Rc<RefCell<Vec<Session>>>,
        pub activation_callback: Rc<RefCell<Option<Box<dyn Fn(&Session) + 'static>>>>,
        pub database: RefCell<Option<Rc<Database>>>,
    }

    impl std::fmt::Debug for SessionList {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SessionList")
                .field("list_box", &self.list_box)
                .field("sessions", &self.sessions)
                .finish()
        }
    }

    impl Default for SessionList {
        fn default() -> Self {
            Self {
                list_box: gtk4::ListBox::new(),
                sessions: Rc::new(RefCell::new(Vec::new())),
                activation_callback: Rc::new(RefCell::new(None)),
                database: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SessionList {
        const NAME: &'static str = "SessionListWidget";
        type Type = super::SessionList;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for SessionList {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.set_orientation(gtk4::Orientation::Vertical);
            obj.set_spacing(0);

            // Configure list box
            self.list_box.set_selection_mode(gtk4::SelectionMode::Single);
            self.list_box.add_css_class("boxed-list");

            // Create scrolled window
            let scrolled = gtk4::ScrolledWindow::new();
            scrolled.set_child(Some(&self.list_box));
            scrolled.set_vexpand(true);
            scrolled.set_propagate_natural_height(true);
            scrolled.set_min_content_height(100);

            obj.append(&scrolled);

            // Add "New Session" button
            let new_session_btn = gtk4::Button::with_label("+ New Session");
            new_session_btn.add_css_class("flat");
            new_session_btn.set_margin_top(6);
            new_session_btn.set_margin_bottom(6);
            new_session_btn.set_margin_start(6);
            new_session_btn.set_margin_end(6);
            new_session_btn.set_action_name(Some("app.new-session"));
            obj.append(&new_session_btn);

            // Handle row activation (double-click)
            let sessions_ref = self.sessions.clone();
            let callback_ref = self.activation_callback.clone();
            self.list_box.connect_row_activated(move |_, row| {
                let index = row.index() as usize;
                let sessions = sessions_ref.borrow();
                if let Some(session) = sessions.get(index) {
                    if let Some(callback) = callback_ref.borrow().as_ref() {
                        callback(session);
                    }
                }
            });
        }
    }

    impl WidgetImpl for SessionList {}
    impl BoxImpl for SessionList {}
}

glib::wrapper! {
    pub struct SessionList(ObjectSubclass<imp::SessionList>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Orientable;
}

impl SessionList {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_database(&self, db: Rc<Database>) {
        self.imp().database.replace(Some(db));
        self.load_from_database();
    }

    pub fn connect_session_activated<F: Fn(&Session) + 'static>(&self, f: F) {
        self.imp().activation_callback.replace(Some(Box::new(f)));
    }

    pub fn add_session(&self, session: Session) {
        let imp = self.imp();

        // Save to database
        if let Some(db) = imp.database.borrow().as_ref() {
            if let Err(e) = db.insert_session(&session) {
                log::error!("Failed to save session to database: {}", e);
            }
        }

        // Create row widget
        let row = self.create_session_row(&session);
        imp.list_box.append(&row);

        // Store session
        imp.sessions.borrow_mut().push(session);
    }

    fn load_from_database(&self) {
        let imp = self.imp();

        if let Some(db) = imp.database.borrow().as_ref() {
            match db.get_all_sessions() {
                Ok(sessions) => {
                    log::info!("Loaded {} sessions from database", sessions.len());
                    for session in sessions {
                        let row = self.create_session_row(&session);
                        imp.list_box.append(&row);
                        imp.sessions.borrow_mut().push(session);
                    }
                }
                Err(e) => {
                    log::error!("Failed to load sessions from database: {}", e);
                }
            }
        }
    }

    fn create_session_row(&self, session: &Session) -> gtk4::ListBoxRow {
        let row = gtk4::ListBoxRow::new();
        row.add_css_class("session-row");

        let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        hbox.set_margin_top(8);
        hbox.set_margin_bottom(8);
        hbox.set_margin_start(12);
        hbox.set_margin_end(12);

        // Status indicator
        let status = gtk4::DrawingArea::new();
        status.set_size_request(10, 10);
        status.add_css_class("status-indicator");
        status.add_css_class("disconnected");

        // Icon
        let icon = gtk4::Image::from_icon_name("network-server-symbolic");
        icon.set_pixel_size(16);

        // Session info
        let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        vbox.set_hexpand(true);

        let name_label = gtk4::Label::new(Some(&session.name));
        name_label.set_halign(gtk4::Align::Start);
        name_label.add_css_class("heading");

        let host_label = gtk4::Label::new(Some(&format!(
            "{}@{}:{}",
            session.username, session.host, session.port
        )));
        host_label.set_halign(gtk4::Align::Start);
        host_label.add_css_class("dim-label");
        host_label.add_css_class("caption");

        vbox.append(&name_label);
        vbox.append(&host_label);

        hbox.append(&status);
        hbox.append(&icon);
        hbox.append(&vbox);

        row.set_child(Some(&hbox));
        row
    }

    pub fn clear(&self) {
        let imp = self.imp();
        while let Some(row) = imp.list_box.first_child() {
            imp.list_box.remove(&row);
        }
        imp.sessions.borrow_mut().clear();
    }

    pub fn refresh(&self) {
        self.clear();
        self.load_from_database();
    }
}

impl Default for SessionList {
    fn default() -> Self {
        Self::new()
    }
}
