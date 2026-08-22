//! Session HMAC helpers ported from `lib/game/identity.ts` and
//! `server/players.ts`.

use std::{
    net::IpAddr,
    sync::atomic::{AtomicU64, Ordering},
};

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

const DEV_FALLBACK_SESSION_SECRET: &str = "dev-insecure-session-secret-change-me";
const INSECURE_SECRET_OPT_IN: &str = "CAT_SERVER_ALLOW_INSECURE_SESSION_SECRET";
const SESSION_ID_VERSION: &str = "v2";
pub(crate) const SESSION_MAX_AGE_MS: i64 = 30 * 24 * 60 * 60 * 1_000;
pub(crate) const SESSION_RENEWAL_GRACE_MS: i64 = 7 * 24 * 60 * 60 * 1_000;

static NEXT_SESSION_NONCE: AtomicU64 = AtomicU64::new(1);

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedSession {
    pub session_id: String,
    pub sig: String,
    pub player_id: String,
}

pub fn session_secret_from_env(bind_addr: IpAddr) -> Result<String, String> {
    session_secret_from_lookup(bind_addr, |key| std::env::var(key).ok())
}

fn session_secret_from_lookup(
    bind_addr: IpAddr,
    mut lookup: impl FnMut(&str) -> Option<String>,
) -> Result<String, String> {
    if let Some(secret) = lookup("SESSION_HMAC_SECRET")
        && !secret.is_empty()
    {
        return Ok(secret);
    }

    let insecure_opt_in = lookup(INSECURE_SECRET_OPT_IN)
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes"));
    if !bind_addr.is_loopback() && !insecure_opt_in {
        return Err(format!(
            "SESSION_HMAC_SECRET must be set when binding publicly; set {INSECURE_SECRET_OPT_IN}=1 only for deliberate insecure development"
        ));
    }
    if lookup("NODE_ENV").as_deref() == Some("production") && !insecure_opt_in {
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
#[cfg(test)]
pub fn verify_session(session_id: &str, sig: Option<&str>, secret: &str) -> bool {
    verify_session_at(session_id, sig, secret, crate::now_ms())
}

#[must_use]
pub fn verify_session_at(session_id: &str, sig: Option<&str>, secret: &str, now_ms: i64) -> bool {
    if !session_signature_valid(session_id, sig, secret) {
        return false;
    }
    let Some(issued_at) = session_issued_at(session_id) else {
        // Arbitrary signed ids are useful for deterministic unit fixtures. Production-issued
        // sessions always use a timestamped shape and legacy permanent bearer tokens are
        // accepted only by the bounded upgrade path outside tests.
        return cfg!(test) && !is_legacy_issued_session(session_id);
    };
    now_ms >= issued_at && now_ms.saturating_sub(issued_at) <= SESSION_MAX_AGE_MS
}

#[must_use]
pub fn session_signature_valid(session_id: &str, sig: Option<&str>, secret: &str) -> bool {
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
    let player_token = &sign_session(&format!("player:{seed}"), secret)[..16];
    issue_session_for_player_token(secret, now_ms, player_token)
}

#[must_use]
pub fn renew_session_at(
    session_id: &str,
    sig: Option<&str>,
    secret: &str,
    now_ms: i64,
) -> Option<SignedSession> {
    if !session_signature_valid(session_id, sig, secret) {
        return None;
    }
    if let Some(issued_at) = session_issued_at(session_id)
        && (now_ms < issued_at
            || now_ms.saturating_sub(issued_at)
                > SESSION_MAX_AGE_MS.saturating_add(SESSION_RENEWAL_GRACE_MS))
    {
        return None;
    }
    let player_id = player_id_for_session(session_id);
    let player_token = player_id.strip_prefix("player_")?;
    Some(issue_session_for_player_token(secret, now_ms, player_token))
}

fn issue_session_for_player_token(secret: &str, now_ms: i64, player_token: &str) -> SignedSession {
    let nonce = NEXT_SESSION_NONCE.fetch_add(1, Ordering::SeqCst);
    let seed = format!("{now_ms}:{player_token}:{nonce}");
    let session_id = format!(
        "session_{SESSION_ID_VERSION}_{now_ms}_{player_token}_{}",
        &sign_session(&seed, secret)[..32]
    );
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
    if let Some(player_token) = v2_player_token(session_id) {
        return format!("player_{player_token}");
    }
    format!(
        "player_{}",
        &sign_session(session_id, "cat-server-player-id")[..16]
    )
}

fn session_issued_at(session_id: &str) -> Option<i64> {
    let rest = session_id.strip_prefix("session_")?;
    let (version, rest) = rest.split_once('_')?;
    if !matches!(version, "v1" | "v2") {
        return None;
    }
    rest.split_once('_')?.0.parse::<i64>().ok()
}

fn is_legacy_issued_session(session_id: &str) -> bool {
    session_id.strip_prefix("session_").is_some_and(|token| {
        token.len() == 32 && token.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn v2_player_token(session_id: &str) -> Option<&str> {
    let rest = session_id.strip_prefix("session_v2_")?;
    let (_, rest) = rest.split_once('_')?;
    let (token, nonce) = rest.split_once('_')?;
    (token.len() == 16 && token.bytes().all(|byte| byte.is_ascii_hexdigit()) && !nonce.is_empty())
        .then_some(token)
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
    use std::collections::BTreeMap;

    fn secret_config(bind_addr: IpAddr, values: &[(&str, &str)]) -> Result<String, String> {
        let values = values
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect::<BTreeMap<_, _>>();
        session_secret_from_lookup(bind_addr, |key| values.get(key).cloned())
    }

    #[test]
    fn public_binding_requires_a_real_secret_unless_insecurely_opted_in() {
        let public = IpAddr::from([0, 0, 0, 0]);
        assert!(secret_config(public, &[]).is_err());
        assert_eq!(
            secret_config(public, &[("SESSION_HMAC_SECRET", "deployment-secret")]),
            Ok("deployment-secret".to_owned())
        );
        assert_eq!(
            secret_config(public, &[(INSECURE_SECRET_OPT_IN, "true")]),
            Ok(DEV_FALLBACK_SESSION_SECRET.to_owned())
        );
        assert_eq!(
            secret_config(IpAddr::from([127, 0, 0, 1]), &[]),
            Ok(DEV_FALLBACK_SESSION_SECRET.to_owned())
        );
    }

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

    #[test]
    fn issued_sessions_expire_after_thirty_days() {
        let issued = issue_session("test-secret", 1_000_000);
        assert!(verify_session_at(
            &issued.session_id,
            Some(&issued.sig),
            "test-secret",
            1_000_000 + SESSION_MAX_AGE_MS
        ));
        assert!(!verify_session_at(
            &issued.session_id,
            Some(&issued.sig),
            "test-secret",
            1_000_001 + SESSION_MAX_AGE_MS
        ));
    }

    #[test]
    fn legacy_and_recently_expired_v1_sessions_renew_without_changing_player() {
        let secret = "test-secret";
        let legacy = signed_session(
            "session_0123456789abcdef0123456789abcdef".to_owned(),
            secret,
        );
        assert!(!verify_session_at(
            &legacy.session_id,
            Some(&legacy.sig),
            secret,
            1_000_000
        ));
        let upgraded = renew_session_at(&legacy.session_id, Some(&legacy.sig), secret, 1_000_000)
            .expect("authentic legacy session upgrades");
        assert_eq!(upgraded.player_id, legacy.player_id);
        assert!(verify_session_at(
            &upgraded.session_id,
            Some(&upgraded.sig),
            secret,
            1_000_000
        ));

        let issued_at = 5_000_000;
        let v1 = signed_session(
            format!("session_v1_{issued_at}_0123456789abcdef0123456789abcdef"),
            secret,
        );
        let renewed_at = issued_at + SESSION_MAX_AGE_MS + 1;
        let renewed = renew_session_at(&v1.session_id, Some(&v1.sig), secret, renewed_at)
            .expect("recently expired v1 session renews");
        assert_eq!(renewed.player_id, v1.player_id);
        assert!(verify_session_at(
            &renewed.session_id,
            Some(&renewed.sig),
            secret,
            renewed_at
        ));
    }

    #[test]
    fn renewal_rejects_tampering_future_tokens_and_credentials_beyond_grace() {
        let secret = "test-secret";
        let issued_at = 5_000_000;
        let v1 = signed_session(
            format!("session_v1_{issued_at}_0123456789abcdef0123456789abcdef"),
            secret,
        );
        assert!(
            renew_session_at(&v1.session_id, Some("bad-signature"), secret, issued_at).is_none()
        );
        assert!(
            renew_session_at(
                &v1.session_id,
                Some(&v1.sig),
                secret,
                issued_at + SESSION_MAX_AGE_MS + SESSION_RENEWAL_GRACE_MS + 1,
            )
            .is_none()
        );
        assert!(renew_session_at(&v1.session_id, Some(&v1.sig), secret, issued_at - 1,).is_none());
    }
}
