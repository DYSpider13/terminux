use crate::ssh::SftpClient;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::glib;
use std::cell::RefCell;
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
    }

    impl Default for FileBrowser {
        fn default() -> Self {
            Self {
                list_box: gtk4::ListBox::new(),
                path_label: gtk4::Label::new(Some("Not connected")),
                toolbar: gtk4::Box::new(gtk4::Orientation::Horizontal, 4),
                sftp_client: RefCell::new(None),
                current_path: RefCell::new("/".to_string()),
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
            let path_ref = self.current_path.clone();
            let label_ref = self.path_label.clone();
            let list_ref = self.list_box.clone();
            self.list_box.connect_row_activated(move |_, row| {
                // SAFETY: We set this data ourselves with the same type
                if let Some(entry) = unsafe { row.data::<FileEntry>("file-entry") } {
                    let entry = unsafe { &*entry.as_ptr() };
                    if entry.is_directory {
                        let new_path = if path_ref.borrow().ends_with('/') {
                            format!("{}{}", path_ref.borrow(), entry.name)
                        } else {
                            format!("{}/{}", path_ref.borrow(), entry.name)
                        };
                        path_ref.replace(new_path.clone());
                        label_ref.set_text(&new_path);

                        // Clear and show placeholder (actual loading would happen via SFTP)
                        while let Some(child) = list_ref.first_child() {
                            list_ref.remove(&child);
                        }
                    }
                }
            });

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
            self.load_directory("/");
        } else {
            self.show_placeholder();
        }
    }

    pub fn load_directory(&self, path: &str) {
        let imp = self.imp();
        imp.current_path.replace(path.to_string());
        imp.path_label.set_text(path);

        // Clear existing entries
        while let Some(row) = imp.list_box.first_child() {
            imp.list_box.remove(&row);
        }

        // TODO: Actually load from SFTP
        // For now, show sample data
        self.show_sample_files();
    }

    fn show_placeholder(&self) {
        let imp = self.imp();
        imp.path_label.set_text("Not connected");

        // Clear existing entries
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

    fn show_sample_files(&self) {
        let entries = vec![
            FileEntry {
                name: "..".to_string(),
                is_directory: true,
                size: 0,
                modified: None,
            },
            FileEntry {
                name: "Documents".to_string(),
                is_directory: true,
                size: 0,
                modified: None,
            },
            FileEntry {
                name: "Projects".to_string(),
                is_directory: true,
                size: 0,
                modified: None,
            },
            FileEntry {
                name: ".bashrc".to_string(),
                is_directory: false,
                size: 3771,
                modified: None,
            },
            FileEntry {
                name: ".profile".to_string(),
                is_directory: false,
                size: 807,
                modified: None,
            },
            FileEntry {
                name: "notes.txt".to_string(),
                is_directory: false,
                size: 1234,
                modified: None,
            },
        ];

        for entry in entries {
            self.add_file_entry(&entry);
        }
    }

    fn add_file_entry(&self, entry: &FileEntry) {
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

        // Store entry data on the row
        unsafe {
            row.set_data("file-entry", Box::new(entry.clone()));
        }

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
