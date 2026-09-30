use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::types::Value as SqlValue;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Row};

use crate::error::AppResult;

// ------------------------------------------------------------------ 数据模型

#[derive(Debug, Clone)]
pub struct UserRow {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: String,
    pub disabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_login_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct NetworkRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub network_name: String,
    pub network_secret_enc: String,
    pub ipv4: String,
    pub dhcp: bool,
    pub hostname: String,
    pub instance_name: String,
    pub listeners: String,
    pub mapped_listeners: String,
    pub peers: String,
    pub external_node: Option<String>,
    pub listen_port: i64,
    pub rpc_port: i64,
    pub secure_mode: bool,
    pub local_private_key_enc: Option<String>,
    pub credential_file: Option<String>,
    pub flags: String,
    pub auto_start: bool,
    pub status: String,
    pub last_error: Option<String>,
    pub owner_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct NetworkShareRow {
    pub network_id: String,
    pub user_id: String,
    pub permission: String,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct NodeRow {
    pub id: String,
    pub network_id: String,
    pub name: String,
    pub hostname: String,
    pub ipv4: Option<String>,
    pub r#type: String,
    pub description: Option<String>,
    pub listeners: String,
    pub proxy_networks: String,
    pub flags: String,
    pub credential_id: Option<String>,
    pub peer_id: Option<i64>,
    pub status: String,
    pub last_seen_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct CredentialRow {
    pub id: String,
    pub network_id: String,
    pub node_id: Option<String>,
    pub credential_id: String,
    pub secret_enc: String,
    pub groups: String,
    pub allow_relay: bool,
    pub reusable: bool,
    pub allowed_proxy_cidrs: String,
    pub ttl_seconds: i64,
    pub expires_at: i64,
    pub revoked: bool,
    pub created_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct NodeLogRow {
    pub id: i64,
    pub network_id: Option<String>,
    pub node_id: Option<String>,
    pub source: String,
    pub level: String,
    pub message: String,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct AuditLogRow {
    pub id: i64,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub detail: Option<String>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: i64,
}

pub struct NewNodeLog {
    pub network_id: Option<String>,
    pub node_id: Option<String>,
    pub source: String,
    pub level: String,
    pub message: String,
    pub created_at: i64,
}

// ------------------------------------------------------------------ 列映射

fn map_user(row: &Row<'_>) -> rusqlite::Result<UserRow> {
    Ok(UserRow {
        id: row.get("id")?,
        username: row.get("username")?,
        password_hash: row.get("password_hash")?,
        display_name: row.get("display_name")?,
        email: row.get("email")?,
        role: row.get("role")?,
        disabled: row.get("disabled")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        last_login_at: row.get("last_login_at")?,
    })
}

fn map_network(row: &Row<'_>) -> rusqlite::Result<NetworkRow> {
    Ok(NetworkRow {
        id: row.get("id")?,
        name: row.get("name")?,
        description: row.get("description")?,
        network_name: row.get("network_name")?,
        network_secret_enc: row.get("network_secret_enc")?,
        ipv4: row.get("ipv4")?,
        dhcp: row.get("dhcp")?,
        hostname: row.get("hostname")?,
        instance_name: row.get("instance_name")?,
        listeners: row.get("listeners")?,
        mapped_listeners: row.get("mapped_listeners")?,
        peers: row.get("peers")?,
        external_node: row.get("external_node")?,
        listen_port: row.get("listen_port")?,
        rpc_port: row.get("rpc_port")?,
        secure_mode: row.get("secure_mode")?,
        local_private_key_enc: row.get("local_private_key_enc")?,
        credential_file: row.get("credential_file")?,
        flags: row.get("flags")?,
        auto_start: row.get("auto_start")?,
        status: row.get("status")?,
        last_error: row.get("last_error")?,
        owner_id: row.get("owner_id")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn map_share(row: &Row<'_>) -> rusqlite::Result<NetworkShareRow> {
    Ok(NetworkShareRow {
        network_id: row.get("network_id")?,
        user_id: row.get("user_id")?,
        permission: row.get("permission")?,
        created_at: row.get("created_at")?,
    })
}

fn map_node(row: &Row<'_>) -> rusqlite::Result<NodeRow> {
    Ok(NodeRow {
        id: row.get("id")?,
        network_id: row.get("network_id")?,
        name: row.get("name")?,
        hostname: row.get("hostname")?,
        ipv4: row.get("ipv4")?,
        r#type: row.get("type")?,
        description: row.get("description")?,
        listeners: row.get("listeners")?,
        proxy_networks: row.get("proxy_networks")?,
        flags: row.get("flags")?,
        credential_id: row.get("credential_id")?,
        peer_id: row.get("peer_id")?,
        status: row.get("status")?,
        last_seen_at: row.get("last_seen_at")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn map_credential(row: &Row<'_>) -> rusqlite::Result<CredentialRow> {
    Ok(CredentialRow {
        id: row.get("id")?,
        network_id: row.get("network_id")?,
        node_id: row.get("node_id")?,
        credential_id: row.get("credential_id")?,
        secret_enc: row.get("secret_enc")?,
        groups: row.get("groups")?,
        allow_relay: row.get("allow_relay")?,
        reusable: row.get("reusable")?,
        allowed_proxy_cidrs: row.get("allowed_proxy_cidrs")?,
        ttl_seconds: row.get("ttl_seconds")?,
        expires_at: row.get("expires_at")?,
        revoked: row.get("revoked")?,
        created_by: row.get("created_by")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn map_node_log(row: &Row<'_>) -> rusqlite::Result<NodeLogRow> {
    Ok(NodeLogRow {
        id: row.get("id")?,
        network_id: row.get("network_id")?,
        node_id: row.get("node_id")?,
        source: row.get("source")?,
        level: row.get("level")?,
        message: row.get("message")?,
        created_at: row.get("created_at")?,
    })
}

fn map_audit_log(row: &Row<'_>) -> rusqlite::Result<AuditLogRow> {
    Ok(AuditLogRow {
        id: row.get("id")?,
        user_id: row.get("user_id")?,
        username: row.get("username")?,
        action: row.get("action")?,
        resource_type: row.get("resource_type")?,
        resource_id: row.get("resource_id")?,
        detail: row.get("detail")?,
        ip: row.get("ip")?,
        user_agent: row.get("user_agent")?,
        created_at: row.get("created_at")?,
    })
}

// ------------------------------------------------------------------ Db

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> anyhow::Result<Db> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;",
        )?;
        Ok(Db {
            conn: Mutex::new(conn),
        })
    }

    pub fn lock(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 幂等建表，与 drizzle schema 保持一致。
    pub fn ensure_schema(&self) -> AppResult<()> {
        let conn = self.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS users (
              id TEXT PRIMARY KEY,
              username TEXT NOT NULL UNIQUE,
              password_hash TEXT NOT NULL,
              display_name TEXT,
              email TEXT,
              role TEXT NOT NULL DEFAULT 'admin',
              disabled INTEGER NOT NULL DEFAULT 0,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              last_login_at INTEGER
            );

            CREATE TABLE IF NOT EXISTS networks (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              description TEXT,
              network_name TEXT NOT NULL,
              network_secret_enc TEXT NOT NULL,
              ipv4 TEXT NOT NULL DEFAULT '10.126.126.1/24',
              dhcp INTEGER NOT NULL DEFAULT 0,
              hostname TEXT NOT NULL,
              instance_name TEXT NOT NULL,
              listeners TEXT NOT NULL DEFAULT '[]',
              mapped_listeners TEXT NOT NULL DEFAULT '[]',
              peers TEXT NOT NULL DEFAULT '[]',
              external_node TEXT,
              listen_port INTEGER NOT NULL,
              rpc_port INTEGER NOT NULL,
              secure_mode INTEGER NOT NULL DEFAULT 1,
              local_private_key_enc TEXT,
              credential_file TEXT,
              flags TEXT NOT NULL DEFAULT '{}',
              auto_start INTEGER NOT NULL DEFAULT 0,
              status TEXT NOT NULL DEFAULT 'stopped',
              last_error TEXT,
              owner_id TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS network_shares (
              network_id TEXT NOT NULL,
              user_id TEXT NOT NULL,
              permission TEXT NOT NULL DEFAULT 'view',
              created_at INTEGER NOT NULL,
              PRIMARY KEY (network_id, user_id)
            );
            CREATE INDEX IF NOT EXISTS network_shares_user_idx ON network_shares(user_id);

            CREATE TABLE IF NOT EXISTS nodes (
              id TEXT PRIMARY KEY,
              network_id TEXT NOT NULL,
              name TEXT NOT NULL,
              hostname TEXT NOT NULL,
              ipv4 TEXT,
              type TEXT NOT NULL DEFAULT 'client',
              description TEXT,
              listeners TEXT NOT NULL DEFAULT '[]',
              proxy_networks TEXT NOT NULL DEFAULT '[]',
              flags TEXT NOT NULL DEFAULT '{}',
              credential_id TEXT,
              peer_id INTEGER,
              status TEXT NOT NULL DEFAULT 'unknown',
              last_seen_at INTEGER,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS nodes_network_idx ON nodes(network_id);

            CREATE TABLE IF NOT EXISTS credentials (
              id TEXT PRIMARY KEY,
              network_id TEXT NOT NULL,
              node_id TEXT,
              credential_id TEXT NOT NULL,
              secret_enc TEXT NOT NULL,
              groups TEXT NOT NULL DEFAULT '[]',
              allow_relay INTEGER NOT NULL DEFAULT 0,
              reusable INTEGER NOT NULL DEFAULT 1,
              allowed_proxy_cidrs TEXT NOT NULL DEFAULT '[]',
              ttl_seconds INTEGER NOT NULL,
              expires_at INTEGER NOT NULL,
              revoked INTEGER NOT NULL DEFAULT 0,
              created_by TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS credentials_network_idx ON credentials(network_id);

            CREATE TABLE IF NOT EXISTS node_logs (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              network_id TEXT,
              node_id TEXT,
              source TEXT NOT NULL DEFAULT 'core',
              level TEXT NOT NULL DEFAULT 'info',
              message TEXT NOT NULL,
              created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS node_logs_network_idx ON node_logs(network_id);
            CREATE INDEX IF NOT EXISTS node_logs_created_idx ON node_logs(created_at);

            CREATE TABLE IF NOT EXISTS audit_logs (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              user_id TEXT,
              username TEXT,
              action TEXT NOT NULL,
              resource_type TEXT,
              resource_id TEXT,
              detail TEXT,
              ip TEXT,
              user_agent TEXT,
              created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS audit_logs_created_idx ON audit_logs(created_at);

            CREATE TABLE IF NOT EXISTS settings (
              key TEXT PRIMARY KEY,
              value TEXT NOT NULL,
              updated_at INTEGER NOT NULL
            );

            UPDATE nodes SET credential_id = NULL
            WHERE credential_id IS NOT NULL
              AND credential_id NOT IN (SELECT id FROM credentials);
            "#,
        )?;

        // 兼容旧库：为 nodes 补充 listeners 列
        let has_listeners = {
            let mut stmt = conn.prepare("PRAGMA table_info(nodes)")?;
            let names = stmt
                .query_map([], |row| row.get::<_, String>("name"))?
                .collect::<rusqlite::Result<Vec<String>>>()?;
            names.iter().any(|n| n == "listeners")
        };
        if !has_listeners {
            conn.execute_batch(
                "ALTER TABLE nodes ADD COLUMN listeners TEXT NOT NULL DEFAULT '[]'",
            )?;
        }

        // 兼容旧库：为 networks 补充 owner_id 列
        let has_owner = {
            let mut stmt = conn.prepare("PRAGMA table_info(networks)")?;
            let names = stmt
                .query_map([], |row| row.get::<_, String>("name"))?
                .collect::<rusqlite::Result<Vec<String>>>()?;
            names.iter().any(|n| n == "owner_id")
        };
        if !has_owner {
            conn.execute_batch("ALTER TABLE networks ADD COLUMN owner_id TEXT")?;
        }
        Ok(())
    }

    // -------------------------------------------------------------- users

    pub fn count_users(&self) -> AppResult<i64> {
        let conn = self.lock();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?;
        Ok(n)
    }

    pub fn count_active_admins(&self) -> AppResult<i64> {
        let conn = self.lock();
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM users WHERE role = 'admin' AND disabled = 0",
            [],
            |r| r.get(0),
        )?;
        Ok(n)
    }

    pub fn list_users(&self) -> AppResult<Vec<UserRow>> {
        let conn = self.lock();
        let mut stmt = conn.prepare("SELECT * FROM users ORDER BY created_at DESC")?;
        let rows = stmt
            .query_map([], map_user)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn get_user_by_id(&self, id: &str) -> AppResult<Option<UserRow>> {
        let conn = self.lock();
        let row = conn
            .query_row("SELECT * FROM users WHERE id = ?1", params![id], map_user)
            .optional()?;
        Ok(row)
    }

    pub fn get_user_by_username(&self, username: &str) -> AppResult<Option<UserRow>> {
        let conn = self.lock();
        let row = conn
            .query_row(
                "SELECT * FROM users WHERE username = ?1",
                params![username],
                map_user,
            )
            .optional()?;
        Ok(row)
    }

    pub fn insert_user(&self, u: &UserRow) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, email, role, disabled, created_at, updated_at, last_login_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                u.id,
                u.username,
                u.password_hash,
                u.display_name,
                u.email,
                u.role,
                u.disabled,
                u.created_at,
                u.updated_at,
                u.last_login_at
            ],
        )?;
        Ok(())
    }

    pub fn update_user_profile(
        &self,
        id: &str,
        display_name: Option<&str>,
        email: Option<&str>,
        role: &str,
        disabled: bool,
        updated_at: i64,
    ) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE users SET display_name=?2, email=?3, role=?4, disabled=?5, updated_at=?6 WHERE id=?1",
            params![id, display_name, email, role, disabled, updated_at],
        )?;
        Ok(())
    }

    pub fn update_user_password(&self, id: &str, hash: &str, updated_at: i64) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE users SET password_hash=?2, updated_at=?3 WHERE id=?1",
            params![id, hash, updated_at],
        )?;
        Ok(())
    }

    pub fn update_user_last_login(&self, id: &str, ts: i64) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE users SET last_login_at=?2, updated_at=?2 WHERE id=?1",
            params![id, ts],
        )?;
        Ok(())
    }

    pub fn delete_user(&self, id: &str) -> AppResult<()> {
        let conn = self.lock();
        conn.execute("DELETE FROM users WHERE id=?1", params![id])?;
        Ok(())
    }

    // -------------------------------------------------------------- networks

    pub fn list_networks(&self) -> AppResult<Vec<NetworkRow>> {
        let conn = self.lock();
        let mut stmt = conn.prepare("SELECT * FROM networks ORDER BY created_at DESC")?;
        let rows = stmt
            .query_map([], map_network)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// 非管理员可见的网络：自己创建的 + 被分享的
    pub fn list_networks_for_user(&self, user_id: &str) -> AppResult<Vec<NetworkRow>> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT * FROM networks WHERE owner_id = ?1 OR id IN (SELECT network_id FROM network_shares WHERE user_id = ?1) ORDER BY created_at DESC",
        )?;
        let rows = stmt
            .query_map(params![user_id], map_network)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn list_share_permissions_for_user(
        &self,
        user_id: &str,
    ) -> AppResult<Vec<(String, String)>> {
        let conn = self.lock();
        let mut stmt =
            conn.prepare("SELECT network_id, permission FROM network_shares WHERE user_id = ?1")?;
        let rows = stmt
            .query_map(params![user_id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn get_share_permission(
        &self,
        network_id: &str,
        user_id: &str,
    ) -> AppResult<Option<String>> {
        let conn = self.lock();
        let permission = conn
            .query_row(
                "SELECT permission FROM network_shares WHERE network_id=?1 AND user_id=?2",
                params![network_id, user_id],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        Ok(permission)
    }

    pub fn list_shares(&self, network_id: &str) -> AppResult<Vec<NetworkShareRow>> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare("SELECT * FROM network_shares WHERE network_id=?1 ORDER BY created_at ASC")?;
        let rows = stmt
            .query_map(params![network_id], map_share)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn upsert_share(
        &self,
        network_id: &str,
        user_id: &str,
        permission: &str,
        created_at: i64,
    ) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO network_shares (network_id, user_id, permission, created_at) VALUES (?1,?2,?3,?4)
             ON CONFLICT(network_id, user_id) DO UPDATE SET permission=excluded.permission",
            params![network_id, user_id, permission, created_at],
        )?;
        Ok(())
    }

    pub fn delete_share(&self, network_id: &str, user_id: &str) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "DELETE FROM network_shares WHERE network_id=?1 AND user_id=?2",
            params![network_id, user_id],
        )?;
        Ok(())
    }

    pub fn delete_shares_by_network(&self, network_id: &str) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "DELETE FROM network_shares WHERE network_id=?1",
            params![network_id],
        )?;
        Ok(())
    }

    pub fn delete_shares_by_user(&self, user_id: &str) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "DELETE FROM network_shares WHERE user_id=?1",
            params![user_id],
        )?;
        Ok(())
    }

    pub fn list_auto_start_networks(&self) -> AppResult<Vec<NetworkRow>> {
        let conn = self.lock();
        let mut stmt = conn.prepare("SELECT * FROM networks WHERE auto_start = 1")?;
        let rows = stmt
            .query_map([], map_network)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn get_network(&self, id: &str) -> AppResult<Option<NetworkRow>> {
        let conn = self.lock();
        let row = conn
            .query_row(
                "SELECT * FROM networks WHERE id = ?1",
                params![id],
                map_network,
            )
            .optional()?;
        Ok(row)
    }

    pub fn insert_network(&self, n: &NetworkRow) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO networks (id, name, description, network_name, network_secret_enc, ipv4, dhcp, hostname, instance_name, listeners, mapped_listeners, peers, external_node, listen_port, rpc_port, secure_mode, local_private_key_enc, credential_file, flags, auto_start, status, last_error, owner_id, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25)",
            params![
                n.id, n.name, n.description, n.network_name, n.network_secret_enc, n.ipv4, n.dhcp,
                n.hostname, n.instance_name, n.listeners, n.mapped_listeners, n.peers, n.external_node,
                n.listen_port, n.rpc_port, n.secure_mode, n.local_private_key_enc, n.credential_file,
                n.flags, n.auto_start, n.status, n.last_error, n.owner_id, n.created_at, n.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn update_network(&self, n: &NetworkRow) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE networks SET name=?2, description=?3, network_name=?4, network_secret_enc=?5, ipv4=?6, dhcp=?7, hostname=?8, instance_name=?9, listeners=?10, mapped_listeners=?11, peers=?12, external_node=?13, listen_port=?14, rpc_port=?15, secure_mode=?16, local_private_key_enc=?17, credential_file=?18, flags=?19, auto_start=?20, status=?21, last_error=?22, owner_id=?23, updated_at=?24 WHERE id=?1",
            params![
                n.id, n.name, n.description, n.network_name, n.network_secret_enc, n.ipv4, n.dhcp,
                n.hostname, n.instance_name, n.listeners, n.mapped_listeners, n.peers, n.external_node,
                n.listen_port, n.rpc_port, n.secure_mode, n.local_private_key_enc, n.credential_file,
                n.flags, n.auto_start, n.status, n.last_error, n.owner_id, n.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn delete_network(&self, id: &str) -> AppResult<()> {
        let conn = self.lock();
        conn.execute("DELETE FROM networks WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn set_network_status(
        &self,
        id: &str,
        status: &str,
        last_error: Option<&str>,
        updated_at: i64,
    ) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE networks SET status=?2, last_error=?3, updated_at=?4 WHERE id=?1",
            params![id, status, last_error, updated_at],
        )?;
        Ok(())
    }

    pub fn list_network_ports(&self) -> AppResult<Vec<(i64, i64)>> {
        let conn = self.lock();
        let mut stmt = conn.prepare("SELECT listen_port, rpc_port FROM networks")?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    // -------------------------------------------------------------- nodes

    pub fn list_nodes(&self, network_id: Option<&str>) -> AppResult<Vec<NodeRow>> {
        let conn = self.lock();
        match network_id {
            Some(nid) => {
                let mut stmt = conn
                    .prepare("SELECT * FROM nodes WHERE network_id=?1 ORDER BY created_at DESC")?;
                let rows = stmt
                    .query_map(params![nid], map_node)?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(rows)
            }
            None => {
                let mut stmt = conn.prepare("SELECT * FROM nodes ORDER BY created_at DESC")?;
                let rows = stmt
                    .query_map([], map_node)?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(rows)
            }
        }
    }

    pub fn get_node(&self, id: &str) -> AppResult<Option<NodeRow>> {
        let conn = self.lock();
        let row = conn
            .query_row("SELECT * FROM nodes WHERE id = ?1", params![id], map_node)
            .optional()?;
        Ok(row)
    }

    pub fn insert_node(&self, n: &NodeRow) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO nodes (id, network_id, name, hostname, ipv4, type, description, listeners, proxy_networks, flags, credential_id, peer_id, status, last_seen_at, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            params![
                n.id, n.network_id, n.name, n.hostname, n.ipv4, n.r#type, n.description, n.listeners,
                n.proxy_networks, n.flags, n.credential_id, n.peer_id, n.status, n.last_seen_at,
                n.created_at, n.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn update_node(&self, n: &NodeRow) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE nodes SET name=?2, hostname=?3, ipv4=?4, description=?5, listeners=?6, proxy_networks=?7, flags=?8, credential_id=?9, peer_id=?10, status=?11, last_seen_at=?12, updated_at=?13 WHERE id=?1",
            params![
                n.id, n.name, n.hostname, n.ipv4, n.description, n.listeners, n.proxy_networks,
                n.flags, n.credential_id, n.peer_id, n.status, n.last_seen_at, n.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn delete_node(&self, id: &str) -> AppResult<()> {
        let conn = self.lock();
        conn.execute("DELETE FROM nodes WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn list_node_ipv4s(&self, network_id: &str) -> AppResult<Vec<Option<String>>> {
        let conn = self.lock();
        let mut stmt = conn.prepare("SELECT ipv4 FROM nodes WHERE network_id=?1")?;
        let rows = stmt
            .query_map(params![network_id], |r| r.get::<_, Option<String>>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn set_node_runtime(
        &self,
        id: &str,
        status: &str,
        last_seen_at: Option<i64>,
        peer_id: Option<i64>,
    ) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE nodes SET status=?2, last_seen_at=COALESCE(?3, last_seen_at), peer_id=COALESCE(?4, peer_id) WHERE id=?1",
            params![id, status, last_seen_at, peer_id],
        )?;
        Ok(())
    }

    pub fn update_node_credential(
        &self,
        id: &str,
        credential_id: Option<&str>,
        updated_at: i64,
    ) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE nodes SET credential_id=?2, updated_at=?3 WHERE id=?1",
            params![id, credential_id, updated_at],
        )?;
        Ok(())
    }

    pub fn clear_node_credential_refs(
        &self,
        credential_id: &str,
        updated_at: i64,
    ) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE nodes SET credential_id = NULL, updated_at=?2 WHERE credential_id=?1",
            params![credential_id, updated_at],
        )?;
        Ok(())
    }

    pub fn delete_nodes_by_network(&self, network_id: &str) -> AppResult<()> {
        let conn = self.lock();
        conn.execute("DELETE FROM nodes WHERE network_id=?1", params![network_id])?;
        Ok(())
    }

    // -------------------------------------------------------------- credentials

    pub fn list_credentials(&self, network_id: Option<&str>) -> AppResult<Vec<CredentialRow>> {
        let conn = self.lock();
        match network_id {
            Some(nid) => {
                let mut stmt = conn.prepare(
                    "SELECT * FROM credentials WHERE network_id=?1 ORDER BY created_at DESC",
                )?;
                let rows = stmt
                    .query_map(params![nid], map_credential)?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(rows)
            }
            None => {
                let mut stmt =
                    conn.prepare("SELECT * FROM credentials ORDER BY created_at DESC")?;
                let rows = stmt
                    .query_map([], map_credential)?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(rows)
            }
        }
    }

    pub fn get_credential(&self, id: &str) -> AppResult<Option<CredentialRow>> {
        let conn = self.lock();
        let row = conn
            .query_row(
                "SELECT * FROM credentials WHERE id = ?1",
                params![id],
                map_credential,
            )
            .optional()?;
        Ok(row)
    }

    pub fn insert_credential(&self, c: &CredentialRow) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO credentials (id, network_id, node_id, credential_id, secret_enc, groups, allow_relay, reusable, allowed_proxy_cidrs, ttl_seconds, expires_at, revoked, created_by, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                c.id, c.network_id, c.node_id, c.credential_id, c.secret_enc, c.groups, c.allow_relay,
                c.reusable, c.allowed_proxy_cidrs, c.ttl_seconds, c.expires_at, c.revoked, c.created_by,
                c.created_at, c.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn set_credential_revoked(
        &self,
        id: &str,
        revoked: bool,
        updated_at: i64,
    ) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE credentials SET revoked=?2, updated_at=?3 WHERE id=?1",
            params![id, revoked, updated_at],
        )?;
        Ok(())
    }

    pub fn delete_credential(&self, id: &str) -> AppResult<()> {
        let conn = self.lock();
        conn.execute("DELETE FROM credentials WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn delete_credentials_by_network(&self, network_id: &str) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "DELETE FROM credentials WHERE network_id=?1",
            params![network_id],
        )?;
        Ok(())
    }

    // -------------------------------------------------------------- node_logs

    pub fn insert_node_logs(&self, logs: &[NewNodeLog]) -> AppResult<()> {
        if logs.is_empty() {
            return Ok(());
        }
        let mut conn = self.lock();
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO node_logs (network_id, node_id, source, level, message, created_at) VALUES (?1,?2,?3,?4,?5,?6)",
            )?;
            for l in logs {
                stmt.execute(params![
                    l.network_id,
                    l.node_id,
                    l.source,
                    l.level,
                    l.message,
                    l.created_at
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_node_logs(
        &self,
        network_id: Option<&str>,
        network_ids: Option<&[String]>,
        node_id: Option<&str>,
        level: Option<&str>,
        search: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> AppResult<(Vec<NodeLogRow>, i64)> {
        let mut clauses: Vec<String> = Vec::new();
        let mut args: Vec<SqlValue> = Vec::new();
        if let Some(v) = network_id {
            clauses.push("network_id = ?".to_string());
            args.push(SqlValue::Text(v.to_string()));
        }
        if let Some(ids) = network_ids {
            if ids.is_empty() {
                clauses.push("1 = 0".to_string());
            } else {
                let placeholders = vec!["?"; ids.len()].join(",");
                clauses.push(format!("network_id IN ({placeholders})"));
                for id in ids {
                    args.push(SqlValue::Text(id.clone()));
                }
            }
        }
        if let Some(v) = node_id {
            clauses.push("node_id = ?".to_string());
            args.push(SqlValue::Text(v.to_string()));
        }
        if let Some(v) = level {
            clauses.push("level = ?".to_string());
            args.push(SqlValue::Text(v.to_string()));
        }
        if let Some(v) = search {
            clauses.push("message LIKE ?".to_string());
            args.push(SqlValue::Text(format!("%{v}%")));
        }
        let where_sql = if clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", clauses.join(" AND "))
        };

        let conn = self.lock();
        let total: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM node_logs{where_sql}"),
            params_from_iter(args.iter()),
            |r| r.get(0),
        )?;
        let mut stmt = conn.prepare(&format!(
            "SELECT * FROM node_logs{where_sql} ORDER BY created_at DESC LIMIT ? OFFSET ?"
        ))?;
        let mut query_args = args.clone();
        query_args.push(SqlValue::Integer(page_size));
        query_args.push(SqlValue::Integer((page - 1) * page_size));
        let rows = stmt
            .query_map(params_from_iter(query_args.iter()), map_node_log)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok((rows, total))
    }

    pub fn clear_node_logs(&self, network_id: Option<&str>, before: Option<i64>) -> AppResult<i64> {
        let mut clauses: Vec<String> = Vec::new();
        let mut args: Vec<SqlValue> = Vec::new();
        if let Some(v) = network_id {
            clauses.push("network_id = ?".to_string());
            args.push(SqlValue::Text(v.to_string()));
        }
        if let Some(v) = before {
            clauses.push("created_at < ?".to_string());
            args.push(SqlValue::Integer(v));
        }
        let where_sql = if clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", clauses.join(" AND "))
        };
        let conn = self.lock();
        let changes = conn.execute(
            &format!("DELETE FROM node_logs{where_sql}"),
            params_from_iter(args.iter()),
        )?;
        Ok(changes as i64)
    }

    pub fn delete_node_logs_by_network(&self, network_id: &str) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "DELETE FROM node_logs WHERE network_id=?1",
            params![network_id],
        )?;
        Ok(())
    }

    /// 删除 created_at 早于 before 的运行日志
    pub fn delete_node_logs_before(&self, before: i64) -> AppResult<i64> {
        let conn = self.lock();
        let changes = conn.execute(
            "DELETE FROM node_logs WHERE created_at < ?1",
            params![before],
        )?;
        Ok(changes as i64)
    }

    /// 删除 created_at 早于 before 的审计日志
    pub fn delete_audit_logs_before(&self, before: i64) -> AppResult<i64> {
        let conn = self.lock();
        let changes = conn.execute(
            "DELETE FROM audit_logs WHERE created_at < ?1",
            params![before],
        )?;
        Ok(changes as i64)
    }

    pub fn list_recent_errors(&self, limit: i64) -> AppResult<Vec<NodeLogRow>> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT * FROM node_logs WHERE level='error' ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], map_node_log)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    // -------------------------------------------------------------- audit_logs

    #[allow(clippy::too_many_arguments)]
    pub fn insert_audit(
        &self,
        user_id: Option<&str>,
        username: Option<&str>,
        action: &str,
        resource_type: Option<&str>,
        resource_id: Option<&str>,
        detail: Option<&str>,
        ip: Option<&str>,
        user_agent: Option<&str>,
        created_at: i64,
    ) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO audit_logs (user_id, username, action, resource_type, resource_id, detail, ip, user_agent, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                user_id,
                username,
                action,
                resource_type,
                resource_id,
                detail,
                ip,
                user_agent,
                created_at
            ],
        )?;
        Ok(())
    }

    pub fn list_audit_logs(
        &self,
        user_id: Option<&str>,
        action: Option<&str>,
        username: Option<&str>,
        search: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> AppResult<(Vec<AuditLogRow>, i64)> {
        let mut clauses: Vec<String> = Vec::new();
        let mut args: Vec<SqlValue> = Vec::new();
        if let Some(v) = user_id {
            clauses.push("user_id = ?".to_string());
            args.push(SqlValue::Text(v.to_string()));
        }
        if let Some(v) = action {
            clauses.push("action = ?".to_string());
            args.push(SqlValue::Text(v.to_string()));
        }
        if let Some(v) = username {
            clauses.push("username = ?".to_string());
            args.push(SqlValue::Text(v.to_string()));
        }
        if let Some(v) = search {
            clauses.push("(action LIKE ? OR resource_id LIKE ? OR detail LIKE ?)".to_string());
            let kw = format!("%{v}%");
            args.push(SqlValue::Text(kw.clone()));
            args.push(SqlValue::Text(kw.clone()));
            args.push(SqlValue::Text(kw));
        }
        let where_sql = if clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", clauses.join(" AND "))
        };

        let conn = self.lock();
        let total: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM audit_logs{where_sql}"),
            params_from_iter(args.iter()),
            |r| r.get(0),
        )?;
        let mut stmt = conn.prepare(&format!(
            "SELECT * FROM audit_logs{where_sql} ORDER BY created_at DESC LIMIT ? OFFSET ?"
        ))?;
        let mut query_args = args.clone();
        query_args.push(SqlValue::Integer(page_size));
        query_args.push(SqlValue::Integer((page - 1) * page_size));
        let rows = stmt
            .query_map(params_from_iter(query_args.iter()), map_audit_log)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok((rows, total))
    }

    pub fn list_recent_audit(&self, limit: i64) -> AppResult<Vec<AuditLogRow>> {
        let conn = self.lock();
        let mut stmt =
            conn.prepare("SELECT * FROM audit_logs ORDER BY created_at DESC LIMIT ?1")?;
        let rows = stmt
            .query_map(params![limit], map_audit_log)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    // -------------------------------------------------------------- settings

    pub fn get_settings(&self) -> AppResult<Vec<(String, String)>> {
        let conn = self.lock();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn get_setting(&self, key: &str) -> AppResult<Option<String>> {
        let conn = self.lock();
        let row: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key=?1",
                params![key],
                |r| r.get(0),
            )
            .optional()?;
        Ok(row)
    }

    pub fn upsert_setting(&self, key: &str, value: &str, updated_at: i64) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1,?2,?3)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
            params![key, value, updated_at],
        )?;
        Ok(())
    }
}
