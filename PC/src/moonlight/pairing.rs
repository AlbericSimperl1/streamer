//! GameStream pairing state en crypto helpers.

use openssl::encrypt::{Decrypter, Encrypter};
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::{Padding, Rsa};
use openssl::sign::{Signer, Verifier};
use openssl::x509::X509;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use aes::cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes128;
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

#[derive(Clone)]
pub struct PairingSession {
    pub client_name: String,
    pub salt: Vec<u8>,
    pub client_cert_der: Vec<u8>,
    pub aes_key: [u8; 16],
    pub server_secret: Option<[u8; 16]>,
    pub created: Instant,
}

static SESSIONS: OnceLock<Mutex<HashMap<String, PairingSession>>> = OnceLock::new();

fn sessions() -> &'static Mutex<HashMap<String, PairingSession>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn insert(unique_id: String, session: PairingSession) {
    let mut map = sessions().lock().unwrap();
    map.retain(|_, s| s.created.elapsed().as_secs() < 300);
    map.insert(unique_id, session);
}

pub fn get(unique_id: &str) -> Option<PairingSession> {
    sessions().lock().unwrap().get(unique_id).cloned()
}

pub fn update_server_secret(unique_id: &str, secret: [u8; 16]) {
    if let Some(s) = sessions().lock().unwrap().get_mut(unique_id) {
        s.server_secret = Some(secret);
    }
}

pub fn remove(unique_id: &str) {
    sessions().lock().unwrap().remove(unique_id);
}

// ---------- Crypto helpers ----------

/// AES-sleutel = PBKDF2-HMAC-SHA256(PIN, salt, 1024 iteraties, 16 bytes output).
pub fn derive_aes_key(pin: &str, salt: &[u8]) -> [u8; 16] {
    let mut key = [0u8; 16];
    pbkdf2_hmac::<Sha256>(pin.as_bytes(), salt, 1024, &mut key);
    key
}

pub fn aes_ecb_decrypt(key: &[u8; 16], data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() % 16 != 0 {
        return Err(format!("AES data not block-aligned: {}", data.len()));
    }
    let cipher = Aes128::new(GenericArray::from_slice(key));
    let mut out = Vec::with_capacity(data.len());
    for chunk in data.chunks(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        out.extend_from_slice(&block);
    }
    Ok(out)
}

pub fn aes_ecb_encrypt(key: &[u8; 16], data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() % 16 != 0 {
        return Err(format!("AES data not block-aligned: {}", data.len()));
    }
    let cipher = Aes128::new(GenericArray::from_slice(key));
    let mut out = Vec::with_capacity(data.len());
    for chunk in data.chunks(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.encrypt_block(&mut block);
        out.extend_from_slice(&block);
    }
    Ok(out)
}

pub fn rsa_sign_sha256(key_pem: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    let rsa =
        Rsa::private_key_from_pem(key_pem.as_bytes()).map_err(|e| format!("rsa private: {e}"))?;
    let pkey = PKey::from_rsa(rsa).map_err(|e| format!("pkey: {e}"))?;
    let mut signer =
        Signer::new(MessageDigest::sha256(), &pkey).map_err(|e| format!("signer: {e}"))?;
    signer.update(data).map_err(|e| format!("update: {e}"))?;
    signer.sign_to_vec().map_err(|e| format!("sign: {e}"))
}

pub fn rsa_verify_sha256(cert_pem: &str, data: &[u8], sig: &[u8]) -> Result<bool, String> {
    let cert = X509::from_pem(cert_pem.as_bytes()).map_err(|e| format!("x509: {e}"))?;
    let pub_key = cert.public_key().map_err(|e| format!("pubkey: {e}"))?;
    let mut verifier =
        Verifier::new(MessageDigest::sha256(), &pub_key).map_err(|e| format!("verifier: {e}"))?;
    verifier.update(data).map_err(|e| format!("update: {e}"))?;
    verifier.verify(sig).map_err(|e| format!("verify: {e}"))
}

pub fn rsa_oaep_encrypt(cert_pem: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    let cert = X509::from_pem(cert_pem.as_bytes()).map_err(|e| format!("x509: {e}"))?;
    let pub_key = cert.public_key().map_err(|e| format!("pubkey: {e}"))?;
    let mut enc = Encrypter::new(&pub_key).map_err(|e| format!("encrypter: {e}"))?;
    enc.set_rsa_padding(Padding::PKCS1_OAEP)
        .map_err(|e| format!("pad: {e}"))?;
    enc.set_rsa_oaep_md(MessageDigest::sha1())
        .map_err(|e| format!("oaep md: {e}"))?;
    enc.set_rsa_mgf1_md(MessageDigest::sha1())
        .map_err(|e| format!("mgf1: {e}"))?;
    let mut out = vec![0u8; enc.encrypt_len(data).map_err(|e| format!("enclen: {e}"))?];
    let n = enc
        .encrypt(data, &mut out)
        .map_err(|e| format!("encrypt: {e}"))?;
    out.truncate(n);
    Ok(out)
}

pub fn rsa_oaep_decrypt(key_pem: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    let rsa =
        Rsa::private_key_from_pem(key_pem.as_bytes()).map_err(|e| format!("rsa private: {e}"))?;
    let pkey = PKey::from_rsa(rsa).map_err(|e| format!("pkey: {e}"))?;
    let mut dec = Decrypter::new(&pkey).map_err(|e| format!("decrypter: {e}"))?;
    dec.set_rsa_padding(Padding::PKCS1_OAEP)
        .map_err(|e| format!("pad: {e}"))?;
    dec.set_rsa_oaep_md(MessageDigest::sha1())
        .map_err(|e| format!("oaep md: {e}"))?;
    dec.set_rsa_mgf1_md(MessageDigest::sha1())
        .map_err(|e| format!("mgf1: {e}"))?;
    let mut out = vec![0u8; dec.decrypt_len(data).map_err(|e| format!("declen: {e}"))?];
    let n = dec
        .decrypt(data, &mut out)
        .map_err(|e| format!("decrypt: {e}"))?;
    out.truncate(n);
    Ok(out)
}

/// Zet een PEM-certificaat om naar hex-encoded DER (wat Moonlight wil).
pub fn pem_to_der_hex(pem: &str) -> Result<String, String> {
    let cert = X509::from_pem(pem.as_bytes()).map_err(|e| format!("x509: {e}"))?;
    let der = cert.to_der().map_err(|e| format!("der: {e}"))?;
    Ok(hex::encode(der))
}

/// Zet hex-encoded DER om naar PEM.
pub fn der_hex_to_pem(hex_str: &str) -> Result<String, String> {
    let der = hex::decode(hex_str).map_err(|e| format!("hex: {e}"))?;
    let cert = X509::from_der(&der).map_err(|e| format!("x509 from der: {e}"))?;
    let pem = cert.to_pem().map_err(|e| format!("pem: {e}"))?;
    Ok(String::from_utf8_lossy(&pem).into_owned())
}
