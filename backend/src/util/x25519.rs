use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use x25519_dalek::{PublicKey, StaticSecret};

pub struct KeyPair {
    pub private_key: String,
    pub public_key: String,
}

/// 生成一对 X25519 密钥（base64 原始 32 字节）
pub fn derive_key_pair() -> KeyPair {
    let secret = StaticSecret::random_from_rng(rand::rngs::OsRng);
    let public = PublicKey::from(&secret);
    KeyPair {
        private_key: STANDARD.encode(secret.to_bytes()),
        public_key: STANDARD.encode(public.as_bytes()),
    }
}

/// 由私钥推导公钥（base64 → base64），失败返回空字符串
pub fn derive_public_key(private_key_b64: &str) -> String {
    let Ok(raw) = STANDARD.decode(private_key_b64) else {
        return String::new();
    };
    if raw.len() != 32 {
        return String::new();
    }
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&raw);
    let secret = StaticSecret::from(bytes);
    let public = PublicKey::from(&secret);
    STANDARD.encode(public.as_bytes())
}
