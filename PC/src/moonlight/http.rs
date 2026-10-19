//! HTTP/HTTPS server voor de GameStream endpoints.
//!
//! Plain HTTP (47989): /serverinfo
//! HTTPS (47984): /pair, /applist, /launch, /resume, /cancel

use super::pairing;
use super::{HostShared, PairedClient, DEFAULT_HTTPS_PORT, DEFAULT_RTSP_PORT};
use crate::rtsp::RtspServer;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rand::rand_bytes;
use openssl::rsa::Rsa;
use openssl::sign::Signer;
use openssl::symm::{Cipher, Crypter, Mode};
use rand::Rng;
use rustls::server::WebPkiClientVerifier;
use rustls::RootCertStore;
use rustls_pemfile;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

pub const APP_VERSION: &str = "7.1.431.-1";
pub const GFE_VERSION: &str = "3.23.0.74";
pub const HTTPS_PORT: u16 = DEFAULT_HTTPS_PORT;
pub const RTSP_PORT: u16 = DEFAULT_RTSP_PORT;

/// H.264 only.
pub const CODEC_MODE_SUPPORT: u32 = 0x1;

pub struct HttpCtx {
    pub hostname: String,
    pub unique_id: String,
    pub http_port: u16,
    pub https_port: u16,
    pub rtsp_port: u16,
    pub shared: HostShared,
}

pub struct Request {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub body: Vec<u8>,
}

// =====================================================================
// Plain HTTP handler (47989)
// =====================================================================

pub fn handle_plain(mut conn: TcpStream, ctx: &HttpCtx) {
    let _ = conn.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = conn.set_write_timeout(Some(Duration::from_secs(3)));

    let req = match read_request(&mut conn) {
        Some(r) => r,
        None => return,
    };

    let local_ip = conn
        .local_addr()
        .map(|a| a.ip().to_string())
        .unwrap_or_default();

    log::info!("http {} {}", req.method, req.path);

    let (status_line, body) = match req.path.as_str() {
        "/serverinfo" => ("200 OK", serverinfo(ctx, &local_ip, false)),
        "/pair" => {
            // Pairing starts on plain HTTP in some clients, but we redirect
            // to HTTPS. Moonlight iOS does the full handshake on 47984.
            ("200 OK", status_xml(200, "use HTTPS port"))
        }
        _ => ("404 Not Found", status_xml(404, "not found")),
    };

    let resp = format!(
        "HTTP/1.1 {status_line}\r\nContent-Type: text/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = conn.write_all(resp.as_bytes());
    let _ = conn.flush();
}

// =====================================================================
// HTTPS server (47984) met mTLS
// =====================================================================

pub async fn run_https_server(
    listener: TcpListener,
    ctx: Arc<HttpCtx>,
    stop: Arc<AtomicBool>,
) -> Result<(), String> {
    let identity = super::identity::load_or_create().map_err(|e| format!("identity: {e}"))?;

    // Laad server certificaat en key
    let cert = rustls_pemfile::certs(&mut identity.cert_pem.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("cert parse: {e}"))?;
    let key = rustls_pemfile::private_key(&mut identity.key_pem.as_bytes())
        .map_err(|e| format!("key parse: {e}"))?
        .ok_or("no private key")?;

    // Client certificate verifier: accepteer alleen bekende paired clients
    let mut root_store = RootCertStore::empty();
    for client in ctx.shared.paired_clients() {
        if !client.cert_pem.is_empty() {
            if let Ok(certs) = rustls_pemfile::certs(&mut client.cert_pem.as_bytes())
                .collect::<Result<Vec<_>, _>>()
            {
                for cert in certs {
                    let _ = root_store.add(cert);
                }
            }
        }
    }

    let client_verifier = WebPkiClientVerifier::builder(Arc::new(root_store))
        .build()
        .map_err(|e| format!("client verifier: {e}"))?;

    let config = rustls::ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(cert, key)
        .map_err(|e| format!("tls config: {e}"))?;

    let acceptor = TlsAcceptor::from(Arc::new(config));

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        match listener.accept().await {
            Ok((stream, _addr)) => {
                let acceptor = acceptor.clone();
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    match acceptor.accept(stream).await {
                        Ok(mut tls_stream) => {
                            let mut buf = vec![0u8; 8192];
                            // Lees het request
                            match tls_stream.read(&mut buf).await {
                                Ok(n) if n > 0 => {
                                    let req = parse_request(&buf[..n]);
                                    if let Some(req) = req {
                                        let local_ip = "0.0.0.0"; // simplified
                                        let (status_line, body) = route_https(&req, &ctx, local_ip);
                                        let resp = format!(
                                            "HTTP/1.1 {status_line}\r\nContent-Type: text/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                                            body.len()
                                        );
                                        let _ = tls_stream.write_all(resp.as_bytes()).await;
                                        let _ = tls_stream.flush().await;
                                    }
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            log::warn!("TLS accept error: {e}");
                        }
                    }
                });
            }
            Err(e) => {
                log::warn!("HTTPS accept error: {e}");
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    Ok(())
}

fn parse_request(buf: &[u8]) -> Option<Request> {
    let text = String::from_utf8_lossy(buf);
    let mut lines = text.split("\r\n");
    let request_line = lines.next()?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?;

    let (path, query_str) = match target.split_once('?') {
        Some((p, q)) => (p, q),
        None => (target, ""),
    };

    let mut headers: HashMap<String, String> = HashMap::new();
    for line in lines.by_ref() {
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_lowercase(), v.trim().to_string());
        }
    }

    // Body = alles na \r\n\r\n
    let body_start = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| i + 4)
        .unwrap_or(buf.len());
    let body_bytes = &buf[body_start..];

    // Parse query + body als form-data (application/x-www-form-urlencoded)
    let mut query = HashMap::new();
    for pair in query_str.split('&').filter(|s| !s.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        query.insert(url_decode(k), url_decode(v));
    }

    if let Some(ct) = headers.get("content-type") {
        if ct.contains("application/x-www-form-urlencoded") {
            let body_str = String::from_utf8_lossy(body_bytes);
            for pair in body_str.split('&').filter(|s| !s.is_empty()) {
                let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
                query.insert(url_decode(k), url_decode(v));
            }
        }
    }

    Some(Request {
        method,
        path: path.to_string(),
        query,
        body: body_bytes.to_vec(),
    })
}

// =====================================================================
// Routing
// =====================================================================

fn route_https(req: &Request, ctx: &HttpCtx, local_ip: &str) -> (&'static str, String) {
    match req.path.as_str() {
        "/serverinfo" => ("200 OK", serverinfo(ctx, local_ip, true)),
        "/pair" => match req.query.get("phrase").map(|s| s.as_str()) {
            Some("getservercert") => pair_getservercert(req, ctx),
            Some("clientchallenge") => pair_clientchallenge(req, ctx),
            Some("serverchallengeresp") => pair_serverchallengeresp(req, ctx),
            Some("clientpairingsecret") => pair_clientpairingsecret(req, ctx),
            _ => ("400 Bad Request", status_xml(400, "unknown pairing phrase")),
        },
        "/applist" => applist(req, ctx),
        "/launch" => launch(req, ctx),
        "/resume" => resume(req, ctx),
        "/cancel" => cancel(req, ctx),
        _ => ("404 Not Found", status_xml(404, "not found")),
    }
}

// =====================================================================
// Serverinfo
// =====================================================================

fn serverinfo(ctx: &HttpCtx, local_ip: &str, https: bool) -> String {
    let busy = ctx.shared.session().is_some();
    let state = if busy {
        "SUNSHINE_SERVER_BUSY"
    } else {
        "SUNSHINE_SERVER_FREE"
    };
    // PairStatus is alleen 1 op HTTPS als de client al paired is.
    // Op plain HTTP is het altijd 0.
    let pair_status = if https { 1 } else { 0 };
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<root status_code=\"200\">\n\
<hostname>{}</hostname>\n\
<appversion>{APP_VERSION}</appversion>\n\
<GfeVersion>{GFE_VERSION}</GfeVersion>\n\
<uniqueid>{}</uniqueid>\n\
<HttpsPort>{}</HttpsPort>\n\
<ExternalPort>{}</ExternalPort>\n\
<MaxLumaPixelsHEVC>0</MaxLumaPixelsHEVC>\n\
<mac>00:00:00:00:00:00</mac>\n\
<LocalIP>{}</LocalIP>\n\
<ServerCodecModeSupport>{CODEC_MODE_SUPPORT}</ServerCodecModeSupport>\n\
<PairStatus>{pair_status}</PairStatus>\n\
<currentgame>{}</currentgame>\n\
<state>{state}</state>\n\
</root>",
        xml_escape(&ctx.hostname),
        xml_escape(&ctx.unique_id),
        ctx.https_port,
        ctx.http_port,
        xml_escape(local_ip),
        if busy { 1 } else { 0 },
    )
}

// =====================================================================
// Pairing: 4 fasen
// =====================================================================
/// Fase 1: getservercert.
fn pair_getservercert(req: &Request, ctx: &HttpCtx) -> (&'static str, String) {
    let salt_hex = req.query.get("salt").cloned().unwrap_or_default();
    let client_cert_hex = req.query.get("clientcert").cloned().unwrap_or_default();
    let unique_id = req.query.get("uniqueid").cloned().unwrap_or_default();
    let client_name = req
        .query
        .get("devicename")
        .cloned()
        .or_else(|| req.query.get("clientname").cloned())
        .unwrap_or_else(|| "Moonlight".into());

    let salt = match hex::decode(&salt_hex) {
        Ok(s) => s,
        Err(e) => {
            return (
                "400 Bad Request",
                status_xml(400, &format!("bad salt: {e}")),
            )
        }
    };

    let client_cert_der = match hex::decode(&client_cert_hex) {
        Ok(c) => c,
        Err(e) => {
            return (
                "400 Bad Request",
                status_xml(400, &format!("bad clientcert: {e}")),
            )
        }
    };

    // Vraag de gebruiker om een PIN.
    let rx = ctx.shared.request_pairing(&client_name, &unique_id);
    let decision = rx.recv_timeout(Duration::from_secs(120));

    let pin = match decision {
        Ok(super::PairDecision::Pin(p)) => p,
        _ => {
            ctx.shared.cancel_pairing(&unique_id);
            return ("403 Forbidden", status_xml(403, "pairing rejected"));
        }
    };

    // Bewaar sessie-state met de afgeleide AES-sleutel.
    let aes_key = pairing::derive_aes_key(&pin, &salt);
    pairing::insert(
        unique_id.clone(),
        pairing::PairingSession {
            client_name: client_name.clone(),
            salt,
            client_cert_der,
            aes_key,
            server_secret: None,
            created: std::time::Instant::now(),
        },
    );

    let identity = super::identity::load_or_create().unwrap();
    let plaincert = match pairing::pem_to_der_hex(&identity.cert_pem) {
        Ok(h) => h,
        Err(e) => {
            return (
                "500 Internal Server Error",
                status_xml(500, &format!("server cert: {e}")),
            )
        }
    };

    let already_paired = false;

    let body = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<root status_code=\"200\">\n\
<plaincert>{plaincert}</plaincert>\n\
<paired>{}</paired>\n\
</root>",
        if already_paired { 1 } else { 0 }
    );
    log::info!(
        "pair: getservercert OK for '{client_name}' (id={unique_id}, paired={already_paired})"
    );
    ("200 OK", body)
}

/// Fase 2: clientchallenge.
/// Fase 2: clientchallenge.
fn pair_clientchallenge(req: &Request, _ctx: &HttpCtx) -> (&'static str, String) {
    let unique_id = req.query.get("uniqueid").cloned().unwrap_or_default();
    let challenge_hex = req
        .query
        .get("clientchallenge")
        .cloned()
        .unwrap_or_default();

    let session = match pairing::get(&unique_id) {
        Some(s) => s,
        None => {
            return ("400 Bad Request", status_xml(400, "no pairing session"));
        }
    };

    let challenge_enc = match hex::decode(&challenge_hex) {
        Ok(c) => c,
        Err(e) => {
            return (
                "400 Bad Request",
                status_xml(400, &format!("bad challenge: {e}")),
            )
        }
    };

    // Decrypt levert 32 bytes = SHA256(originele challenge)
    let challenge_plain = match pairing::aes_ecb_decrypt(&session.aes_key, &challenge_enc) {
        Ok(p) => p,
        Err(e) => {
            return (
                "400 Bad Request",
                status_xml(400, &format!("aes decrypt: {e}")),
            )
        }
    };

    // Onderteken de 32 bytes met RSA-SHA256.
    let identity = super::identity::load_or_create().unwrap();
    let sig = match pairing::rsa_sign_sha256(&identity.key_pem, &challenge_plain) {
        Ok(s) => s,
        Err(e) => {
            return (
                "500 Internal Server Error",
                status_xml(500, &format!("sign: {e}")),
            )
        }
    };

    // Genereer server_secret (16 bytes) en AES-encrypt met de sessie-key.
    let mut server_secret = [0u8; 16];
    rand_bytes(&mut server_secret).unwrap();
    pairing::update_server_secret(&unique_id, server_secret);

    let server_secret_enc = match pairing::aes_ecb_encrypt(&session.aes_key, &server_secret) {
        Ok(c) => c,
        Err(e) => {
            return (
                "500 Internal Server Error",
                status_xml(500, &format!("aes encrypt: {e}")),
            )
        }
    };

    let body = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<root status_code=\"200\">\n\
<challengeresp>{}</challengeresp>\n\
<serverchallengerresp>{}</serverchallengerresp>\n\
</root>",
        hex::encode(&sig),
        hex::encode(&server_secret_enc)
    );
    log::info!("pair: clientchallenge OK for id={unique_id}");
    ("200 OK", body)
}

/// Fase 3: serverchallengeresp.
/// De client stuurt een RSA-handtekening over SHA256(server_secret).
fn pair_serverchallengeresp(req: &Request, _ctx: &HttpCtx) -> (&'static str, String) {
    let unique_id = req.query.get("uniqueid").cloned().unwrap_or_default();
    let resp_hex = req
        .query
        .get("serverchallengeresp")
        .cloned()
        .unwrap_or_default();

    let session = match pairing::get(&unique_id) {
        Some(s) => s,
        None => return ("400 Bad Request", status_xml(400, "no pairing session")),
    };

    let server_secret = match session.server_secret {
        Some(s) => s,
        None => {
            return (
                "400 Bad Request",
                status_xml(400, "no server secret stored"),
            )
        }
    };

    // 1. De client stuurt zijn handtekening over SHA256(server_secret).
    //    Let op: NIET AES-decrypten — het is een RSA signature.
    let client_sig = match hex::decode(&resp_hex) {
        Ok(s) => s,
        Err(e) => {
            return (
                "400 Bad Request",
                status_xml(400, &format!("bad sig hex: {e}")),
            )
        }
    };

    let client_cert_pem = match pairing::der_hex_to_pem(&hex::encode(&session.client_cert_der)) {
        Ok(p) => p,
        Err(e) => {
            return (
                "500 Internal Server Error",
                status_xml(500, &format!("client cert: {e}")),
            )
        }
    };

    // 2. Data die ondertekend moet zijn: SHA256(server_secret)
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(&server_secret);
    let hash = h.finalize();

    // 3. Verifieer de handtekening met het client-certificaat.
    match pairing::rsa_verify_sha256(&client_cert_pem, &hash, &client_sig) {
        Ok(true) => {}
        Ok(false) => {
            pairing::remove(&unique_id);
            return ("403 Forbidden", status_xml(403, "client signature invalid"));
        }
        Err(e) => {
            pairing::remove(&unique_id);
            return (
                "403 Forbidden",
                status_xml(403, &format!("verify error: {e}")),
            );
        }
    }

    // 4. Genereer een server_pairing_secret en versleutel met client pub key (RSA-OAEP-SHA1).
    let mut pairing_secret = [0u8; 16];
    rand_bytes(&mut pairing_secret).unwrap();
    let enc = match pairing::rsa_oaep_encrypt(&client_cert_pem, &pairing_secret) {
        Ok(e) => e,
        Err(e) => {
            return (
                "500 Internal Server Error",
                status_xml(500, &format!("oaep encrypt: {e}")),
            )
        }
    };

    let body = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<root status_code=\"200\">\n\
<pairingsecret>{}</pairingsecret>\n\
</root>",
        hex::encode(&enc)
    );
    log::info!("pair: serverchallengeresp OK for id={unique_id}");
    ("200 OK", body)
}

/// Fase 4: clientpairingsecret.
fn pair_clientpairingsecret(req: &Request, ctx: &HttpCtx) -> (&'static str, String) {
    let unique_id = req.query.get("uniqueid").cloned().unwrap_or_default();
    let secret_hex = req
        .query
        .get("clientpairingsecret")
        .cloned()
        .unwrap_or_default();

    let session = match pairing::get(&unique_id) {
        Some(s) => s,
        None => return ("400 Bad Request", status_xml(400, "no pairing session")),
    };

    let identity = super::identity::load_or_create().unwrap();
    let enc = match hex::decode(&secret_hex) {
        Ok(e) => e,
        Err(e) => return ("400 Bad Request", status_xml(400, &format!("hex: {e}"))),
    };

    // Decrypt met onze private key (RSA-OAEP-SHA1).
    let _client_secret = match pairing::rsa_oaep_decrypt(&identity.key_pem, &enc) {
        Ok(s) => s,
        Err(e) => {
            pairing::remove(&unique_id);
            return (
                "403 Forbidden",
                status_xml(403, &format!("oaep decrypt: {e}")),
            );
        }
    };

    let client_cert_pem = match pairing::der_hex_to_pem(&hex::encode(&session.client_cert_der)) {
        Ok(p) => p,
        Err(e) => {
            return (
                "500 Internal Server Error",
                status_xml(500, &format!("client cert: {e}")),
            )
        }
    };

    ctx.shared.add_paired(super::PairedClient {
        unique_id: unique_id.clone(),
        name: session.client_name.clone(),
        cert_pem: client_cert_pem,
        paired_at: super::unix_now(),
    });

    pairing::remove(&unique_id);

    let body = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<root status_code=\"200\">\n\
<paired>1</paired>\n\
</root>"
        .to_string();

    log::info!("pair: clientpairingsecret OK — '{unique_id}' is nu gepaired");
    ("200 OK", body)
}

/// Fase 3: serverchallengeresp.
// fn pair_serverchallengeresp(req: &Request, _ctx: &HttpCtx) -> (&'static str, String) {
//     let unique_id = req.query.get("uniqueid").cloned().unwrap_or_default();
//     let resp_hex = req
//         .query
//         .get("serverchallengeresp")
//         .cloned()
//         .unwrap_or_default();
//
//     let session = match pairing::get(&unique_id) {
//         Some(s) => s,
//         None => return ("400 Bad Request", status_xml(400, "no pairing session")),
//     };
//
//     let server_secret = match session.server_secret {
//         Some(s) => s,
//         None => {
//             return (
//                 "400 Bad Request",
//                 status_xml(400, "no server secret stored"),
//             )
//         }
//     };
//
//     let client_sig = match hex::decode(&resp_hex) {
//         Ok(s) => s,
//         Err(e) => {
//             return (
//                 "400 Bad Request",
//                 status_xml(400, &format!("bad sig hex: {e}")),
//             )
//         }
//     };
//
//     let client_cert_pem = match pairing::der_hex_to_pem(&hex::encode(&session.client_cert_der)) {
//         Ok(p) => p,
//         Err(e) => {
//             return (
//                 "500 Internal Server Error",
//                 status_xml(500, &format!("client cert: {e}")),
//             )
//         }
//     };
//
//     // Verifieer de handtekening van de client over het server_secret.
//     match pairing::rsa_verify_sha256(&client_cert_pem, &server_secret, &client_sig) {
//         Ok(true) => {}
//         Ok(false) => {
//             pairing::remove(&unique_id);
//             return ("403 Forbidden", status_xml(403, "client signature invalid"));
//         }
//         Err(e) => {
//             pairing::remove(&unique_id);
//             return (
//                 "403 Forbidden",
//                 status_xml(403, &format!("verify error: {e}")),
//             );
//         }
//     }
//
//     // Genereer een pairing_secret en versleutel met de client pub key.
//     let mut pairing_secret = [0u8; 16];
//     rand_bytes(&mut pairing_secret).unwrap();
//     let enc = match pairing::rsa_oaep_encrypt(&client_cert_pem, &pairing_secret) {
//         Ok(e) => e,
//         Err(e) => {
//             return (
//                 "500 Internal Server Error",
//                 status_xml(500, &format!("oaep encrypt: {e}")),
//             )
//         }
//     };
//
//     let body = format!(
//         "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
// <root status_code=\"200\">\n\
// <pairingsecretresp>{}</pairingsecretresp>\n\
// </root>",
//         hex::encode(&enc)
//     );
//     log::info!("pair: serverchallengeresp OK for id={unique_id}");
//     ("200 OK", body)
// }
//
// /// Fase 4: clientpairingsecret.
// fn pair_clientpairingsecret(req: &Request, ctx: &HttpCtx) -> (&'static str, String) {
//     let unique_id = req.query.get("uniqueid").cloned().unwrap_or_default();
//     let secret_hex = req
//         .query
//         .get("clientpairingsecret")
//         .cloned()
//         .unwrap_or_default();
//
//     let session = match pairing::get(&unique_id) {
//         Some(s) => s,
//         None => return ("400 Bad Request", status_xml(400, "no pairing session")),
//     };
//
//     let identity = super::identity::load_or_create().unwrap();
//     let enc = match hex::decode(&secret_hex) {
//         Ok(e) => e,
//         Err(e) => return ("400 Bad Request", status_xml(400, &format!("hex: {e}"))),
//     };
//
//     let _client_secret = match pairing::rsa_oaep_decrypt(&identity.key_pem, &enc) {
//         Ok(s) => s,
//         Err(e) => {
//             pairing::remove(&unique_id);
//             return (
//                 "403 Forbidden",
//                 status_xml(403, &format!("oaep decrypt: {e}")),
//             );
//         }
//     };
//
//     let client_cert_pem = match pairing::der_hex_to_pem(&hex::encode(&session.client_cert_der)) {
//         Ok(p) => p,
//         Err(e) => {
//             return (
//                 "500 Internal Server Error",
//                 status_xml(500, &format!("client cert: {e}")),
//             )
//         }
//     };
//
//     ctx.shared.add_paired(super::PairedClient {
//         unique_id: unique_id.clone(),
//         name: session.client_name.clone(),
//         cert_pem: client_cert_pem,
//         paired_at: super::unix_now(),
//     });
//
//     pairing::remove(&unique_id);
//
//     let body = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
// <root status_code=\"200\">\n\
// <paired>1</paired>\n\
// </root>"
//         .to_string();
//
//     log::info!("pair: clientpairingsecret OK — '{unique_id}' is nu gepaired");
//     ("200 OK", body)
// }

// =====================================================================
// Launch / Resume / Cancel
// =====================================================================

fn launch(req: &Request, ctx: &HttpCtx) -> (&'static str, String) {
    let _app_id = req.query.get("appid").cloned().unwrap_or_default();
    let width = req
        .query
        .get("width")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1920);
    let height = req
        .query
        .get("height")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1080);
    let fps = req
        .query
        .get("fps")
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    let rikey = req.query.get("rikey").cloned().unwrap_or_default();
    let rikeyid = req.query.get("rikeyid").cloned().unwrap_or_default();

    log::info!("launch: {}x{} @ {}fps", width, height, fps);

    // RTSP URL die de client moet gebruiken
    let local_ip = "0.0.0.0"; // simplified
    let rtsp_url = format!("rtsp://{local_ip}:{}/", ctx.rtsp_port);

    ctx.shared.set_session(Some(super::SessionInfo {
        client_name: "Moonlight".into(),
        client_addr: local_ip.into(),
        width,
        height,
        fps,
        bitrate_kbps: 20000,
        started: std::time::SystemTime::now(),
        rtsp_url: rtsp_url.clone(),
    }));

    let body = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<root status_code=\"200\">\n\
<sessionUrl0>{}</sessionUrl0>\n\
<resid>{}</resid>\n\
</root>",
        xml_escape(&rtsp_url),
        rikeyid
    );
    ("200 OK", body)
}

fn resume(req: &Request, ctx: &HttpCtx) -> (&'static str, String) {
    launch(req, ctx)
}

fn cancel(_req: &Request, ctx: &HttpCtx) -> (&'static str, String) {
    ctx.shared.set_session(None);
    ("200 OK", status_xml(200, "cancelled"))
}

// =====================================================================
// Helpers
// =====================================================================

fn status_xml(code: u16, msg: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<root status_code=\"{code}\" status_message=\"{}\"/>",
        xml_escape(msg)
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn read_request(conn: &mut TcpStream) -> Option<Request> {
    let mut buf = Vec::with_capacity(1024);
    let mut tmp = [0u8; 1024];
    loop {
        match conn.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if buf.windows(4).any(|w| w == b"\r\n\r\n") || buf.len() > 16 * 1024 {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let text = String::from_utf8_lossy(&buf);
    let line = text.lines().next()?;
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?;

    let (path, query_str) = match target.split_once('?') {
        Some((p, q)) => (p, q),
        None => (target, ""),
    };

    let mut query = HashMap::new();
    for pair in query_str.split('&').filter(|s| !s.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        query.insert(url_decode(k), url_decode(v));
    }

    Some(Request {
        method,
        path: path.to_string(),
        query,
        body: Vec::new(),
    })
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() + 0 && i + 2 <= bytes.len() - 1 => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(b) => {
                        out.push(b);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(b'%');
                        i += 1;
                    }
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

// =====================================================================
// App list
// =====================================================================

fn applist(_req: &Request, _ctx: &HttpCtx) -> (&'static str, String) {
    // Voor een virtual display is er geen "app" — we bieden een generieke
    // desktop-app aan die de hele virtuele monitor streamt.
    let body = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<root status_code=\"200\">\n\
<App>\n\
<AppTitle>Desktop</AppTitle>\n\
<ID>1</ID>\n\
<IsHdrSupported>0</IsHdrSupported>\n\
</App>\n\
</root>"
        .to_string();
    ("200 OK", body)
}
