use crate::ssh::{SftpClient, SftpEntry};
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::glib;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

mod imp {
    use super::*;

    #[derive(Debug)]
    pub struct FileBrowser {
        pub list_box: gtk4::ListBox,
        pub path_label: gtk4::Label,
        pub toolbar: gtk4::Box,
        pub sftp_client: RefCell<Option<Arc<SftpClient>>>,
        pub current_path: RefCell<String>,
        pub entries: RefCell<HashMap<i32, super::FileEntry>>,
    }

    impl Default for FileBrowser {
        fn default() -> Self {
            Self {
                list_box: gtk4::ListBox::new(),
                path_label: gtk4::Label::new(Some("Not connected")),
                toolbar: gtk4::Box::new(gtk4::Orientation::Horizontal, 4),
                sftp_client: RefCell::new(None),
                current_path: RefCell::new("/".to_string()),
                entries: RefCell::new(HashMap::new()),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileBrowser {
        const NAME: &'static str = "FileBrowserWidget";
        type Type = super::FileBrowser;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for FileBrowser {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.set_orientation(gtk4::Orientation::Vertical);
            obj.set_spacing(0);
            obj.add_css_class("file-browser");

            // Path bar
            let path_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
            path_box.set_margin_start(8);
            path_box.set_margin_end(8);
            path_box.set_margin_top(4);
            path_box.set_margin_bottom(4);

            self.path_label.set_halign(gtk4::Align::Start);
            self.path_label.set_hexpand(true);
            self.path_label.set_ellipsize(gtk4::pango::EllipsizeMode::Start);
            self.path_label.add_css_class("dim-label");
            self.path_label.add_css_class("monospace");

            path_box.append(&self.path_label);
            obj.append(&path_box);

            // Separator
            let sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
            obj.append(&sep);

            // File list
            self.list_box.set_selection_mode(gtk4::SelectionMode::Single);
            self.list_box.add_css_class("boxed-list");

            let scrolled = gtk4::ScrolledWindow::new();
            scrolled.set_child(Some(&self.list_box));
            scrolled.set_vexpand(true);
            scrolled.set_min_content_height(150);

            obj.append(&scrolled);

            // Toolbar
            let sep2 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
            obj.append(&sep2);

            self.toolbar.set_margin_start(8);
            self.toolbar.set_margin_end(8);
            self.toolbar.set_margin_top(4);
            self.toolbar.set_margin_bottom(4);
            self.toolbar.set_halign(gtk4::Align::Center);

            // Up button
            let up_btn = gtk4::Button::from_icon_name("go-up-symbolic");
            up_btn.set_tooltip_text(Some("Go to parent directory"));
            up_btn.add_css_class("flat");
            up_btn.connect_clicked(glib::clone!(
                #[weak]
                obj,
                move |_| {
                    obj.navigate_up();
                }
            ));

            // Refresh button
            let refresh_btn = gtk4::Button::from_icon_name("view-refresh-symbolic");
            refresh_btn.set_tooltip_text(Some("Refresh"));
            refresh_btn.add_css_class("flat");
            refresh_btn.connect_clicked(glib::clone!(
                #[weak]
                obj,
                move |_| {
                    obj.refresh();
                }
            ));

            // Download button
            let download_btn = gtk4::Button::from_icon_name("document-save-symbolic");
            download_btn.set_tooltip_text(Some("Download selected file"));
            download_btn.add_css_class("flat");

            // Upload button
            let upload_btn = gtk4::Button::from_icon_name("document-open-symbolic");
            upload_btn.set_tooltip_text(Some("Upload file"));
            upload_btn.add_css_class("flat");

            self.toolbar.append(&up_btn);
            self.toolbar.append(&refresh_btn);
            self.toolbar.append(&download_btn);
            self.toolbar.append(&upload_btn);

            obj.append(&self.toolbar);

            // Handle row activation (directory navigation)
            self.list_box.connect_row_activated(glib::clone!(
                #[weak]
                obj,
                move |_, row| {
                    let index = row.index();
                    let imp = obj.imp();
                    let entries = imp.entries.borrow();
                    if let Some(entry) = entries.get(&index) {
                        if entry.is_directory {
                            let current = imp.current_path.borrow().clone();
                            let new_path = if entry.name == ".." {
                                std::path::Path::new(&current)
                                    .parent()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "/".to_string())
                            } else if current.ends_with('/') {
                                format!("{}{}", current, entry.name)
                            } else {
                                format!("{}/{}", current, entry.name)
                            };
                            drop(entries); // Release borrow before calling load_directory
                            obj.load_directory(&new_path);
                        }
                    }
                }
            ));

            // Show placeholder content
            obj.show_placeholder();
        }
    }

    impl WidgetImpl for FileBrowser {}
    impl BoxImpl for FileBrowser {}
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub is_directory: bool,
    pub size: u64,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
}

glib::wrapper! {
    pub struct FileBrowser(ObjectSubclass<imp::FileBrowser>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Orientable;
}

impl FileBrowser {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_sftp_client(&self, client: Option<Arc<SftpClient>>) {
        let imp = self.imp();
        imp.sftp_client.replace(client.clone());

        if client.is_some() {
            // Load home directory
            self.load_home_directory();
        } else {
            self.show_placeholder();
        }
    }

    fn load_home_directory(&self) {
        let imp = self.imp();

        if let Some(sftp) = imp.sftp_client.borrow().clone() {
            // Show loading state
            imp.path_label.set_text("Loading...");

            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = browser)]
                self,
                async move {
                    let home = std::thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async {
                            sftp.home_directory().await.unwrap_or_else(|_| "/".to_string())
                        })
                    }).join().unwrap_or_else(|_| "/".to_string());

                    browser.load_directory(&home);
                }
            ));
        }
    }

    pub fn load_directory(&self, path: &str) {
        let imp = self.imp();
        imp.current_path.replace(path.to_string());
        imp.path_label.set_text(path);

        // Clear existing entries
        imp.entries.borrow_mut().clear();
        while let Some(row) = imp.list_box.first_child() {
            imp.list_box.remove(&row);
        }

        // Load from SFTP
        if let Some(sftp) = imp.sftp_client.borrow().clone() {
            let path = path.to_string();

            // Add loading indicator
            let loading = gtk4::Spinner::new();
            loading.start();
            loading.set_margin_top(20);
            loading.set_margin_bottom(20);
            let loading_row = gtk4::ListBoxRow::new();
            loading_row.set_selectable(false);
            loading_row.set_child(Some(&loading));
            imp.list_box.append(&loading_row);

            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = browser)]
                self,
                async move {
                    let result = std::thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async {
                            sftp.list_directory(&path).await
                        })
                    }).join();

                    // Clear loading indicator
                    let imp = browser.imp();
                    while let Some(row) = imp.list_box.first_child() {
                        imp.list_box.remove(&row);
                    }

                    match result {
                        Ok(Ok(entries)) => {
                            for entry in entries {
                                browser.add_sftp_entry(&entry);
                            }
                        }
                        Ok(Err(e)) => {
                            log::error!("Failed to list directory: {}", e);
                            browser.show_error(&format!("Error: {}", e));
                        }
                        Err(_) => {
                            browser.show_error("Failed to list directory");
                        }
                    }
                }
            ));
        } else {
            self.show_placeholder();
        }
    }

    fn add_sftp_entry(&self, entry: &SftpEntry) {
        let imp = self.imp();

        let row = gtk4::ListBoxRow::new();
        row.add_css_class("file-row");
        if entry.is_directory {
            row.add_css_class("directory");
        }

        let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        hbox.set_margin_top(6);
        hbox.set_margin_bottom(6);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);

        // Icon
        let icon_name = if entry.is_directory {
            "folder-symbolic"
        } else {
            "text-x-generic-symbolic"
        };
        let icon = gtk4::Image::from_icon_name(icon_name);
        icon.set_pixel_size(16);

        // Name
        let name_label = gtk4::Label::new(Some(&entry.name));
        name_label.set_halign(gtk4::Align::Start);
        name_label.set_hexpand(true);

        // Size (for files)
        let size_label = if entry.is_directory {
            gtk4::Label::new(None)
        } else {
            let size_str = Self::format_size(entry.size);
            let label = gtk4::Label::new(Some(&size_str));
            label.add_css_class("dim-label");
            label.add_css_class("numeric");
            label
        };

        hbox.append(&icon);
        hbox.append(&name_label);
        hbox.append(&size_label);

        row.set_child(Some(&hbox));
        imp.list_box.append(&row);

        // Store entry data in HashMap using row index
        let file_entry = FileEntry {
            name: entry.name.clone(),
            is_directory: entry.is_directory,
            size: entry.size,
            modified: None,
        };
        imp.entries.borrow_mut().insert(row.index(), file_entry);
    }

    fn show_error(&self, message: &str) {
        let imp = self.imp();

        let error_label = gtk4::Label::new(Some(message));
        error_label.set_margin_top(20);
        error_label.set_margin_bottom(20);
        error_label.add_css_class("dim-label");
        error_label.add_css_class("error");

        let row = gtk4::ListBoxRow::new();
        row.set_selectable(false);
        row.set_activatable(false);
        row.set_child(Some(&error_label));

        imp.list_box.append(&row);
    }

    fn show_placeholder(&self) {
        let imp = self.imp();
        imp.path_label.set_text("Not connected");

        // Clear existing entries
        imp.entries.borrow_mut().clear();
        while let Some(row) = imp.list_box.first_child() {
            imp.list_box.remove(&row);
        }

        // Add placeholder message
        let placeholder = gtk4::Label::new(Some("Connect to a server\nto browse files"));
        placeholder.set_margin_top(20);
        placeholder.set_margin_bottom(20);
        placeholder.add_css_class("dim-label");
        placeholder.set_justify(gtk4::Justification::Center);

        let row = gtk4::ListBoxRow::new();
        row.set_selectable(false);
        row.set_activatable(false);
        row.set_child(Some(&placeholder));

        imp.list_box.append(&row);
    }

    fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    pub fn navigate_up(&self) {
        let imp = self.imp();
        let current = imp.current_path.borrow().clone();

        if current != "/" {
            let parent = std::path::Path::new(&current)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "/".to_string());

            self.load_directory(&parent);
        }
    }

    pub fn refresh(&self) {
        let current = self.imp().current_path.borrow().clone();
        self.load_directory(&current);
    }
}

impl Default for FileBrowser {
    fn default() -> Self {
        Self::new()
    }
}
