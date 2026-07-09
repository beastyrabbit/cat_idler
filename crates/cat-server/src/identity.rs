//! Session HMAC helpers ported from `lib/game/identity.ts` and
//! `server/players.ts`.

use std::sync::atomic::{AtomicU64, Ordering};

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

const DEV_FALLBACK_SESSION_SECRET: &str = "dev-insecure-session-secret-change-me";

static NEXT_SESSION_NONCE: AtomicU64 = AtomicU64::new(1);

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedSession {
    pub session_id: String,
    pub sig: String,
    pub player_id: String,
}

pub fn session_secret_from_env() -> Result<String, String> {
    if let Ok(secret) = std::env::var("SESSION_HMAC_SECRET")
        && !secret.is_empty()
    {
        return Ok(secret);
    }

    if std::env::var("NODE_ENV").as_deref() == Ok("production") {
        return Err(
            "SESSION_HMAC_SECRET must be set in production; refusing development fallback"
                .to_owned(),
        );
    }

    Ok(DEV_FALLBACK_SESSION_SECRET.to_owned())
}

#[must_use]
pub fn sign_session(session_id: &str, secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts secrets of any byte length");
    mac.update(session_id.as_bytes());
    hex_encode(mac.finalize().into_bytes().as_ref())
}

#[must_use]
pub fn verify_session(session_id: &str, sig: Option<&str>, secret: &str) -> bool {
    let Some(sig) = sig else {
        return false;
    };
    if sig.len() != 64 {
        return false;
    }

    let mut provided = [0_u8; 32];
    if !hex_decode_32(sig, &mut provided) {
        return false;
    }

    let expected_hex = sign_session(session_id, secret);
    let mut expected = [0_u8; 32];
    if !hex_decode_32(&expected_hex, &mut expected) {
        return false;
    }

    let mut diff = 0_u8;
    for (left, right) in expected.iter().zip(provided.iter()) {
        diff |= left ^ right;
    }
    diff == 0
}

#[must_use]
pub fn issue_session(secret: &str, now_ms: i64) -> SignedSession {
    let nonce = NEXT_SESSION_NONCE.fetch_add(1, Ordering::SeqCst);
    let seed = format!("{now_ms}:{nonce}");
    let session_id = format!("session_{}", &sign_session(&seed, secret)[..32]);
    signed_session(session_id, secret)
}

#[must_use]
pub fn signed_session(session_id: String, secret: &str) -> SignedSession {
    let sig = sign_session(&session_id, secret);
    let player_id = player_id_for_session(&session_id);
    SignedSession {
        session_id,
        sig,
        player_id,
    }
}

#[must_use]
pub fn player_id_for_session(session_id: &str) -> String {
    format!(
        "player_{}",
        &sign_session(session_id, "cat-server-player-id")[..16]
    )
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn hex_decode_32(input: &str, out: &mut [u8; 32]) -> bool {
    let bytes = input.as_bytes();
    if bytes.len() != 64 {
        return false;
    }

    for index in 0..32 {
        let Some(high) = hex_nibble(bytes[index * 2]) else {
            return false;
        };
        let Some(low) = hex_nibble(bytes[index * 2 + 1]) else {
            return false;
        };
        out[index] = (high << 4) | low;
    }
    true
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_sig_verify_accepts_signed_session_and_rejects_bad_sigs() {
        let secret = "test-secret";
        let sig = sign_session("session-1", secret);

        assert!(verify_session("session-1", Some(&sig), secret));
        assert!(verify_session(
            "session-1",
            Some(&sig.to_uppercase()),
            secret
        ));
        assert!(!verify_session("session-2", Some(&sig), secret));
        assert!(!verify_session("session-1", Some("not-hex"), secret));
        assert!(!verify_session("session-1", None, secret));
    }
}
