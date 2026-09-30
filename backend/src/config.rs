use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;

/// 可在运行时被 `/api/system/settings` 覆盖的 EasyTier 设置。
#[derive(Clone, Debug)]
pub struct EasyTierSettings {
    pub core_path: String,
    pub cli_path: String,
    pub default_external_node: String,
    pub rpc_port_start: u16,
    pub listen_port_start: u16,
    pub disable_env_parsing: bool,
}

impl Default for EasyTierSettings {
    fn default() -> Self {
        Self {
            core_path: "easytier-core".to_string(),
            cli_path: "easytier-cli".to_string(),
            default_external_node: "tcp://public.easytier.cn:11010".to_string(),
            rpc_port_start: 15888,
            listen_port_start: 11010,
            disable_env_parsing: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub env: String,
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub cors_origin: Vec<String>,

    pub data_dir: PathBuf,
    pub repo_root: PathBuf,
    pub frontend_dist: PathBuf,
    pub db_path: PathBuf,
    pub networks_dir: PathBuf,
    pub credentials_dir: PathBuf,
    pub logs_dir: PathBuf,

    pub app_secret: String,
    pub jwt_ttl: u64,

    pub easytier: EasyTierSettings,
}

fn env_str(name: &str, fallback: &str) -> String {
    match env::var(name) {
        Ok(v) if !v.is_empty() => v,
        _ => fallback.to_string(),
    }
}

fn env_int(name: &str, fallback: u16) -> u16 {
    env::var(name)
        .ok()
        .and_then(|v| v.trim().parse::<u16>().ok())
        .unwrap_or(fallback)
}

fn env_i64(name: &str, fallback: i64) -> i64 {
    env::var(name)
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(fallback)
}

fn env_bool(name: &str, fallback: bool) -> bool {
    match env::var(name) {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            if v.is_empty() {
                fallback
            } else {
                matches!(v.as_str(), "1" | "true" | "yes" | "on")
            }
        }
        Err(_) => fallback,
    }
}

fn ensure_dir(dir: &Path) -> PathBuf {
    if let Err(err) = fs::create_dir_all(dir) {
        tracing::warn!(error = %err, path = %dir.display(), "无法创建目录");
    }
    dir.to_path_buf()
}

/// 计算仓库根目录：backend/ 下运行 -> 上一级；否则使用当前目录。
pub fn detect_repo_root() -> PathBuf {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let is_backend = cwd.join("Cargo.toml").is_file()
        && cwd.file_name().map(|n| n == "backend").unwrap_or(false);
    if is_backend {
        cwd.parent().map(Path::to_path_buf).unwrap_or(cwd)
    } else {
        cwd
    }
}

/// 依次加载根目录与 backend 目录下的 .env（后者优先）。
pub fn load_env(repo_root: &Path) {
    let _ = dotenvy::from_path(repo_root.join(".env"));
    let _ = dotenvy::from_path_override(repo_root.join("backend").join(".env"));
}

/// 读取或生成用于 JWT 签名 + 敏感字段加密的主密钥。
fn resolve_app_secret(data_dir: &Path) -> String {
    let from_env = env_str("APP_SECRET", "");
    if !from_env.is_empty() {
        return from_env;
    }
    let secret_file = data_dir.join(".app_secret");
    if let Ok(existing) = fs::read_to_string(&secret_file) {
        let existing = existing.trim().to_string();
        if !existing.is_empty() {
            return existing;
        }
    }
    let mut bytes = [0u8; 48];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let generated = URL_SAFE_NO_PAD.encode(bytes);
    if let Err(err) = fs::write(&secret_file, &generated) {
        tracing::warn!(error = %err, "写入 .app_secret 失败");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&secret_file, fs::Permissions::from_mode(0o600));
    }
    generated
}

impl Config {
    pub fn load() -> Config {
        let repo_root = detect_repo_root();
        load_env(&repo_root);

        let data_dir_raw = env_str("DATA_DIR", "./data");
        let data_dir = if Path::new(&data_dir_raw).is_absolute() {
            PathBuf::from(&data_dir_raw)
        } else {
            repo_root.join(&data_dir_raw)
        };
        let data_dir = ensure_dir(&data_dir);

        let db_path_raw = env_str("DATABASE_PATH", "");
        let db_path = if !db_path_raw.is_empty() && Path::new(&db_path_raw).is_absolute() {
            PathBuf::from(&db_path_raw)
        } else {
            data_dir.join("easytier-admin.db")
        };

        let frontend_raw = env_str("FRONTEND_DIST", "");
        let frontend_dist = if !frontend_raw.is_empty() {
            let p = PathBuf::from(&frontend_raw);
            if p.is_absolute() {
                p
            } else {
                repo_root.join(p)
            }
        } else {
            repo_root.join("frontend").join("dist")
        };

        let cors_origin = env_str("CORS_ORIGIN", "")
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let networks_dir = ensure_dir(&data_dir.join("networks"));
        let credentials_dir = ensure_dir(&data_dir.join("credentials"));
        let logs_dir = ensure_dir(&data_dir.join("logs"));

        let app_secret = resolve_app_secret(&data_dir);

        Config {
            env: env_str("NODE_ENV", "development"),
            host: env_str("HOST", "0.0.0.0"),
            port: env_int("PORT", 11211),
            log_level: env_str("LOG_LEVEL", "info"),
            cors_origin,

            data_dir,
            repo_root,
            frontend_dist,
            db_path,
            networks_dir,
            credentials_dir,
            logs_dir,

            app_secret,
            jwt_ttl: env_i64("JWT_TTL", 86400).max(0) as u64,

            easytier: EasyTierSettings {
                core_path: env_str("EASYTIER_CORE_PATH", "easytier-core"),
                cli_path: env_str("EASYTIER_CLI_PATH", "easytier-cli"),
                default_external_node: env_str(
                    "EASYTIER_DEFAULT_EXTERNAL_NODE",
                    "tcp://public.easytier.cn:11010",
                ),
                rpc_port_start: env_int("EASYTIER_RPC_PORT_START", 15888),
                listen_port_start: env_int("EASYTIER_LISTEN_PORT_START", 11010),
                disable_env_parsing: env_bool("EASYTIER_DISABLE_ENV_PARSING", true),
            },
        }
    }

    pub fn config_path_for(&self, network_id: &str) -> PathBuf {
        self.networks_dir.join(format!("{network_id}.toml"))
    }

    pub fn credential_file_path_for(&self, network_id: &str) -> PathBuf {
        self.credentials_dir.join(format!("{network_id}.json"))
    }

    /// 用于前端展示的数据目录绝对路径。
    pub fn data_dir_string(&self) -> String {
        self.data_dir.to_string_lossy().to_string()
    }
}
