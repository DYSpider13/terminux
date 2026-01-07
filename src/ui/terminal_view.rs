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

    #[derive(Debug)]
    pub struct TerminalView {
        pub vte: vte4::Terminal,
        pub sftp_client: RefCell<Option<Arc<SftpClient>>>,
        pub is_ssh: RefCell<bool>,
        pub session: RefCell<Option<Session>>,
        pub command_sender: RefCell<Option<Sender<SshCommand>>>,
    }

    impl Default for TerminalView {
        fn default() -> Self {
            Self {
                vte: vte4::Terminal::new(),
                sftp_client: RefCell::new(None),
                is_ssh: RefCell::new(false),
                session: RefCell::new(None),
                command_sender: RefCell::new(None),
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

            // Set colors (dark theme)
            let fg = gtk4::gdk::RGBA::parse("#e0e0e0").unwrap();
            let bg = gtk4::gdk::RGBA::parse("#1e1e1e").unwrap();

            let palette: [gtk4::gdk::RGBA; 16] = [
                gtk4::gdk::RGBA::parse("#1e1e1e").unwrap(), // Black
                gtk4::gdk::RGBA::parse("#f44747").unwrap(), // Red
                gtk4::gdk::RGBA::parse("#6a9955").unwrap(), // Green
                gtk4::gdk::RGBA::parse("#dcdcaa").unwrap(), // Yellow
                gtk4::gdk::RGBA::parse("#569cd6").unwrap(), // Blue
                gtk4::gdk::RGBA::parse("#c586c0").unwrap(), // Magenta
                gtk4::gdk::RGBA::parse("#4ec9b0").unwrap(), // Cyan
                gtk4::gdk::RGBA::parse("#d4d4d4").unwrap(), // White
                gtk4::gdk::RGBA::parse("#808080").unwrap(), // Bright Black
                gtk4::gdk::RGBA::parse("#f44747").unwrap(), // Bright Red
                gtk4::gdk::RGBA::parse("#6a9955").unwrap(), // Bright Green
                gtk4::gdk::RGBA::parse("#dcdcaa").unwrap(), // Bright Yellow
                gtk4::gdk::RGBA::parse("#569cd6").unwrap(), // Bright Blue
                gtk4::gdk::RGBA::parse("#c586c0").unwrap(), // Bright Magenta
                gtk4::gdk::RGBA::parse("#4ec9b0").unwrap(), // Bright Cyan
                gtk4::gdk::RGBA::parse("#e0e0e0").unwrap(), // Bright White
            ];

            let palette_refs: Vec<&gtk4::gdk::RGBA> = palette.iter().collect();
            self.vte.set_colors(Some(&fg), Some(&bg), &palette_refs);

            // Create scrolled window for terminal
            let scrolled = gtk4::ScrolledWindow::new();
            scrolled.set_child(Some(&self.vte));
            scrolled.set_vexpand(true);
            scrolled.set_hexpand(true);

            obj.append(&scrolled);

            // Connect terminal signals
            self.vte.connect_child_exited(glib::clone!(
                #[weak]
                obj,
                move |_, _| {
                    log::info!("Terminal child process exited");
                }
            ));
        }
    }

    impl WidgetImpl for TerminalView {}
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

        // Handle terminal resize
        let cmd_tx_resize = command_tx.clone();
        imp.vte.connect_resize_window(move |terminal, cols, rows| {
            let tx = cmd_tx_resize.clone();
            glib::spawn_future_local(async move {
                let _ = tx.send(SshCommand::Resize(cols as u32, rows as u32)).await;
            });
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
                    }
                }
            }
        ));
    }

    fn spawn_local_shell(&self) {
        let imp = self.imp();

        // Get user's shell
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());

        // Spawn the shell process
        let pty_flags = vte4::PtyFlags::DEFAULT;

        imp.vte.spawn_async(
            pty_flags,
            None,                    // working directory (None = current)
            &[&shell],               // command
            &[],                     // environment
            glib::SpawnFlags::DEFAULT,
            || {},                   // child setup
            -1,                      // timeout (-1 = default)
            None::<&gtk4::gio::Cancellable>,
            |result| {
                match result {
                    Ok(_pid) => log::debug!("Shell spawned successfully"),
                    Err(e) => log::error!("Failed to spawn shell: {}", e),
                }
            },
        );
    }

    pub fn get_sftp_client(&self) -> Option<Arc<SftpClient>> {
        self.imp().sftp_client.borrow().clone()
    }

    pub fn set_sftp_client(&self, client: Option<Arc<SftpClient>>) {
        self.imp().sftp_client.replace(client);
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
