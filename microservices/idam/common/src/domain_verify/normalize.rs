//! Canonical domain form: lower-case IDNA A-label (UTS-46, non-transitional), no trailing dot.

use std::fmt;
use std::net::IpAddr;

use super::DomainError;

/// Longest name in presentation form without the trailing dot (RFC 1035 255 octets on the wire).
const MAX_NAME: usize = 253;
/// Longest label (RFC 1035).
const MAX_LABEL: usize = 63;
/// Upper bound on raw input before any processing.
const MAX_INPUT: usize = 1024;

/// A normalised domain name. Only [`normalize_domain`] creates one, so holding a `Domain` means the
/// name is canonical: compare and store it as is.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Domain(String);

impl Domain {
    /// The canonical A-label form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Labels from left to right.
    #[must_use]
    pub fn labels(&self) -> impl DoubleEndedIterator<Item = &str> {
        self.0.split('.')
    }

    /// True when `self` equals `parent` or is a subdomain of it (label-aligned: `ab.com` is not under `b.com`).
    #[must_use]
    pub fn is_same_or_under(&self, parent: &Domain) -> bool {
        self.0 == parent.0
            || (self.0.len() > parent.0.len()
                && self.0.ends_with(parent.0.as_str())
                && self.0.as_bytes()[self.0.len() - parent.0.len() - 1] == b'.')
    }
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Domain({})", self.0)
    }
}

impl AsRef<str> for Domain {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Normalise user input into a canonical [`Domain`].
///
/// Trims surrounding whitespace, strips one trailing dot, maps Unicode to the IDNA A-label with UTS-46
/// (non-transitional, STD3 rules, which also lower-cases), and refuses IP literals, empty labels, labels over
/// 63 octets, names over 253 octets, characters outside letters, digits and hyphen, and an all-numeric
/// top-level label.
///
/// # Errors
///
/// [`DomainError::InvalidDomain`] with a fixed reason.
pub fn normalize_domain(input: &str) -> Result<Domain, DomainError> {
    if input.len() > MAX_INPUT {
        return Err(DomainError::InvalidDomain("too long"));
    }
    let trimmed = input.trim();
    let trimmed = trimmed.strip_suffix('.').unwrap_or(trimmed);
    if trimmed.is_empty() {
        return Err(DomainError::InvalidDomain("empty"));
    }
    let bare = trimmed
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(trimmed);
    if bare.parse::<IpAddr>().is_ok() {
        return Err(DomainError::InvalidDomain("IP address"));
    }
    let ascii = idna::domain_to_ascii_strict(trimmed)
        .map_err(|_| DomainError::InvalidDomain("not a valid IDNA host name"))?;
    check_syntax(&ascii)?;
    Ok(Domain(ascii))
}

fn check_syntax(name: &str) -> Result<(), DomainError> {
    if name.is_empty() {
        return Err(DomainError::InvalidDomain("empty"));
    }
    if name.len() > MAX_NAME {
        return Err(DomainError::InvalidDomain("name over 253 octets"));
    }
    let mut last = "";
    for label in name.split('.') {
        if label.is_empty() {
            return Err(DomainError::InvalidDomain("empty label"));
        }
        if label.len() > MAX_LABEL {
            return Err(DomainError::InvalidDomain("label over 63 octets"));
        }
        if !label
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(DomainError::InvalidDomain("character outside LDH"));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(DomainError::InvalidDomain(
                "label starts or ends with hyphen",
            ));
        }
        last = label;
    }
    if last.bytes().all(|b| b.is_ascii_digit()) {
        return Err(DomainError::InvalidDomain("numeric top-level label"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(input: &str) -> String {
        normalize_domain(input)
            .unwrap_or_else(|e| panic!("{input}: {e}"))
            .as_str()
            .to_string()
    }

    fn err(input: &str) -> DomainError {
        normalize_domain(input).expect_err(input)
    }

    #[test]
    fn unicode_mixed_case_and_trailing_dot() {
        assert_eq!(ok("BüCher.Example."), "xn--bcher-kva.example");
        assert_eq!(ok("  BigBank.COM \n"), "bigbank.com");
        assert_eq!(ok("faß.de"), "xn--fa-hia.de");
        assert_eq!(ok("ＢＩＧＢＡＮＫ．ｃｏｍ"), "bigbank.com");
    }

    #[test]
    fn refuses_ip_literals() {
        for ip in ["1.2.3.4", "::1", "[2001:db8::1]", "10.0.0.1."] {
            assert_eq!(err(ip).code(), "invalid_domain", "{ip}");
        }
        assert_eq!(err("bigbank.123").code(), "invalid_domain");
    }

    #[test]
    fn refuses_bad_shapes() {
        let long_label = format!("{}.com", "a".repeat(64));
        let long_name = format!("{}.com", vec!["a".repeat(62); 4].join("."));
        assert!(long_name.len() > 253);
        for bad in [
            "",
            ".",
            "a..b",
            ".bigbank.com",
            "bigbank.com..",
            "exa_mple.com",
            "-a.com",
            "a-.com",
            "big bank.com",
            "bigbank.com/x",
            "user@bigbank.com",
            "xn--zz.com",
            long_label.as_str(),
            long_name.as_str(),
        ] {
            assert_eq!(err(bad).code(), "invalid_domain", "{bad:?}");
        }
    }

    #[test]
    fn longest_valid_shapes_pass() {
        let label63 = format!("{}.com", "a".repeat(63));
        assert_eq!(ok(&label63), label63);
        // 3 x 63 + 57 + 3 dots + "com" = 253 would need care; 4 labels of 61 + "com" = 4*61 + 4 + 3 = 251.
        let name = format!("{}.com", vec!["b".repeat(61); 4].join("."));
        assert!(name.len() <= 253);
        assert_eq!(ok(&name), name);
    }

    #[test]
    fn same_or_under_is_label_aligned() {
        let parent = normalize_domain("bank.com").expect("valid");
        assert!(normalize_domain("bank.com")
            .expect("valid")
            .is_same_or_under(&parent));
        assert!(normalize_domain("eu.bank.com")
            .expect("valid")
            .is_same_or_under(&parent));
        assert!(!normalize_domain("bigbank.com")
            .expect("valid")
            .is_same_or_under(&parent));
        assert!(!normalize_domain("com")
            .expect("valid")
            .is_same_or_under(&parent));
    }

    #[test]
    fn errors_never_echo_input() {
        let e = err("secret-ish..value.com");
        assert!(!e.to_string().contains("secret-ish"));
    }
}
