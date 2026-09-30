use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use rand::RngCore;
use scrypt::{scrypt, Params};
use sha2::{Digest, Sha256};

const SCRYPT_SALT: &[u8] = b"easytier-admin::field-encryption";
const TAG_LEN: usize = 16;
const NONCE_LEN: usize = 12;

/// 敏感字段（network_secret / credential_secret / 私钥等）的静态加密。
/// 使用 AES-256-GCM，密钥由 APP_SECRET 经 scrypt 派生。
/// 存储格式与 Node 版本保持一致：base64(iv[12] || tag[16] || ciphertext)。
#[derive(Clone)]
pub struct Crypto {
    key: [u8; 32],
}

impl Crypto {
    pub fn new(app_secret: &str) -> Self {
        let mut key = [0u8; 32];
        let params = Params::new(14, 8, 1, 32).expect("valid scrypt params");
        scrypt(app_secret.as_bytes(), SCRYPT_SALT, &params, &mut key)
            .expect("scrypt derivation failed");
        Self { key }
    }

    fn cipher(&self) -> Aes256Gcm {
        Aes256Gcm::new_from_slice(&self.key).expect("32-byte key")
    }

    pub fn encrypt(&self, plain: &str) -> String {
        let mut iv = [0u8; NONCE_LEN];
        rand::rngs::OsRng.fill_bytes(&mut iv);
        let sealed = self
            .cipher()
            .encrypt(Nonce::from_slice(&iv), plain.as_bytes())
            .expect("AES-GCM encryption failed");
        // aes-gcm 返回 ciphertext || tag，转换为 Node 的 tag || ciphertext
        let split = sealed.len() - TAG_LEN;
        let (ciphertext, tag) = sealed.split_at(split);
        let mut out = Vec::with_capacity(NONCE_LEN + TAG_LEN + ciphertext.len());
        out.extend_from_slice(&iv);
        out.extend_from_slice(tag);
        out.extend_from_slice(ciphertext);
        STANDARD.encode(out)
    }

    pub fn decrypt(&self, payload: &str) -> Result<String, String> {
        let raw = STANDARD
            .decode(payload)
            .map_err(|e| format!("base64 解码失败: {e}"))?;
        if raw.len() < NONCE_LEN + TAG_LEN {
            return Err("密文长度不足".to_string());
        }
        let iv = &raw[..NONCE_LEN];
        let tag = &raw[NONCE_LEN..NONCE_LEN + TAG_LEN];
        let ciphertext = &raw[NONCE_LEN + TAG_LEN..];
        let mut sealed = Vec::with_capacity(ciphertext.len() + TAG_LEN);
        sealed.extend_from_slice(ciphertext);
        sealed.extend_from_slice(tag);
        let plain = self
            .cipher()
            .decrypt(Nonce::from_slice(iv), sealed.as_ref())
            .map_err(|_| "解密失败".to_string())?;
        String::from_utf8(plain).map_err(|e| format!("UTF-8 解码失败: {e}"))
    }

    /// 安全解密：解密失败或为空时返回空字符串，避免历史数据损坏导致接口 500。
    pub fn try_decrypt(&self, payload: Option<&str>) -> String {
        match payload {
            Some(p) if !p.is_empty() => self.decrypt(p).unwrap_or_default(),
            _ => String::new(),
        }
    }
}

pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn random_token(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::rngs::OsRng.fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}
