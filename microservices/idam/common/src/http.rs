//! Outbound HTTP for Sesame-IDAM — delegates to [`brrtrouter::http`].
//!
//! All inter-service and JWKS fetch paths must use this module (not `reqwest` or
//! direct `may_http`). BRRTRouter wraps `may_http` for plain HTTP and rustls for HTTPS.

pub use brrtrouter::http::{
    fetch_get, fetch_get_text_with_retry, fetch_post, HttpFetchError, HttpFetchOptions,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::Duration;

    fn read_request(stream: &mut TcpStream) -> String {
        let mut buf = [0u8; 2048];
        let n = stream.read(&mut buf).unwrap_or(0);
        String::from_utf8_lossy(&buf[..n]).into_owned()
    }

    #[test]
    fn fetch_get_via_brrtrouter_http() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{}:{}/jwks", addr.ip(), addr.port());
        let server = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let _ = read_request(&mut stream);
                let body = r#"{"keys":[]}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(resp.as_bytes()).unwrap();
            }
        });

        let options = HttpFetchOptions {
            timeout: Duration::from_secs(2),
            max_body_bytes: 4096,
            extra_headers: Vec::new(),
        };
        let (status, body) = fetch_get(&url, &options).unwrap();
        assert_eq!(status, 200);
        assert!(body.windows(6).any(|w| w == b"\"keys\""));
        server.join().ok();
    }

    #[test]
    fn fetch_post_via_brrtrouter_http() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{}:{}/authz/principals/effective", addr.ip(), addr.port());
        let server = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let req = read_request(&mut stream);
                assert!(req.contains("POST "));
                assert!(req.to_ascii_lowercase().contains("content-type: application/json"));
                let body = r#"{"roles":[{"role":"OWNER"}]}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(resp.as_bytes()).unwrap();
            }
        });

        let payload = br#"{"user_id":"u1","tenant_id":"t1"}"#;
        let options = HttpFetchOptions {
            timeout: Duration::from_secs(2),
            max_body_bytes: 4096,
            extra_headers: vec![(
                "content-type".to_string(),
                "application/json".to_string(),
            )],
        };
        let (status, body) = fetch_post(&url, payload, &options).unwrap();
        assert_eq!(status, 200);
        assert!(body.windows(7).any(|w| w == b"\"OWNER\""));
        server.join().ok();
    }
}
