use crate::storage::{AuthType, Session};
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::glib;
use libadwaita as adw;
use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

mod imp {
    use super::*;

    pub struct SessionDialog {
        // Connection fields
        pub name_entry: RefCell<Option<adw::EntryRow>>,
        pub host_entry: RefCell<Option<adw::EntryRow>>,
        pub port_entry: RefCell<Option<adw::EntryRow>>,
        pub username_entry: RefCell<Option<adw::EntryRow>>,

        // Auth fields
        pub auth_password: RefCell<Option<gtk4::CheckButton>>,
        pub auth_key: RefCell<Option<gtk4::CheckButton>>,
        pub password_entry: RefCell<Option<adw::PasswordEntryRow>>,
        pub key_path_entry: RefCell<Option<adw::EntryRow>>,
        pub passphrase_entry: RefCell<Option<adw::PasswordEntryRow>>,
        pub save_password: RefCell<Option<gtk4::CheckButton>>,

        // Advanced fields
        pub jump_host_check: RefCell<Option<gtk4::CheckButton>>,
        pub jump_host_entry: RefCell<Option<adw::EntryRow>>,
        pub agent_forward_check: RefCell<Option<gtk4::CheckButton>>,
        pub port_forward_check: RefCell<Option<gtk4::CheckButton>>,
        pub local_port_entry: RefCell<Option<adw::EntryRow>>,
        pub remote_addr_entry: RefCell<Option<adw::EntryRow>>,

        // Options
        pub auto_connect: RefCell<Option<gtk4::CheckButton>>,

        // Callback for session creation
        pub on_session_created: Rc<RefCell<Option<Box<dyn Fn(Session) + 'static>>>>,
    }

    impl std::fmt::Debug for SessionDialog {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SessionDialog").finish()
        }
    }

    impl Default for SessionDialog {
        fn default() -> Self {
            Self {
                name_entry: RefCell::new(None),
                host_entry: RefCell::new(None),
                port_entry: RefCell::new(None),
                username_entry: RefCell::new(None),
                auth_password: RefCell::new(None),
                auth_key: RefCell::new(None),
                password_entry: RefCell::new(None),
                key_path_entry: RefCell::new(None),
                passphrase_entry: RefCell::new(None),
                save_password: RefCell::new(None),
                jump_host_check: RefCell::new(None),
                jump_host_entry: RefCell::new(None),
                agent_forward_check: RefCell::new(None),
                port_forward_check: RefCell::new(None),
                local_port_entry: RefCell::new(None),
                remote_addr_entry: RefCell::new(None),
                auto_connect: RefCell::new(None),
                on_session_created: Rc::new(RefCell::new(None)),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SessionDialog {
        const NAME: &'static str = "SessionDialog";
        type Type = super::SessionDialog;
        type ParentType = adw::Window;
    }

    impl ObjectImpl for SessionDialog {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_ui();
        }
    }

    impl WidgetImpl for SessionDialog {}
    impl WindowImpl for SessionDialog {}
    impl AdwWindowImpl for SessionDialog {}
}

glib::wrapper! {
    pub struct SessionDialog(ObjectSubclass<imp::SessionDialog>)
        @extends gtk4::Widget, gtk4::Window, adw::Window,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl SessionDialog {
    pub fn new(parent: &crate::window::TerminuxWindow) -> Self {
        let dialog: Self = glib::Object::builder()
            .property("title", "New SSH Session")
            .property("default-width", 450)
            .property("default-height", 650)
            .property("modal", true)
            .build();

        dialog.set_transient_for(Some(parent));
        dialog
    }

    fn setup_ui(&self) {
        let imp = self.imp();

        // Main container
        let toolbar_view = adw::ToolbarView::new();

        // Header
        let header = adw::HeaderBar::new();
        header.set_show_end_title_buttons(false);
        header.set_show_start_title_buttons(false);

        let cancel_btn = gtk4::Button::with_label("Cancel");
        cancel_btn.connect_clicked(glib::clone!(
            #[weak(rename_to = dialog)]
            self,
            move |_| {
                dialog.close();
            }
        ));
        header.pack_start(&cancel_btn);

        let save_btn = gtk4::Button::with_label("Save & Connect");
        save_btn.add_css_class("suggested-action");
        save_btn.connect_clicked(glib::clone!(
            #[weak(rename_to = dialog)]
            self,
            move |_| {
                dialog.on_save_clicked();
            }
        ));
        header.pack_end(&save_btn);

        toolbar_view.add_top_bar(&header);

        // Content
        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);

        // Session Name
        let name_group = adw::PreferencesGroup::new();
        let name_entry = adw::EntryRow::new();
        name_entry.set_title("Session Name");
        name_group.add(&name_entry);
        content.append(&name_group);
        imp.name_entry.replace(Some(name_entry));

        // Connection section
        let conn_group = adw::PreferencesGroup::new();
        conn_group.set_title("Connection");

        let host_entry = adw::EntryRow::new();
        host_entry.set_title("Host / IP");
        conn_group.add(&host_entry);
        imp.host_entry.replace(Some(host_entry));

        let port_entry = adw::EntryRow::new();
        port_entry.set_title("Port");
        port_entry.set_text("22");
        conn_group.add(&port_entry);
        imp.port_entry.replace(Some(port_entry));

        let username_entry = adw::EntryRow::new();
        username_entry.set_title("Username");
        conn_group.add(&username_entry);
        imp.username_entry.replace(Some(username_entry));

        content.append(&conn_group);

        // Authentication section
        let auth_group = adw::PreferencesGroup::new();
        auth_group.set_title("Authentication");

        // Password auth row
        let password_row = adw::ActionRow::new();
        password_row.set_title("Password");

        let auth_password = gtk4::CheckButton::new();
        auth_password.set_active(true);
        password_row.add_prefix(&auth_password);
        password_row.set_activatable_widget(Some(&auth_password));

        auth_group.add(&password_row);

        let password_entry = adw::PasswordEntryRow::new();
        password_entry.set_title("Password");
        auth_group.add(&password_entry);
        imp.password_entry.replace(Some(password_entry.clone()));

        let save_password_row = adw::ActionRow::new();
        save_password_row.set_title("Save password in keyring");
        let save_password = gtk4::CheckButton::new();
        save_password_row.add_prefix(&save_password);
        save_password_row.set_activatable_widget(Some(&save_password));
        auth_group.add(&save_password_row);
        imp.save_password.replace(Some(save_password));

        // Key auth row
        let key_row = adw::ActionRow::new();
        key_row.set_title("SSH Key");

        let auth_key = gtk4::CheckButton::new();
        auth_key.set_group(Some(&auth_password));
        key_row.add_prefix(&auth_key);
        key_row.set_activatable_widget(Some(&auth_key));

        auth_group.add(&key_row);

        let key_path_entry = adw::EntryRow::new();
        key_path_entry.set_title("Key file");
        key_path_entry.set_text("~/.ssh/id_rsa");

        let key_browse_btn = gtk4::Button::from_icon_name("document-open-symbolic");
        key_browse_btn.set_valign(gtk4::Align::Center);
        key_browse_btn.add_css_class("flat");
        key_path_entry.add_suffix(&key_browse_btn);

        key_browse_btn.connect_clicked(glib::clone!(
            #[weak(rename_to = dialog)]
            self,
            #[weak]
            key_path_entry,
            move |_| {
                let file_dialog = gtk4::FileDialog::new();
                file_dialog.set_title("Select SSH Key");

                // Start in ~/.ssh directory if it exists
                let ssh_dir = glib::home_dir().join(".ssh");
                if ssh_dir.exists() {
                    let file = gtk4::gio::File::for_path(&ssh_dir);
                    file_dialog.set_initial_folder(Some(&file));
                }

                file_dialog.open(
                    Some(&dialog),
                    gtk4::gio::Cancellable::NONE,
                    glib::clone!(
                        #[weak]
                        key_path_entry,
                        move |result| {
                            if let Ok(file) = result {
                                if let Some(path) = file.path() {
                                    key_path_entry.set_text(&path.to_string_lossy());
                                }
                            }
                        }
                    ),
                );
            }
        ));

        auth_group.add(&key_path_entry);
        imp.key_path_entry.replace(Some(key_path_entry.clone()));

        let passphrase_entry = adw::PasswordEntryRow::new();
        passphrase_entry.set_title("Passphrase (optional)");
        auth_group.add(&passphrase_entry);
        imp.passphrase_entry.replace(Some(passphrase_entry.clone()));

        content.append(&auth_group);

        // Toggle visibility based on auth type
        imp.auth_password.replace(Some(auth_password.clone()));
        imp.auth_key.replace(Some(auth_key.clone()));

        auth_password.connect_toggled(glib::clone!(
            #[weak]
            password_entry,
            #[weak]
            key_path_entry,
            #[weak]
            passphrase_entry,
            move |btn| {
                let is_password = btn.is_active();
                password_entry.set_sensitive(is_password);
                key_path_entry.set_sensitive(!is_password);
                passphrase_entry.set_sensitive(!is_password);
            }
        ));

        // Set initial sensitivity
        key_path_entry.set_sensitive(false);
        passphrase_entry.set_sensitive(false);

        // Advanced section
        let advanced_group = adw::PreferencesGroup::new();
        advanced_group.set_title("Advanced");

        // Jump host
        let jump_host_row = adw::ActionRow::new();
        jump_host_row.set_title("Use Jump Host (ProxyJump)");
        let jump_host_check = gtk4::CheckButton::new();
        jump_host_row.add_prefix(&jump_host_check);
        jump_host_row.set_activatable_widget(Some(&jump_host_check));
        advanced_group.add(&jump_host_row);
        imp.jump_host_check.replace(Some(jump_host_check.clone()));

        let jump_host_entry = adw::EntryRow::new();
        jump_host_entry.set_title("Jump Host");
        jump_host_entry.set_sensitive(false);
        advanced_group.add(&jump_host_entry);
        imp.jump_host_entry.replace(Some(jump_host_entry.clone()));

        jump_host_check.connect_toggled(glib::clone!(
            #[weak]
            jump_host_entry,
            move |btn| {
                jump_host_entry.set_sensitive(btn.is_active());
            }
        ));

        // Agent forwarding
        let agent_row = adw::ActionRow::new();
        agent_row.set_title("Enable Agent Forwarding");
        let agent_forward_check = gtk4::CheckButton::new();
        agent_row.add_prefix(&agent_forward_check);
        agent_row.set_activatable_widget(Some(&agent_forward_check));
        advanced_group.add(&agent_row);
        imp.agent_forward_check.replace(Some(agent_forward_check));

        // Port forwarding
        let port_forward_row = adw::ActionRow::new();
        port_forward_row.set_title("Port Forwarding");
        let port_forward_check = gtk4::CheckButton::new();
        port_forward_row.add_prefix(&port_forward_check);
        port_forward_row.set_activatable_widget(Some(&port_forward_check));
        advanced_group.add(&port_forward_row);
        imp.port_forward_check.replace(Some(port_forward_check.clone()));

        let local_port_entry = adw::EntryRow::new();
        local_port_entry.set_title("Local Port");
        local_port_entry.set_sensitive(false);
        advanced_group.add(&local_port_entry);
        imp.local_port_entry.replace(Some(local_port_entry.clone()));

        let remote_addr_entry = adw::EntryRow::new();
        remote_addr_entry.set_title("Remote Address (host:port)");
        remote_addr_entry.set_sensitive(false);
        advanced_group.add(&remote_addr_entry);
        imp.remote_addr_entry.replace(Some(remote_addr_entry.clone()));

        port_forward_check.connect_toggled(glib::clone!(
            #[weak]
            local_port_entry,
            #[weak]
            remote_addr_entry,
            move |btn| {
                let active = btn.is_active();
                local_port_entry.set_sensitive(active);
                remote_addr_entry.set_sensitive(active);
            }
        ));

        content.append(&advanced_group);

        // Options section
        let options_group = adw::PreferencesGroup::new();
        options_group.set_title("Options");

        let auto_connect_row = adw::ActionRow::new();
        auto_connect_row.set_title("Connect on startup");
        let auto_connect = gtk4::CheckButton::new();
        auto_connect_row.add_prefix(&auto_connect);
        auto_connect_row.set_activatable_widget(Some(&auto_connect));
        options_group.add(&auto_connect_row);
        imp.auto_connect.replace(Some(auto_connect));

        content.append(&options_group);

        scrolled.set_child(Some(&content));
        toolbar_view.set_content(Some(&scrolled));

        self.set_content(Some(&toolbar_view));
    }

    fn on_save_clicked(&self) {
        let imp = self.imp();

        // Gather data from form
        let name = imp.name_entry.borrow().as_ref().map(|e| e.text().to_string()).unwrap_or_default();
        let host = imp.host_entry.borrow().as_ref().map(|e| e.text().to_string()).unwrap_or_default();
        let port: u16 = imp.port_entry.borrow()
            .as_ref()
            .map(|e| e.text().parse().unwrap_or(22))
            .unwrap_or(22);
        let username = imp.username_entry.borrow().as_ref().map(|e| e.text().to_string()).unwrap_or_default();

        // Validate
        if host.is_empty() || username.is_empty() {
            // Show error toast
            log::warn!("Host and username are required");
            return;
        }

        let auth_type = if imp.auth_key.borrow().as_ref().map(|b| b.is_active()).unwrap_or(false) {
            AuthType::Key
        } else {
            AuthType::Password
        };

        let key_path = if matches!(auth_type, AuthType::Key) {
            imp.key_path_entry.borrow().as_ref().map(|e| e.text().to_string())
        } else {
            None
        };

        let jump_host = if imp.jump_host_check.borrow().as_ref().map(|c| c.is_active()).unwrap_or(false) {
            imp.jump_host_entry.borrow().as_ref().map(|e| e.text().to_string())
        } else {
            None
        };

        let agent_forwarding = imp.agent_forward_check.borrow().as_ref().map(|c| c.is_active()).unwrap_or(false);

        let (port_forward_local, port_forward_remote) = if imp.port_forward_check.borrow().as_ref().map(|c| c.is_active()).unwrap_or(false) {
            let local: Option<u16> = imp.local_port_entry.borrow().as_ref().and_then(|e| e.text().parse().ok());
            let remote: Option<String> = imp.remote_addr_entry.borrow().as_ref().map(|e| e.text().to_string());
            (local, remote)
        } else {
            (None, None)
        };

        let auto_connect = imp.auto_connect.borrow().as_ref().map(|c| c.is_active()).unwrap_or(false);

        let session = Session {
            id: uuid::Uuid::new_v4().to_string(),
            name: if name.is_empty() { format!("{}@{}", username, host) } else { name },
            host,
            port,
            username,
            auth_type,
            key_path,
            folder_id: None,
            auto_connect,
            jump_host,
            agent_forwarding,
            port_forward_local,
            port_forward_remote,
        };

        log::info!("Creating session: {:?}", session);

        // TODO: Save to database

        // Call the session created callback
        if let Some(callback) = self.imp().on_session_created.borrow().as_ref() {
            callback(session);
        }

        // Close dialog
        self.close();
    }

    pub fn connect_session_created<F: Fn(Session) + 'static>(&self, f: F) {
        self.imp().on_session_created.replace(Some(Box::new(f)));
    }
}

impl Default for SessionDialog {
    fn default() -> Self {
        glib::Object::new()
    }
}
