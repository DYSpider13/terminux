use crate::storage::{AuthType, Session};
use async_channel::{Receiver, Sender};
use russh::client::{self, Config, Handle, Msg};
use russh::keys::key::PublicKey;
use russh::{Channel, ChannelId, ChannelMsg, Disconnect};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq)]
pub enum SshConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

/// Events sent from SSH to the UI
#[derive(Debug)]
pub enum SshEvent {
    Connected,
    Disconnected,
    Data(Vec<u8>),
    Error(String),
}

/// Commands sent from UI to SSH
#[derive(Debug)]
pub enum SshCommand {
    SendData(Vec<u8>),
    Resize(u32, u32),
    Disconnect,
}

/// SSH client handler for russh
struct ClientHandler {
    event_tx: Sender<SshEvent>,
}

#[async_trait::async_trait]
impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        // TODO: Implement proper host key verification
        // For now, accept all keys (NOT SECURE - for development only)
        log::warn!("Host key verification skipped - implement proper verification!");
        Ok(true)
    }
}

/// Represents an active SSH connection
pub struct SshConnection {
    session_info: Session,
    state: SshConnectionState,
    handle: Option<Handle<ClientHandler>>,
    channel: Option<Channel<Msg>>,
    event_tx: Sender<SshEvent>,
    event_rx: Receiver<SshEvent>,
    command_tx: Sender<SshCommand>,
    command_rx: Receiver<SshCommand>,
}

impl SshConnection {
    pub fn new(session: Session) -> Self {
        let (event_tx, event_rx) = async_channel::unbounded();
        let (command_tx, command_rx) = async_channel::unbounded();

        Self {
            session_info: session,
            state: SshConnectionState::Disconnected,
            handle: None,
            channel: None,
            event_tx,
            event_rx,
            command_tx,
            command_rx,
        }
    }

    /// Get the event receiver for UI updates
    pub fn event_receiver(&self) -> Receiver<SshEvent> {
        self.event_rx.clone()
    }

    /// Get the command sender for sending input
    pub fn command_sender(&self) -> Sender<SshCommand> {
        self.command_tx.clone()
    }

    /// Connect to the SSH server
    pub async fn connect(&mut self, password: Option<&str>) -> anyhow::Result<()> {
        self.state = SshConnectionState::Connecting;
        log::info!(
            "Connecting to {}@{}:{}",
            self.session_info.username,
            self.session_info.host,
            self.session_info.port
        );

        let config = Arc::new(Config::default());
        let addr = format!("{}:{}", self.session_info.host, self.session_info.port);

        let handler = ClientHandler {
            event_tx: self.event_tx.clone(),
        };

        // Connect to the server
        let mut session = match client::connect(config, &addr, handler).await {
            Ok(session) => session,
            Err(e) => {
                self.state = SshConnectionState::Error(e.to_string());
                let _ = self.event_tx.send(SshEvent::Error(e.to_string())).await;
                return Err(e.into());
            }
        };

        // Authenticate
        let auth_result = match &self.session_info.auth_type {
            AuthType::Password => {
                let pwd = password.unwrap_or("");
                session
                    .authenticate_password(&self.session_info.username, pwd)
                    .await
            }
            AuthType::Key => {
                if let Some(key_path) = &self.session_info.key_path {
                    let expanded_path = shellexpand::tilde(key_path);
                    match russh_keys::load_secret_key(&*expanded_path, None) {
                        Ok(key) => {
                            session
                                .authenticate_publickey(&self.session_info.username, Arc::new(key))
                                .await
                        }
                        Err(e) => {
                            self.state = SshConnectionState::Error(e.to_string());
                            let _ = self.event_tx.send(SshEvent::Error(e.to_string())).await;
                            return Err(anyhow::anyhow!("Failed to load key: {}", e));
                        }
                    }
                } else {
                    return Err(anyhow::anyhow!("Key path not specified"));
                }
            }
        };

        match auth_result {
            Ok(authenticated) => {
                if !authenticated {
                    self.state = SshConnectionState::Error("Authentication failed".to_string());
                    let _ = self
                        .event_tx
                        .send(SshEvent::Error("Authentication failed".to_string()))
                        .await;
                    return Err(anyhow::anyhow!("Authentication failed"));
                }
            }
            Err(e) => {
                self.state = SshConnectionState::Error(e.to_string());
                let _ = self.event_tx.send(SshEvent::Error(e.to_string())).await;
                return Err(e.into());
            }
        }

        // Open a PTY channel
        let channel = session.channel_open_session().await?;

        // Request PTY
        channel
            .request_pty(
                false,
                "xterm-256color",
                80,  // columns
                24,  // rows
                0,   // pixel width
                0,   // pixel height
                &[], // terminal modes
            )
            .await?;

        // Request shell
        channel.request_shell(false).await?;

        self.handle = Some(session);
        self.channel = Some(channel);
        self.state = SshConnectionState::Connected;

        let _ = self.event_tx.send(SshEvent::Connected).await;
        log::info!("SSH connection established successfully");

        Ok(())
    }

    /// Run the connection event loop (call this in a separate task)
    pub async fn run(&mut self) -> anyhow::Result<()> {
        let channel = self.channel.take();
        let mut channel = match channel {
            Some(c) => c,
            None => return Err(anyhow::anyhow!("No channel available")),
        };

        loop {
            tokio::select! {
                // Handle commands from UI
                cmd = self.command_rx.recv() => {
                    match cmd {
                        Ok(SshCommand::SendData(data)) => {
                            if let Err(e) = channel.data(&data[..]).await {
                                log::error!("Failed to send data: {}", e);
                                break;
                            }
                        }
                        Ok(SshCommand::Resize(cols, rows)) => {
                            if let Err(e) = channel.window_change(cols, rows, 0, 0).await {
                                log::error!("Failed to resize: {}", e);
                            }
                        }
                        Ok(SshCommand::Disconnect) => {
                            log::info!("Disconnect requested");
                            break;
                        }
                        Err(_) => {
                            log::info!("Command channel closed");
                            break;
                        }
                    }
                }
                // Handle channel messages
                msg = channel.wait() => {
                    match msg {
                        Some(ChannelMsg::Data { data }) => {
                            let _ = self.event_tx.send(SshEvent::Data(data.to_vec())).await;
                        }
                        Some(ChannelMsg::ExtendedData { data, .. }) => {
                            let _ = self.event_tx.send(SshEvent::Data(data.to_vec())).await;
                        }
                        Some(ChannelMsg::Eof) => {
                            log::info!("Channel EOF received");
                            break;
                        }
                        Some(ChannelMsg::Close) => {
                            log::info!("Channel closed");
                            break;
                        }
                        Some(ChannelMsg::ExitStatus { exit_status }) => {
                            log::info!("Exit status: {}", exit_status);
                        }
                        None => {
                            log::info!("Channel ended");
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        // Clean up
        self.state = SshConnectionState::Disconnected;
        let _ = self.event_tx.send(SshEvent::Disconnected).await;

        if let Some(handle) = self.handle.take() {
            let _ = handle
                .disconnect(Disconnect::ByApplication, "User disconnected", "en")
                .await;
        }

        Ok(())
    }

    /// Send data to the remote shell
    pub async fn send_data(&self, data: &[u8]) -> anyhow::Result<()> {
        self.command_tx
            .send(SshCommand::SendData(data.to_vec()))
            .await?;
        Ok(())
    }

    /// Resize the PTY
    pub async fn resize(&self, cols: u32, rows: u32) -> anyhow::Result<()> {
        self.command_tx.send(SshCommand::Resize(cols, rows)).await?;
        Ok(())
    }

    /// Disconnect from the server
    pub async fn disconnect(&self) -> anyhow::Result<()> {
        self.command_tx.send(SshCommand::Disconnect).await?;
        Ok(())
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        matches!(self.state, SshConnectionState::Connected)
    }

    /// Get current state
    pub fn state(&self) -> &SshConnectionState {
        &self.state
    }
}

/// Manages multiple SSH connections
pub struct ConnectionManager {
    connections: Arc<Mutex<std::collections::HashMap<String, Arc<Mutex<SshConnection>>>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Create a new connection for a session
    pub async fn create_connection(&self, session: Session) -> Arc<Mutex<SshConnection>> {
        let connection = Arc::new(Mutex::new(SshConnection::new(session.clone())));

        let mut connections = self.connections.lock().await;
        connections.insert(session.id.clone(), connection.clone());

        connection
    }

    /// Get an existing connection by session ID
    pub async fn get_connection(&self, session_id: &str) -> Option<Arc<Mutex<SshConnection>>> {
        let connections = self.connections.lock().await;
        connections.get(session_id).cloned()
    }

    /// Remove a connection
    pub async fn remove_connection(&self, session_id: &str) {
        let mut connections = self.connections.lock().await;
        connections.remove(session_id);
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
