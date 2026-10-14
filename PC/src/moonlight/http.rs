//! Tiny HTTP/1.1 server for the plain (47989) Moonlight endpoint.
//! Handlers are written against `Request` / `HttpCtx` so the future TLS
//! listener can reuse the same router.

use super::HostShared;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub const APP_VERSION: &str = "7.1.431.-1";
pub const GFE_VERSION: &str = "3.23.0.74";
pub const HTTPS_PORT: u16 = 47984;
/// H.264 only.
pub const CODEC_MODE_SUPPORT: u32 = 0x1;

pub struct HttpCtx {
    pub hostname: String,
    pub unique_id: String,
    pub http_port: u16,
    pub shared: HostShared,
}

pub struct Request {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
}

pub fn handle(mut conn: TcpStream, ctx: &HttpCtx) {
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

    let (status_line, body) = route(&req, ctx, &local_ip);
    let resp = format!(
        "HTTP/1.1 {status_line}\r\nContent-Type: text/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = conn.write_all(resp.as_bytes());
    let _ = conn.flush();
}

fn route(req: &Request, ctx: &HttpCtx, local_ip: &str) -> (&'static str, String) {
    match req.path.as_str() {
        "/serverinfo" => ("200 OK", serverinfo(ctx, local_ip)),
        "/pair" | "/unpair" | "/launch" | "/resume" | "/cancel" | "/applist" => {
            ("200 OK", status_xml(503, "not implemented yet"))
        }
        _ => ("404 Not Found", status_xml(404, "not found")),
    }
}

fn serverinfo(ctx: &HttpCtx, local_ip: &str) -> String {
    let busy = ctx.shared.session().is_some();
    let state = if busy {
        "SUNSHINE_SERVER_BUSY"
    } else {
        "SUNSHINE_SERVER_FREE"
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<root status_code=\"200\">\n\
<hostname>{}</hostname>\n\
<appversion>{APP_VERSION}</appversion>\n\
<GfeVersion>{GFE_VERSION}</GfeVersion>\n\
<uniqueid>{}</uniqueid>\n\
<HttpsPort>{HTTPS_PORT}</HttpsPort>\n\
<ExternalPort>{}</ExternalPort>\n\
<MaxLumaPixelsHEVC>0</MaxLumaPixelsHEVC>\n\
<mac>00:00:00:00:00:00</mac>\n\
<LocalIP>{}</LocalIP>\n\
<ServerCodecModeSupport>{CODEC_MODE_SUPPORT}</ServerCodecModeSupport>\n\
<PairStatus>0</PairStatus>\n\
<currentgame>{}</currentgame>\n\
<state>{state}</state>\n\
</root>",
        xml_escape(&ctx.hostname),
        xml_escape(&ctx.unique_id),
        ctx.http_port,
        xml_escape(local_ip),
        if busy { 1 } else { 0 },
    )
}

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
