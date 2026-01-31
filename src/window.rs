use crate::app::TerminuxApplication;
use crate::ui::{FileBrowser, MatrixRain, SessionList, TerminalView};
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{gio, glib};
use libadwaita as adw;
use libadwaita::subclass::prelude::*;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk4::CompositeTemplate)]
    #[template(string = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <interface>
            <template class="TerminuxWindow" parent="AdwApplicationWindow">
                <property name="title">Terminux</property>
                <property name="default-width">1200</property>
                <property name="default-height">800</property>
                <child>
                    <object class="GtkOverlay" id="main_overlay">
                        <child>
                            <object class="AdwToolbarView">
                                <child type="top">
                                    <object class="AdwHeaderBar" id="header_bar">
                                        <child type="start">
                                            <object class="GtkButton" id="new_session_btn">
                                                <property name="icon-name">list-add-symbolic</property>
                                                <property name="tooltip-text">New Session (Ctrl+Shift+N)</property>
                                                <property name="action-name">app.new-session</property>
                                            </object>
                                        </child>
                                        <child type="end">
                                            <object class="GtkMenuButton" id="menu_button">
                                                <property name="icon-name">open-menu-symbolic</property>
                                                <property name="menu-model">primary_menu</property>
                                                <property name="tooltip-text">Main Menu</property>
                                            </object>
                                        </child>
                                    </object>
                                </child>
                                <child>
                                    <object class="GtkPaned" id="main_paned">
                                        <property name="orientation">horizontal</property>
                                        <property name="position">800</property>
                                        <property name="shrink-start-child">false</property>
                                        <property name="shrink-end-child">false</property>
                                        <property name="resize-start-child">true</property>
                                        <property name="resize-end-child">false</property>
                                        <style>
                                            <class name="main-paned"/>
                                        </style>
                                        <child>
                                            <object class="AdwTabView" id="tab_view">
                                            </object>
                                        </child>
                                        <child>
                                            <object class="GtkBox" id="sidebar_box">
                                                <property name="orientation">vertical</property>
                                                <property name="width-request">300</property>
                                                <style>
                                                    <class name="sidebar-panel"/>
                                                </style>
                                            </object>
                                        </child>
                                    </object>
                                </child>
                                <child type="top">
                                    <object class="AdwTabBar" id="tab_bar">
                                        <property name="view">tab_view</property>
                                    </object>
                                </child>
                            </object>
                        </child>
                    </object>
                </child>
            </template>
            <menu id="primary_menu">
                <section>
                    <item>
                        <attribute name="label" translatable="yes">New Session</attribute>
                        <attribute name="action">app.new-session</attribute>
                    </item>
                    <item>
                        <attribute name="label" translatable="yes">New Local Tab</attribute>
                        <attribute name="action">app.new-tab</attribute>
                    </item>
                </section>
                <section>
                    <item>
                        <attribute name="label" translatable="yes">About Terminux</attribute>
                        <attribute name="action">app.about</attribute>
                    </item>
                    <item>
                        <attribute name="label" translatable="yes">Quit</attribute>
                        <attribute name="action">app.quit</attribute>
                    </item>
                </section>
            </menu>
        </interface>
    "#)]
    pub struct TerminuxWindow {
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub tab_view: TemplateChild<adw::TabView>,
        #[template_child]
        pub tab_bar: TemplateChild<adw::TabBar>,
        #[template_child]
        pub main_paned: TemplateChild<gtk4::Paned>,
        #[template_child]
        pub sidebar_box: TemplateChild<gtk4::Box>,
        #[template_child]
        pub main_overlay: TemplateChild<gtk4::Overlay>,

        pub session_list: RefCell<Option<SessionList>>,
        pub file_browser: RefCell<Option<FileBrowser>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TerminuxWindow {
        const NAME: &'static str = "TerminuxWindow";
        type Type = super::TerminuxWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TerminuxWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_sidebar();
            obj.setup_tab_view();
            obj.setup_actions();
            obj.setup_matrix_rain();

            // Add initial local terminal tab
            obj.add_local_terminal_tab();
        }
    }

    impl WidgetImpl for TerminuxWindow {}
    impl WindowImpl for TerminuxWindow {}
    impl ApplicationWindowImpl for TerminuxWindow {}
    impl AdwApplicationWindowImpl for TerminuxWindow {}
}

glib::wrapper! {
    pub struct TerminuxWindow(ObjectSubclass<imp::TerminuxWindow>)
        @extends gtk4::Widget, gtk4::Window, gtk4::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl TerminuxWindow {
    pub fn new(app: &crate::app::TerminuxApplication) -> Self {
        let window: Self = glib::Object::builder().property("application", app).build();

        // Set up database after window is created (application property is now available)
        if let Some(db) = app.database() {
            if let Some(session_list) = window.imp().session_list.borrow().as_ref() {
                session_list.set_database(db);
            }
        }

        window
    }

    fn setup_matrix_rain(&self) {
        let rain = MatrixRain::new();
        rain.set_can_target(false);
        rain.set_can_focus(false);
        rain.set_hexpand(true);
        rain.set_vexpand(true);
        self.imp().main_overlay.add_overlay(&rain);
    }

    fn setup_sidebar(&self) {
        let imp = self.imp();

        // Create sessions panel
        let sessions_frame = gtk4::Frame::new(None);
        let sessions_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

        let sessions_header = gtk4::Label::new(Some("Sessions"));
        sessions_header.add_css_class("sidebar-header");
        sessions_header.set_halign(gtk4::Align::Start);

        let session_list = SessionList::new();

        // Connect session activation
        let window = self.clone();
        session_list.connect_session_activated(move |session| {
            window.connect_to_session(session);
        });

        sessions_box.append(&sessions_header);
        sessions_box.append(&session_list);
        sessions_frame.set_child(Some(&sessions_box));
        sessions_frame.set_vexpand(true);

        // Create file browser panel
        let browser_frame = gtk4::Frame::new(None);
        let browser_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

        let browser_header = gtk4::Label::new(Some("File Browser"));
        browser_header.add_css_class("sidebar-header");
        browser_header.set_halign(gtk4::Align::Start);

        let file_browser = FileBrowser::new();

        browser_box.append(&browser_header);
        browser_box.append(&file_browser);
        browser_frame.set_child(Some(&browser_box));
        browser_frame.set_vexpand(true);

        // Add to sidebar using a paned widget for resizable sections
        let sidebar_paned = gtk4::Paned::new(gtk4::Orientation::Vertical);
        sidebar_paned.set_start_child(Some(&sessions_frame));
        sidebar_paned.set_end_child(Some(&browser_frame));
        sidebar_paned.set_position(350);

        imp.sidebar_box.append(&sidebar_paned);

        // Store references
        imp.session_list.replace(Some(session_list));
        imp.file_browser.replace(Some(file_browser));
    }

    fn setup_tab_view(&self) {
        let imp = self.imp();
        let tab_view = &imp.tab_view;

        // Setup tab view signals
        tab_view.connect_close_page(|tab_view, page| {
            // Check if this is the last tab
            if tab_view.n_pages() <= 1 {
                // Don't close the last tab, instead close the window
                if let Some(window) = page.child().root() {
                    if let Some(win) = window.downcast_ref::<gtk4::Window>() {
                        win.close();
                    }
                }
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });

        // Handle tab selection changes (for file browser context)
        let window = self.clone();
        tab_view.connect_selected_page_notify(move |tab_view| {
            if let Some(page) = tab_view.selected_page() {
                window.on_tab_selected(&page);
            }
        });
    }

    fn setup_actions(&self) {
        // Close tab action
        let action_close_tab = gio::ActionEntry::builder("close-tab")
            .activate(|win: &Self, _, _| {
                let tab_view = &win.imp().tab_view;
                if let Some(page) = tab_view.selected_page() {
                    tab_view.close_page(&page);
                }
            })
            .build();

        self.add_action_entries([action_close_tab]);
    }

    pub fn add_local_terminal_tab(&self) {
        let imp = self.imp();

        let terminal = TerminalView::new_local();
        let page = imp.tab_view.append(&terminal);
        page.set_title("Local");
        page.set_icon(Some(&gio::ThemedIcon::new("utilities-terminal-symbolic")));

        imp.tab_view.set_selected_page(&page);
    }

    pub fn add_ssh_terminal_tab(&self, session: &crate::storage::Session) {
        let imp = self.imp();

        let terminal = TerminalView::new_ssh(session.clone());
        let page = imp.tab_view.append(&terminal);
        page.set_title(&session.name);
        page.set_icon(Some(&gio::ThemedIcon::new("network-server-symbolic")));

        imp.tab_view.set_selected_page(&page);

        // Connect SFTP ready callback to update file browser
        if let Some(file_browser) = imp.file_browser.borrow().clone() {
            terminal.connect_sftp_ready(move |sftp| {
                file_browser.set_sftp_client(Some(sftp));
            });
        }

        // For password auth, we would show a dialog here
        // For now, attempt connection with key auth or empty password
        let password = if matches!(session.auth_type, crate::storage::AuthType::Password) {
            // TODO: Show password dialog or retrieve from keyring
            Some(String::new())
        } else {
            None
        };

        terminal.connect_ssh(password);
    }

    pub fn show_new_session_dialog(&self) {
        let dialog = crate::ui::SessionDialog::new(self);

        // Handle session creation
        let window = self.clone();
        dialog.connect_session_created(move |session| {
            // Add session to the sidebar list
            if let Some(session_list) = window.imp().session_list.borrow().as_ref() {
                session_list.add_session(session.clone());
            }

            // Connect to the session
            window.add_ssh_terminal_tab(&session);
        });

        dialog.present();
    }

    fn connect_to_session(&self, session: &crate::storage::Session) {
        self.add_ssh_terminal_tab(session);
    }

    fn on_tab_selected(&self, page: &adw::TabPage) {
        let imp = self.imp();

        // Update file browser based on the selected terminal's SSH connection
        if let Some(file_browser) = imp.file_browser.borrow().as_ref() {
            if let Some(terminal) = page.child().downcast_ref::<TerminalView>() {
                if let Some(sftp) = terminal.get_sftp_client() {
                    file_browser.set_sftp_client(Some(sftp));
                } else {
                    file_browser.set_sftp_client(None);
                }
            }
        }
    }
}
