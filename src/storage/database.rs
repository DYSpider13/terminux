use super::session_store::{AuthType, Folder, Session};
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::PathBuf;

pub struct Database {
    conn: Connection,
}

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Database").finish()
    }
}

impl Database {
    pub fn new() -> anyhow::Result<Self> {
        let db_path = Self::get_db_path()?;

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;
        let db = Self { conn };
        db.initialize_schema()?;

        Ok(db)
    }

    pub fn new_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.initialize_schema()?;
        Ok(db)
    }

    fn get_db_path() -> anyhow::Result<PathBuf> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find data directory"))?;

        Ok(data_dir.join("terminux").join("sessions.db"))
    }

    fn initialize_schema(&self) -> SqliteResult<()> {
        self.conn.execute_batch(
            r#"
            -- Sessions table
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                host TEXT NOT NULL,
                port INTEGER DEFAULT 22,
                username TEXT NOT NULL,
                auth_type TEXT NOT NULL,
                key_path TEXT,
                folder_id TEXT,
                auto_connect INTEGER DEFAULT 0,
                jump_host TEXT,
                agent_forwarding INTEGER DEFAULT 0,
                port_forward_local INTEGER,
                port_forward_remote TEXT,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                last_connected TEXT,
                FOREIGN KEY (folder_id) REFERENCES folders(id)
            );

            -- Folders table
            CREATE TABLE IF NOT EXISTS folders (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                parent_id TEXT,
                sort_order INTEGER DEFAULT 0,
                FOREIGN KEY (parent_id) REFERENCES folders(id)
            );

            -- Connection history table
            CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                connected_at TEXT DEFAULT CURRENT_TIMESTAMP,
                disconnected_at TEXT,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );

            -- Create indexes
            CREATE INDEX IF NOT EXISTS idx_sessions_folder ON sessions(folder_id);
            CREATE INDEX IF NOT EXISTS idx_history_session ON history(session_id);
            "#,
        )
    }

    // Session operations

    pub fn get_all_sessions(&self) -> anyhow::Result<Vec<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, host, port, username, auth_type, key_path, folder_id,
                    auto_connect, jump_host, agent_forwarding, port_forward_local, port_forward_remote
             FROM sessions ORDER BY name",
        )?;

        let sessions = stmt.query_map([], |row| {
            let auth_type_str: String = row.get(5)?;
            let auth_type = match auth_type_str.as_str() {
                "Key" => AuthType::Key,
                _ => AuthType::Password,
            };

            Ok(Session {
                id: row.get(0)?,
                name: row.get(1)?,
                host: row.get(2)?,
                port: row.get(3)?,
                username: row.get(4)?,
                auth_type,
                key_path: row.get(6)?,
                folder_id: row.get(7)?,
                auto_connect: row.get::<_, i32>(8)? != 0,
                jump_host: row.get(9)?,
                agent_forwarding: row.get::<_, i32>(10)? != 0,
                port_forward_local: row.get(11)?,
                port_forward_remote: row.get(12)?,
            })
        })?;

        Ok(sessions.filter_map(|s| s.ok()).collect())
    }

    pub fn get_session(&self, id: &str) -> anyhow::Result<Option<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, host, port, username, auth_type, key_path, folder_id,
                    auto_connect, jump_host, agent_forwarding, port_forward_local, port_forward_remote
             FROM sessions WHERE id = ?",
        )?;

        let session = stmt.query_row([id], |row| {
            let auth_type_str: String = row.get(5)?;
            let auth_type = match auth_type_str.as_str() {
                "Key" => AuthType::Key,
                _ => AuthType::Password,
            };

            Ok(Session {
                id: row.get(0)?,
                name: row.get(1)?,
                host: row.get(2)?,
                port: row.get(3)?,
                username: row.get(4)?,
                auth_type,
                key_path: row.get(6)?,
                folder_id: row.get(7)?,
                auto_connect: row.get::<_, i32>(8)? != 0,
                jump_host: row.get(9)?,
                agent_forwarding: row.get::<_, i32>(10)? != 0,
                port_forward_local: row.get(11)?,
                port_forward_remote: row.get(12)?,
            })
        });

        match session {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn insert_session(&self, session: &Session) -> anyhow::Result<()> {
        let auth_type_str = match session.auth_type {
            AuthType::Password => "Password",
            AuthType::Key => "Key",
        };

        self.conn.execute(
            "INSERT INTO sessions (id, name, host, port, username, auth_type, key_path, folder_id,
                                   auto_connect, jump_host, agent_forwarding, port_forward_local, port_forward_remote)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                session.id,
                session.name,
                session.host,
                session.port,
                session.username,
                auth_type_str,
                session.key_path,
                session.folder_id,
                session.auto_connect as i32,
                session.jump_host,
                session.agent_forwarding as i32,
                session.port_forward_local,
                session.port_forward_remote,
            ],
        )?;

        Ok(())
    }

    pub fn update_session(&self, session: &Session) -> anyhow::Result<()> {
        let auth_type_str = match session.auth_type {
            AuthType::Password => "Password",
            AuthType::Key => "Key",
        };

        self.conn.execute(
            "UPDATE sessions SET name = ?, host = ?, port = ?, username = ?, auth_type = ?,
                                 key_path = ?, folder_id = ?, auto_connect = ?, jump_host = ?,
                                 agent_forwarding = ?, port_forward_local = ?, port_forward_remote = ?
             WHERE id = ?",
            params![
                session.name,
                session.host,
                session.port,
                session.username,
                auth_type_str,
                session.key_path,
                session.folder_id,
                session.auto_connect as i32,
                session.jump_host,
                session.agent_forwarding as i32,
                session.port_forward_local,
                session.port_forward_remote,
                session.id,
            ],
        )?;

        Ok(())
    }

    pub fn delete_session(&self, id: &str) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM sessions WHERE id = ?", [id])?;
        Ok(())
    }

    pub fn update_last_connected(&self, session_id: &str) -> anyhow::Result<()> {
        self.conn.execute(
            "UPDATE sessions SET last_connected = CURRENT_TIMESTAMP WHERE id = ?",
            [session_id],
        )?;
        Ok(())
    }

    // Folder operations

    pub fn get_all_folders(&self) -> anyhow::Result<Vec<Folder>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, parent_id, sort_order FROM folders ORDER BY sort_order, name",
        )?;

        let folders = stmt.query_map([], |row| {
            Ok(Folder {
                id: row.get(0)?,
                name: row.get(1)?,
                parent_id: row.get(2)?,
                sort_order: row.get(3)?,
            })
        })?;

        Ok(folders.filter_map(|f| f.ok()).collect())
    }

    pub fn insert_folder(&self, folder: &Folder) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO folders (id, name, parent_id, sort_order) VALUES (?, ?, ?, ?)",
            params![folder.id, folder.name, folder.parent_id, folder.sort_order],
        )?;

        Ok(())
    }

    pub fn delete_folder(&self, id: &str) -> anyhow::Result<()> {
        // Move sessions in this folder to no folder
        self.conn.execute(
            "UPDATE sessions SET folder_id = NULL WHERE folder_id = ?",
            [id],
        )?;

        // Delete subfolders recursively
        self.conn.execute(
            "DELETE FROM folders WHERE parent_id = ?",
            [id],
        )?;

        // Delete the folder
        self.conn.execute("DELETE FROM folders WHERE id = ?", [id])?;

        Ok(())
    }

    // History operations

    pub fn record_connection(&self, session_id: &str) -> anyhow::Result<i64> {
        self.conn.execute(
            "INSERT INTO history (session_id) VALUES (?)",
            [session_id],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn record_disconnection(&self, history_id: i64) -> anyhow::Result<()> {
        self.conn.execute(
            "UPDATE history SET disconnected_at = CURRENT_TIMESTAMP WHERE id = ?",
            [history_id],
        )?;

        Ok(())
    }
}
