//! Host identity: RSA-2048 self-signed certificate, private key and the
//! unique host id. Generated once, persisted in `config_dir()/identity/`.

use super::config_dir;
use openssl::asn1::Asn1Time;
use openssl::bn::BigNum;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::x509::{X509NameBuilder, X509};
use std::fs;

#[derive(Clone, Debug)]
pub struct Identity {
    pub unique_id: String,
    pub cert_pem: String,
    pub key_pem: String,
}

pub fn load_or_create() -> Result<Identity, String> {
    let dir = config_dir().join("identity");
    fs::create_dir_all(&dir).map_err(|e| format!("create {dir:?}: {e}"))?;

    let id_path = dir.join("uniqueid.txt");
    let cert_path = dir.join("cert.pem");
    let key_path = dir.join("key.pem");

    if let (Ok(id), Ok(cert), Ok(key)) = (
        fs::read_to_string(&id_path),
        fs::read_to_string(&cert_path),
        fs::read_to_string(&key_path),
    ) {
        let id = id.trim().to_string();
        if !id.is_empty() && !cert.is_empty() && !key.is_empty() {
            return Ok(Identity {
                unique_id: id,
                cert_pem: cert,
                key_pem: key,
            });
        }
    }

    let (cert_pem, key_pem) = generate_cert().map_err(|e| format!("cert generation: {e}"))?;
    let unique_id = generate_unique_id().map_err(|e| format!("unique id: {e}"))?;

    fs::write(&id_path, &unique_id).map_err(|e| format!("write {id_path:?}: {e}"))?;
    fs::write(&cert_path, &cert_pem).map_err(|e| format!("write {cert_path:?}: {e}"))?;
    fs::write(&key_path, &key_pem).map_err(|e| format!("write {key_path:?}: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600));
    }

    Ok(Identity {
        unique_id,
        cert_pem,
        key_pem,
    })
}

fn generate_unique_id() -> Result<String, openssl::error::ErrorStack> {
    let mut bytes = [0u8; 8];
    openssl::rand::rand_bytes(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02X}")).collect())
}

fn generate_cert() -> Result<(String, String), openssl::error::ErrorStack> {
    let rsa = Rsa::generate(2048)?;
    let pkey = PKey::from_rsa(rsa)?;

    let mut name = X509NameBuilder::new()?;
    name.append_entry_by_text("CN", "NVIDIA GameStream Host")?;
    let name = name.build();

    let mut serial = [0u8; 8];
    openssl::rand::rand_bytes(&mut serial)?;
    serial[0] &= 0x7F;
    let serial = BigNum::from_slice(&serial)?.to_asn1_integer()?;

    let not_before = Asn1Time::days_from_now(0)?;
    let not_after = Asn1Time::days_from_now(7300)?;

    let mut b = X509::builder()?;
    b.set_version(2)?;
    b.set_serial_number(&serial)?;
    b.set_subject_name(&name)?;
    b.set_issuer_name(&name)?;
    b.set_pubkey(&pkey)?;
    b.set_not_before(&not_before)?;
    b.set_not_after(&not_after)?;
    b.sign(&pkey, MessageDigest::sha256())?;
    let cert = b.build();

    let cert_pem = String::from_utf8_lossy(&cert.to_pem()?).into_owned();
    let key_pem = String::from_utf8_lossy(&pkey.private_key_to_pem_pkcs8()?).into_owned();
    Ok((cert_pem, key_pem))
}
