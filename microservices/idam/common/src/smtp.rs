//! Minimal SMTP submission client for transactional mail (OTP, magic links, invites).
//!
//! Hand-rolled over `std::net::TcpStream` for the may coroutine runtime (short,
//! time-boxed blocking).
//!
//! Env:
//! - `SMTP_HOST`, `MAIL_FROM`: required; in development (`SESAME_DEV_MODE=1`) they default to the in-cluster
//!   Mailpit (`mailpit.data.svc.cluster.local`) and `no-reply@sesame-idam.dev`. Unset elsewhere, a send fails.
//! - `MAIL_FROM_NAME`: the From display name (`Sesame`)
//! - `SMTP_PORT` (`1025`)
//! - `SMTP_TIMEOUT_MS` (`5000`)
//!
//! Mail to a reserved, non-deliverable domain (RFC 2606 / RFC 6761: `example.com`, `example.net`, `example.org` and
//! their subdomains; the `.test`, `.invalid`, `.localhost` and `.example` TLDs) is never handed to the relay: it
//! can only bounce, and bounces cost the sending domain its reputation. [`send_email`] logs it with the local part
//! redacted, counts it (`sesame_mail_send_total{outcome="suppressed"}`) and returns `Ok`, so the flow that asked for
//! the mail (registration, a sign-in code, a security notice) completes as if it had been sent.
//!
//! Capture (pre-production only): with `MAIL_CAPTURE_DOMAIN` set (pp: `mail.pp.test`), mail to that domain or its
//! subdomains is handed to a capture-only sink instead of the relay: `MAIL_CAPTURE_SMTP_HOST` / `MAIL_CAPTURE_SMTP_PORT`
//! (Mailpit; default `SMTP_HOST` / `SMTP_PORT`). Test users live there, and their mail can be read back from the sink's
//! API. Unset (production), the capture domain is just another reserved `.test` domain and its mail is dropped.
//! `MAIL_ALLOW_RESERVED_DOMAINS=1` turns the reserved-domain guard off, for a capture-only sink (the development Mailpit and the BDD
//! tests that read mail back from it); never set it where `SMTP_HOST` is a real relay.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use anyhow::{bail, Context, Result};

fn required(name: &str, dev_default: &str) -> Result<String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Ok(v.trim().to_string()),
        _ if crate::deployment::dev_mode() => Ok(dev_default.to_string()),
        _ => bail!("{name} is not set: no mail is sent"),
    }
}

fn smtp_host() -> Result<String> {
    required("SMTP_HOST", "mailpit.data.svc.cluster.local")
}

fn smtp_port() -> u16 {
    std::env::var("SMTP_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1025)
}

fn mail_from() -> Result<String> {
    required("MAIL_FROM", "no-reply@sesame-idam.dev")
}

/// `MAIL_FROM_NAME`, the display name on the From header (Sesame when unset). Printable ASCII only, quoted: a
/// name can neither break the header nor need RFC 2047 encoding.
fn from_header(addr: &str) -> String {
    let name: String = std::env::var("MAIL_FROM_NAME")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "Sesame".into())
        .chars()
        .filter(|c| (' '..='~').contains(c) && !matches!(c, '"' | '\\' | '<' | '>'))
        .collect();
    format!("\"{}\" <{addr}>", name.trim())
}

fn timeout() -> Duration {
    Duration::from_millis(
        std::env::var("SMTP_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5000),
    )
}

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
/// Returns an error on connection failure, timeout, or unexpected SMTP reply.
pub fn send_email(to: &str, subject: &str, body: &str) -> Result<()> {
    let captured = capture_domain().is_some_and(|domain| in_domain(to, &domain));
    if !captured && is_reserved_recipient(to) && !reserved_domains_allowed() {
        let kind = mail_kind(subject);
        crate::observability::metrics::MAIL_SENDS
            .with_label_values(&[kind, "suppressed"])
            .inc();
        tracing::warn!(
            kind,
            outcome = "suppressed",
            to = %redact_local_part(to),
            "sesame_mail: reserved, non-deliverable domain (RFC 2606/6761); not sent"
        );
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = (if captured {
        capture_addr()
    } else {
        relay_addr()
    })
    .and_then(|addr| send_email_inner(&addr, to, subject, body));
    crate::observability::metrics::MAIL_DURATION.observe(started.elapsed().as_secs_f64());
    let outcome = match &result {
        Ok(()) if captured => "captured",
        Ok(()) => "sent",
        Err(e) => classify_failure(e),
    };
    let kind = mail_kind(subject);
    crate::observability::metrics::MAIL_SENDS
        .with_label_values(&[kind, outcome])
        .inc();
    // The recipient's domain only: enough to see a provider refusing, never the address.
    let domain = to.rsplit_once('@').map_or("", |(_, d)| d);
    match &result {
        Ok(()) => tracing::info!(kind, outcome, to_domain = domain, "sesame_mail"),
        Err(error) => {
            crate::observability::metrics::DEPENDENCY_ERRORS
                .with_label_values(&["smtp", "mail"])
                .inc();
            tracing::warn!(kind, outcome, to_domain = domain, error = %format!("{error:#}"), "sesame_mail");
        }
    }
    result
}

/// Second-level domains reserved for documentation (RFC 2606 §3); they and all their subdomains are non-deliverable.
const RESERVED_DOMAINS: [&str; 3] = ["example.com", "example.net", "example.org"];
/// Top-level domains that never resolve to a mail host (RFC 2606 §2, RFC 6761 §6).
const RESERVED_TLDS: [&str; 4] = ["test", "invalid", "localhost", "example"];

/// Whether `to` is at a reserved, non-deliverable domain (a subdomain counts; case and a trailing dot are ignored).
/// An address without a domain is not judged here: it fails at the relay as before.
#[must_use]
pub fn is_reserved_recipient(to: &str) -> bool {
    let Some((_, domain)) = to.trim().trim_end_matches('>').rsplit_once('@') else {
        return false;
    };
    let domain = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    if domain.is_empty() {
        return false;
    }
    let tld = domain.rsplit('.').next().unwrap_or_default();
    RESERVED_TLDS.contains(&tld)
        || RESERVED_DOMAINS
            .iter()
            .any(|r| domain == *r || domain.ends_with(&format!(".{r}")))
}

/// `MAIL_CAPTURE_DOMAIN`, normalised (lowercase, no trailing dot); `None` when unset (production: no capture).
fn capture_domain() -> Option<String> {
    std::env::var("MAIL_CAPTURE_DOMAIN")
        .ok()
        .map(|v| v.trim().trim_end_matches('.').to_ascii_lowercase())
        .filter(|v| !v.is_empty())
}

/// Whether `to` is at `domain` or one of its subdomains (case and a trailing dot ignored).
#[must_use]
pub fn in_domain(to: &str, domain: &str) -> bool {
    let Some((_, d)) = to.trim().rsplit_once('@') else {
        return false;
    };
    let d = d.trim().trim_end_matches('.').to_ascii_lowercase();
    let domain = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    !domain.is_empty() && (d == domain || d.ends_with(&format!(".{domain}")))
}

/// The relay: `SMTP_HOST:SMTP_PORT`.
fn relay_addr() -> Result<String> {
    Ok(format!("{}:{}", smtp_host()?, smtp_port()))
}

/// The capture sink: `MAIL_CAPTURE_SMTP_HOST:MAIL_CAPTURE_SMTP_PORT`, each defaulting to the relay's.
fn capture_addr() -> Result<String> {
    let host = match std::env::var("MAIL_CAPTURE_SMTP_HOST") {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => smtp_host()?,
    };
    let port = std::env::var("MAIL_CAPTURE_SMTP_PORT")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or_else(smtp_port);
    Ok(format!("{host}:{port}"))
}

/// `MAIL_ALLOW_RESERVED_DOMAINS` is `1` or `true`: the relay is a capture-only sink (Mailpit), nothing can bounce.
fn reserved_domains_allowed() -> bool {
    std::env::var("MAIL_ALLOW_RESERVED_DOMAINS")
        .is_ok_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true"))
}

/// `***@domain`: the address with its local part removed, safe to log.
#[must_use]
pub fn redact_local_part(to: &str) -> String {
    to.trim()
        .rsplit_once('@')
        .map_or_else(|| "***".to_string(), |(_, d)| format!("***@{d}"))
}

/// The kind of message, from its subject line, as a metric label (the subject itself is never logged).
fn mail_kind(subject: &str) -> &'static str {
    let s = subject.to_ascii_lowercase();
    if s.starts_with("security alert") {
        "security_notice"
    } else if s.contains("password") {
        "password_reset"
    } else if s.contains("verification code") || s.contains("code") {
        "otp"
    } else if s.contains("sign-in link") || s.contains("magic") {
        "magic_link"
    } else if s.contains("ownership") {
        "owner_transfer"
    } else if s.contains("invit") {
        "invite"
    } else {
        "other"
    }
}

/// `refused` when the relay answered with an SMTP error (a 4xx/5xx reply: Gmail's 451 arrives this way),
/// `unreachable` when it could not be reached at all, otherwise `error`.
fn classify_failure(error: &anyhow::Error) -> &'static str {
    let text = format!("{error:#}");
    if text.contains("SMTP: expected") {
        "refused"
    } else if text.contains("SMTP: connect")
        || text.contains("SMTP: resolving")
        || text.contains("SMTP: no address")
    {
        "unreachable"
    } else {
        "error"
    }
}

fn send_email_inner(addr: &str, to: &str, subject: &str, body: &str) -> Result<()> {
    let from = mail_from()?;
    let mut stream = {
        use std::net::ToSocketAddrs;
        let sock = addr
            .to_socket_addrs()
            .with_context(|| format!("SMTP: resolving {addr}"))?
            .next()
            .with_context(|| format!("SMTP: no address for {addr}"))?;
        TcpStream::connect_timeout(&sock, timeout())
            .with_context(|| format!("SMTP: connect {addr}"))?
    };
    stream.set_read_timeout(Some(timeout()))?;
    stream.set_write_timeout(Some(timeout()))?;
    let mut reader = BufReader::new(stream.try_clone()?);

    expect_code(&mut reader, 220)?;
    command(&mut stream, &mut reader, "EHLO sesame-idam", 250)?;
    command(
        &mut stream,
        &mut reader,
        &format!("MAIL FROM:<{from}>"),
        250,
    )?;
    command(&mut stream, &mut reader, &format!("RCPT TO:<{to}>"), 250)?;
    command(&mut stream, &mut reader, "DATA", 354)?;

    let mut message = String::new();
    message.push_str(&format!("From: {}\r\n", from_header(&from)));
    message.push_str(&format!("To: <{to}>\r\n"));
    message.push_str(&format!("Subject: {subject}\r\n"));
    message.push_str("MIME-Version: 1.0\r\n");
    message.push_str("Content-Type: text/plain; charset=utf-8\r\n");
    message.push_str("\r\n");
    for line in body.lines() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_domains_and_their_subdomains_are_refused() {
        for to in [
            "a@example.com",
            "mfa-verify-1-a@example.com",
            "A@EXAMPLE.ORG",
            "x@example.net.",
            "x@mail.example.com",
            "x@deep.sub.example.org",
            "x@host.test",
            "x@thing.invalid",
            "x@localhost",
            "x@box.localhost",
            "x@login.sesame.example",
            "x@EXAMPLE",
            " x@example.com ",
        ] {
            assert!(is_reserved_recipient(to), "{to}");
        }
    }

    #[test]
    fn deliverable_and_lookalike_domains_are_sent() {
        for to in [
            "a@gmail.com",
            "a@pricewhisperer.ai",
            "a@notexample.com",
            "a@example.com.au",
            "a@example.co.uk",
            "a@myexample.org",
            "a@testing.io",
            "a@test.com",
            "a@invalid.net",
            "a@example.community",
            "no-domain",
            "trailing@",
        ] {
            assert!(!is_reserved_recipient(to), "{to}");
        }
    }

    #[test]
    fn security_notices_have_their_own_kind() {
        assert_eq!(
            mail_kind("Security alert: a recovery code was used to sign in"),
            "security_notice"
        );
        assert_eq!(mail_kind("Your verification code"), "otp");
    }

    #[test]
    fn the_capture_domain_matches_itself_and_its_subdomains_only() {
        assert!(in_domain("mfa+a@mail.pp.test", "mail.pp.test"));
        assert!(in_domain("X@MAIL.PP.TEST.", "mail.pp.test."));
        assert!(in_domain("x@run1.mail.pp.test", "mail.pp.test"));
        assert!(!in_domain("x@pp.test", "mail.pp.test"));
        assert!(!in_domain("x@evilmail.pp.test", "mail.pp.test"));
        assert!(!in_domain("x@mail.pp.test.example.com", "mail.pp.test"));
        assert!(!in_domain("x@mail.pp.test", ""));
        assert!(!in_domain("no-domain", "mail.pp.test"));
    }

    #[test]
    fn the_local_part_never_reaches_the_log() {
        assert_eq!(
            redact_local_part("alice.smith@example.com"),
            "***@example.com"
        );
        assert_eq!(redact_local_part("no-domain"), "***");
    }

    #[test]
    fn a_reserved_recipient_is_reported_sent_without_touching_the_relay() {
        // No SMTP_HOST in the test environment: a real send would fail. Suppression answers Ok and counts it.
        let before = crate::observability::metrics::MAIL_SENDS
            .with_label_values(&["otp", "suppressed"])
            .get();
        assert!(send_email("someone@example.com", "Your verification code", "123456").is_ok());
        let after = crate::observability::metrics::MAIL_SENDS
            .with_label_values(&["otp", "suppressed"])
            .get();
        assert_eq!(after, before + 1);
    }
}
