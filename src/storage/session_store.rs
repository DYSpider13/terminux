use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthType {
    Password,
    Key,
}

impl Default for AuthType {
    fn default() -> Self {
        AuthType::Password
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: AuthType,
    pub key_path: Option<String>,
    pub folder_id: Option<String>,
    pub auto_connect: bool,
    // Advanced SSH options
    pub jump_host: Option<String>,
    pub agent_forwarding: bool,
    pub port_forward_local: Option<u16>,
    pub port_forward_remote: Option<String>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            host: String::new(),
            port: 22,
            username: String::new(),
            auth_type: AuthType::Password,
            key_path: None,
            folder_id: None,
            auto_connect: false,
            jump_host: None,
            agent_forwarding: false,
            port_forward_local: None,
            port_forward_remote: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub sort_order: i32,
}

impl Default for Folder {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            parent_id: None,
            sort_order: 0,
        }
    }
}

/// Session store for CRUD operations on sessions
pub struct SessionStore {
    db: super::Database,
}

impl SessionStore {
    pub fn new(db: super::Database) -> Self {
        Self { db }
    }

    pub fn get_all_sessions(&self) -> anyhow::Result<Vec<Session>> {
        self.db.get_all_sessions()
    }

    pub fn get_session(&self, id: &str) -> anyhow::Result<Option<Session>> {
        self.db.get_session(id)
    }

    pub fn create_session(&self, session: &Session) -> anyhow::Result<()> {
        self.db.insert_session(session)
    }

    pub fn update_session(&self, session: &Session) -> anyhow::Result<()> {
        self.db.update_session(session)
    }

    pub fn delete_session(&self, id: &str) -> anyhow::Result<()> {
        self.db.delete_session(id)
    }

    pub fn get_all_folders(&self) -> anyhow::Result<Vec<Folder>> {
        self.db.get_all_folders()
    }

    pub fn create_folder(&self, folder: &Folder) -> anyhow::Result<()> {
        self.db.insert_folder(folder)
    }

    pub fn delete_folder(&self, id: &str) -> anyhow::Result<()> {
        self.db.delete_folder(id)
    }
}
