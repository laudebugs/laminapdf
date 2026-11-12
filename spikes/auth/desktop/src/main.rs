//! The desktop half of the H1 sign-in (RFC 8252 loopback redirect with PKCE, RFC 7636), as
//! Lamina PDF's Rust main process would run it: listen on 127.0.0.1 on a free port, open the
//! sign-in page in the system browser (here: print its address for check.mjs to open), wait
//! for one request carrying the code, trade code + verifier for a bearer session token, then
//! read the session with it. Std only besides three hashing and encoding crates; the token
//! exchange is plain HTTP to the local Worker (a real build would use an HTTPS client).

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use sha2::{Digest as _, Sha256};
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn random(len: usize) -> Result<String> {
    let mut bytes = vec![0u8; len];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
    Ok(B64.encode(bytes))
}

/// One HTTP/1.1 request to the local Worker; returns the body.
fn http(host: &str, method: &str, path: &str, headers: &str, body: &str) -> Result<String> {
    let mut stream = TcpStream::connect(host)?;
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{headers}\r\n{body}",
        body.len()
    )?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    let (_, body) = response.split_once("\r\n\r\n").ok_or("no body")?;
    Ok(body.to_owned())
}

/// The value of `name` in a query string, percent-decoding only what the Worker writes.
fn query<'a>(query: &'a str, name: &str) -> Option<&'a str> {
    query.split('&').find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
}

fn main() -> Result<()> {
    let base = std::env::args().nth(1).unwrap_or_else(|| "localhost:8787".to_owned());
    let verifier = random(32)?;
    let challenge = B64.encode(Sha256::digest(verifier.as_bytes()));
    let state = random(16)?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let redirect = format!("http%3A%2F%2F127.0.0.1%3A{port}%2Fcb");
    // Lamina PDF would hand this to tauri-plugin-opener from Rust (ADR 0008 §5): the system browser.
    println!("http://{base}/api/desktop/authorize?redirect_uri={redirect}&code_challenge={challenge}&state={state}");

    // Take connections until one asks for /cb: a browser may open a speculative socket first and
    // send nothing on it, and another local program may connect too. Each gets two seconds to
    // send its request line; the whole wait ends after two minutes.
    let started = Instant::now();
    listener.set_nonblocking(true)?;
    let line = loop {
        if started.elapsed() > Duration::from_secs(120) {
            return Err("no sign-in within two minutes".into());
        }
        let mut stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(20));
                continue;
            }
            Err(e) => return Err(e.into()),
        };
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        let mut line = String::new();
        if BufReader::new(&stream).read_line(&mut line).is_err() || !line.starts_with("GET /cb?") {
            continue;
        }
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nYou're signed in. You can close this tab and go back to Lamina PDF.")?;
        break line;
    };
    drop(listener);

    let target = line.split_whitespace().nth(1).ok_or("bad request line")?;
    let params = target.split_once('?').map(|(_, q)| q).ok_or("no query")?;
    if query(params, "state") != Some(state.as_str()) {
        return Err("state mismatch".into());
    }
    let code = query(params, "code").ok_or("no code")?;
    let token_body = http(
        &base,
        "POST",
        "/api/desktop/token",
        "",
        &format!(r#"{{"code":"{code}","code_verifier":"{verifier}"}}"#),
    )?;
    let token = token_body
        .split(r#""token":""#)
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .ok_or_else(|| format!("no token in {token_body}"))?;
    // The app would keep `token` in the OS keychain, never in the webview.
    let session = http(&base, "GET", "/api/auth/get-session", &format!("Authorization: Bearer {token}\r\n"), "")?;
    println!("SESSION {session}");
    println!("ELAPSED_MS {}", started.elapsed().as_millis());
    Ok(())
}
