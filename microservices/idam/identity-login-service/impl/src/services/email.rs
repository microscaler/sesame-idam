//! Minimal SMTP submission client for transactional auth mail (OTP codes,
//! magic links).
//!
//! Hand-rolled over `std::net::TcpStream` rather than pulling an async mail
//! crate: the target is the in-cluster Mailpit test endpoint (`data`
//! namespace) and, later, a plaintext submission relay — EHLO / MAIL FROM /
//! RCPT TO / DATA / QUIT with strict timeouts is the whole protocol surface
//! we need, and it stays friendly to the may coroutine runtime (short,
//! time-boxed blocking, no executor coupling). A production ESP integration
//! (TLS/auth or HTTPS API) replaces the transport behind [`send_email`]
//! without touching callers.
//!
//! Env (defaults target the Mailpit service in the `data` namespace):
//! - `SMTP_HOST` (`mailpit.data.svc.cluster.local`)
//! - `SMTP_PORT` (`1025`)
//! - `MAIL_FROM` (`no-reply@sesame-idam.dev`)
//! - `SMTP_TIMEOUT_MS` (`5000`)

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use anyhow::{bail, Context, Result};

fn smtp_host() -> String {
    std::env::var("SMTP_HOST").unwrap_or_else(|_| "mailpit.data.svc.cluster.local".to_string())
}

fn smtp_port() -> u16 {
    std::env::var("SMTP_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1025)
}

fn mail_from() -> String {
    std::env::var("MAIL_FROM").unwrap_or_else(|_| "no-reply@sesame-idam.dev".to_string())
}

fn timeout() -> Duration {
    Duration::from_millis(
        std::env::var("SMTP_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5000),
    )
}

/// Read one SMTP reply (handles multi-line `250-…` continuations) and
/// require the expected status code.
fn expect_code(reader: &mut BufReader<TcpStream>, expected: u16) -> Result<()> {
    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .context("SMTP: reading server reply")?;
        if line.len() < 4 {
            bail!("SMTP: short reply: {line:?}");
        }
        let code: u16 = line[..3].parse().context("SMTP: unparseable reply code")?;
        let cont = line.as_bytes()[3] == b'-';
        if !cont {
            if code != expected {
                bail!("SMTP: expected {expected}, got: {}", line.trim_end());
            }
            return Ok(());
        }
    }
}

fn command(
    stream: &mut TcpStream,
    reader: &mut BufReader<TcpStream>,
    cmd: &str,
    expected: u16,
) -> Result<()> {
    stream
        .write_all(format!("{cmd}\r\n").as_bytes())
        .with_context(|| format!("SMTP: sending {cmd}"))?;
    expect_code(reader, expected)
}

/// Send a plain-text email through the configured SMTP endpoint.
///
/// # Errors
///
/// Returns an error on connection failure, timeout, or any unexpected SMTP
/// reply. Callers on the auth path log + audit the failure but keep the
/// HTTP response generic (no provider-status oracle).
pub fn send_email(to: &str, subject: &str, body: &str) -> Result<()> {
    let addr = format!("{}:{}", smtp_host(), smtp_port());
    let mut stream = {
        // connect_timeout needs a resolved SocketAddr; resolve then connect.
        use std::net::ToSocketAddrs;
        let sock = addr
            .to_socket_addrs()
            .with_context(|| format!("SMTP: resolving {addr}"))?
            .next()
            .with_context(|| format!("SMTP: no address for {addr}"))?;
        TcpStream::connect_timeout(&sock, timeout()).with_context(|| format!("SMTP: connect {addr}"))?
    };
    stream.set_read_timeout(Some(timeout()))?;
    stream.set_write_timeout(Some(timeout()))?;
    let mut reader = BufReader::new(stream.try_clone()?);

    expect_code(&mut reader, 220)?;
    command(&mut stream, &mut reader, "EHLO sesame-idam", 250)?;
    command(
        &mut stream,
        &mut reader,
        &format!("MAIL FROM:<{}>", mail_from()),
        250,
    )?;
    command(&mut stream, &mut reader, &format!("RCPT TO:<{to}>"), 250)?;
    command(&mut stream, &mut reader, "DATA", 354)?;

    let mut message = String::new();
    message.push_str(&format!("From: Sesame <{}>\r\n", mail_from()));
    message.push_str(&format!("To: <{to}>\r\n"));
    message.push_str(&format!("Subject: {subject}\r\n"));
    message.push_str("MIME-Version: 1.0\r\n");
    message.push_str("Content-Type: text/plain; charset=utf-8\r\n");
    message.push_str("\r\n");
    for line in body.lines() {
        // Dot-stuffing (RFC 5321 §4.5.2).
        if line.starts_with('.') {
            message.push('.');
        }
        message.push_str(line);
        message.push_str("\r\n");
    }
    stream.write_all(message.as_bytes())?;
    command(&mut stream, &mut reader, ".", 250)?;
    let _ = command(&mut stream, &mut reader, "QUIT", 221);
    Ok(())
}
