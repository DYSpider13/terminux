use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::glib;
use vte4::prelude::*;
use std::cell::RefCell;
use std::sync::Arc;

use crate::ssh::{SftpClient, SshCommand, SshEvent};
use crate::storage::Session;

mod imp {
    use super::*;
    use async_channel::Sender;

    pub struct TerminalView {
        pub vte: vte4::Terminal,
        pub sftp_client: RefCell<Option<Arc<SftpClient>>>,
        pub is_ssh: RefCell<bool>,
        pub session: RefCell<Option<Session>>,
        pub command_sender: RefCell<Option<Sender<SshCommand>>>,
        pub sftp_ready_callback: RefCell<Option<Box<dyn Fn(Arc<SftpClient>) + 'static>>>,
    }

    impl std::fmt::Debug for TerminalView {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("TerminalView")
                .field("vte", &self.vte)
                .field("is_ssh", &self.is_ssh)
                .finish()
        }
    }

    impl Default for TerminalView {
        fn default() -> Self {
            Self {
                vte: vte4::Terminal::new(),
                sftp_client: RefCell::new(None),
                is_ssh: RefCell::new(false),
                session: RefCell::new(None),
                command_sender: RefCell::new(None),
                sftp_ready_callback: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TerminalView {
        const NAME: &'static str = "TerminalViewWidget";
        type Type = super::TerminalView;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for TerminalView {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.set_orientation(gtk4::Orientation::Vertical);
            obj.add_css_class("terminal-view");

            // Configure VTE terminal
            self.vte.set_scroll_on_output(false);
            self.vte.set_scroll_on_keystroke(true);
            self.vte.set_scrollback_lines(10000);
            self.vte.set_cursor_blink_mode(vte4::CursorBlinkMode::On);
            self.vte.set_cursor_shape(vte4::CursorShape::Block);

            // Set font
            let font_desc = gtk4::pango::FontDescription::from_string("Monospace 11");
            self.vte.set_font(Some(&font_desc));

            // Set colors (cyberpunk/Matrix theme)
            let fg = gtk4::gdk::RGBA::parse("#c5d0dc").unwrap();
            let bg = gtk4::gdk::RGBA::parse("#0a0e14").unwrap();

            let palette: [gtk4::gdk::RGBA; 16] = [
                gtk4::gdk::RGBA::parse("#0a0e14").unwrap(), // Black
                gtk4::gdk::RGBA::parse("#ff2e97").unwrap(), // Red (hot pink)
                gtk4::gdk::RGBA::parse("#00ff41").unwrap(), // Green (neon)
                gtk4::gdk::RGBA::parse("#ffb700").unwrap(), // Yellow (amber)
                gtk4::gdk::RGBA::parse("#00e5ff").unwrap(), // Blue (cyan)
                gtk4::gdk::RGBA::parse("#c74ded").unwrap(), // Magenta (purple)
                gtk4::gdk::RGBA::parse("#00e5ff").unwrap(), // Cyan
                gtk4::gdk::RGBA::parse("#c5d0dc").unwrap(), // White
                gtk4::gdk::RGBA::parse("#4a5568").unwrap(), // Bright Black (dim)
                gtk4::gdk::RGBA::parse("#ff6ac1").unwrap(), // Bright Red (lighter pink)
                gtk4::gdk::RGBA::parse("#69ff94").unwrap(), // Bright Green
                gtk4::gdk::RGBA::parse("#ffd866").unwrap(), // Bright Yellow
                gtk4::gdk::RGBA::parse("#62efff").unwrap(), // Bright Blue (light cyan)
                gtk4::gdk::RGBA::parse("#d98ef0").unwrap(), // Bright Magenta
                gtk4::gdk::RGBA::parse("#62efff").unwrap(), // Bright Cyan
                gtk4::gdk::RGBA::parse("#eaf2ff").unwrap(), // Bright White
            ];

            let palette_refs: Vec<&gtk4::gdk::RGBA> = palette.iter().collect();
            self.vte.set_colors(Some(&fg), Some(&bg), &palette_refs);

            // VTE handles its own scrolling, so add it directly without ScrolledWindow
            // Using ScrolledWindow can cause conflicts with VTE's internal scroll buffer
            // and readline features like Ctrl+R (reverse search)
            self.vte.set_vexpand(true);
            self.vte.set_hexpand(true);

            obj.append(&self.vte);

            // Connect terminal signals
            self.vte.connect_child_exited(glib::clone!(
                #[weak]
                obj,
                move |_, _| {
                    log::info!("Terminal child process exited");
                }
            ));

            // Set up keyboard shortcuts for copy/paste
            let key_controller = gtk4::EventControllerKey::new();
            let vte_clone = self.vte.clone();
            key_controller.connect_key_pressed(move |_, key, _, modifier| {
                let ctrl = modifier.contains(gtk4::gdk::ModifierType::CONTROL_MASK);
                let shift = modifier.contains(gtk4::gdk::ModifierType::SHIFT_MASK);

                // Ctrl+Shift+V or Ctrl+V for paste
                if ctrl && (key == gtk4::gdk::Key::v || key == gtk4::gdk::Key::V) {
                    vte_clone.paste_clipboard();
                    return glib::Propagation::Stop;
                }

                // Ctrl+Shift+C for copy
                if ctrl && shift && (key == gtk4::gdk::Key::c || key == gtk4::gdk::Key::C) {
                    vte_clone.copy_clipboard_format(vte4::Format::Text);
                    return glib::Propagation::Stop;
                }

                glib::Propagation::Proceed
            });
            self.vte.add_controller(key_controller);
        }
    }

    impl WidgetImpl for TerminalView {
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            // Let the Box allocate VTE first so it recalculates columns/rows
            self.parent_size_allocate(width, height, baseline);

            // Force-sync PTY dimensions with VTE's actual column/row count.
            // This fires on every layout change (window resize, paned drag, etc.)
            // and ensures the shell always has the correct COLUMNS/LINES values.
            if let Some(pty) = self.vte.pty() {
                let rows = self.vte.row_count() as i32;
                let cols = self.vte.column_count() as i32;
                if cols > 0 && rows > 0 {
                    let _ = pty.set_size(rows, cols);
                }
            }
        }
    }
    impl BoxImpl for TerminalView {}
}

glib::wrapper! {
    pub struct TerminalView(ObjectSubclass<imp::TerminalView>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Orientable;
}

impl TerminalView {
    /// Create a new terminal with a local shell
    pub fn new_local() -> Self {
        let obj: Self = glib::Object::new();
        obj.spawn_local_shell();
        obj
    }

    /// Create a new terminal for an SSH connection
    pub fn new_ssh(session: Session) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();

        imp.is_ssh.replace(true);
        imp.session.replace(Some(session.clone()));

        // Show connecting message
        obj.feed_data(format!("Connecting to {}@{}:{}...\r\n",
            session.username, session.host, session.port).as_bytes());

        obj
    }

    /// Connect to SSH and start the terminal session
    pub fn connect_ssh(&self, password: Option<String>) {
        let imp = self.imp();

        let session = match imp.session.borrow().clone() {
            Some(s) => s,
            None => {
                self.feed_data(b"\r\nError: No session configured\r\n");
                return;
            }
        };

        let vte = imp.vte.clone();

        // Create SSH connection
        let mut ssh_conn = crate::ssh::SshConnection::new(session);
        let event_rx = ssh_conn.event_receiver();
        let command_tx = ssh_conn.command_sender();

        // Store the command sender for later use
        imp.command_sender.replace(Some(command_tx.clone()));

        // Connect VTE input to SSH
        let cmd_tx = command_tx.clone();
        imp.vte.connect_commit(move |_, text, _| {
            let data = text.as_bytes().to_vec();
            let tx = cmd_tx.clone();
            glib::spawn_future_local(async move {
                let _ = tx.send(SshCommand::SendData(data)).await;
            });
        });

        // Send initial terminal size after a short delay to ensure connection is ready
        let cmd_tx_init = command_tx.clone();
        let vte_init = imp.vte.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
            let cols = vte_init.column_count() as u32;
            let rows = vte_init.row_count() as u32;
            let tx = cmd_tx_init.clone();
            glib::spawn_future_local(async move {
                let _ = tx.send(SshCommand::Resize(cols, rows)).await;
            });
        });

        // Handle terminal resize using size-allocate signal
        let cmd_tx_resize = command_tx.clone();
        let vte_resize = imp.vte.clone();
        let last_size: std::rc::Rc<std::cell::Cell<(i64, i64)>> = std::rc::Rc::new(std::cell::Cell::new((0, 0)));
        imp.vte.connect_notify_local(Some("columns"), move |_, _| {
            let cols = vte_resize.column_count();
            let rows = vte_resize.row_count();
            let current = (cols, rows);
            if last_size.get() != current {
                last_size.set(current);
                let tx = cmd_tx_resize.clone();
                glib::spawn_future_local(async move {
                    let _ = tx.send(SshCommand::Resize(cols as u32, rows as u32)).await;
                });
            }
        });

        // Spawn SSH connection task on a tokio runtime (russh requires tokio)
        let password_clone = password.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            rt.block_on(async move {
                // Connect
                if let Err(e) = ssh_conn.connect(password_clone.as_deref()).await {
                    log::error!("SSH connection failed: {}", e);
                    return;
                }

                // Run the connection event loop
                let _ = ssh_conn.run().await;
            });
        });

        // Handle events from SSH in the main thread
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = terminal)]
            self,
            #[weak]
            vte,
            async move {
                while let Ok(event) = event_rx.recv().await {
                    match event {
                        SshEvent::Connected => {
                            log::info!("SSH connected");
                        }
                        SshEvent::Disconnected => {
                            vte.feed(b"\r\n[Connection closed]\r\n");
                            break;
                        }
                        SshEvent::Data(data) => {
                            vte.feed(&data);
                        }
                        SshEvent::Error(err) => {
                            vte.feed(format!("\r\n[Error: {}]\r\n", err).as_bytes());
                        }
                        SshEvent::SftpReady(sftp_client) => {
                            log::info!("SFTP client ready");
                            terminal.set_sftp_client(Some(sftp_client));
                        }
                    }
                }
            }
        ));
    }

    fn spawn_local_shell(&self) {
        let vte = self.imp().vte.clone();

        // Defer shell spawn until GTK has allocated the widget.
        // If we spawn immediately during constructed(), the PTY gets default 80x24
        // dimensions. The shell's readline then wraps commands at column 80 instead
        // of the actual terminal width, causing text overlap on long commands.
        glib::idle_add_local_once(move || {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
            let term_env = "TERM=xterm-256color";

            let vte_resize = vte.clone();
            vte.spawn_async(
                vte4::PtyFlags::DEFAULT,
                None,
                &[&shell],
                &[term_env],
                glib::SpawnFlags::DEFAULT,
                || {},
                -1,
                None::<&gtk4::gio::Cancellable>,
                move |result| {
                    match result {
                        Ok(_pid) => {
                            log::debug!("Shell spawned successfully");
                            // Force-sync PTY size with actual VTE dimensions.
                            // Even after idle, there can be a brief race where the PTY
                            // was created with stale dimensions.
                            if let Some(pty) = vte_resize.pty() {
                                let rows = vte_resize.row_count() as i32;
                                let cols = vte_resize.column_count() as i32;
                                let _ = pty.set_size(rows, cols);
                            }
                        }
                        Err(e) => log::error!("Failed to spawn shell: {}", e),
                    }
                },
            );
        });
    }

    pub fn get_sftp_client(&self) -> Option<Arc<SftpClient>> {
        self.imp().sftp_client.borrow().clone()
    }

    pub fn set_sftp_client(&self, client: Option<Arc<SftpClient>>) {
        let imp = self.imp();
        if let Some(ref sftp) = client {
            // Notify callback if set
            if let Some(callback) = imp.sftp_ready_callback.borrow().as_ref() {
                callback(sftp.clone());
            }
        }
        imp.sftp_client.replace(client);
    }

    /// Connect a callback to be called when SFTP becomes ready
    pub fn connect_sftp_ready<F: Fn(Arc<SftpClient>) + 'static>(&self, f: F) {
        self.imp().sftp_ready_callback.replace(Some(Box::new(f)));
    }

    pub fn feed_data(&self, data: &[u8]) {
        self.imp().vte.feed(data);
    }

    pub fn is_ssh(&self) -> bool {
        *self.imp().is_ssh.borrow()
    }

    pub fn get_session(&self) -> Option<Session> {
        self.imp().session.borrow().clone()
    }

    /// Send data to the terminal (for SSH connections)
    pub fn send_data(&self, data: &[u8]) {
        if let Some(tx) = self.imp().command_sender.borrow().as_ref() {
            let tx = tx.clone();
            let data = data.to_vec();
            glib::spawn_future_local(async move {
                let _ = tx.send(SshCommand::SendData(data)).await;
            });
        }
    }
}

impl Default for TerminalView {
    fn default() -> Self {
        Self::new_local()
    }
}
