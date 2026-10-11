//! Story 17.6: IdP metadata parsing.

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use openssl::asn1::Asn1Time;
use openssl::bn::BigNum;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::x509::{X509Builder, X509NameBuilder};
use sesame_saml_verifier::metadata::parse_metadata;
use sesame_saml_verifier::Refusal;

fn cert(days: u32) -> String {
    let key = PKey::from_rsa(Rsa::generate(2048).expect("rsa")).expect("pkey");
    let mut name = X509NameBuilder::new().expect("name");
    name.append_entry_by_text("CN", "idp").expect("cn");
    let name = name.build();
    let mut b = X509Builder::new().expect("x509");
    b.set_version(2).expect("v3");
    b.set_serial_number(
        &BigNum::from_u32(7)
            .expect("bn")
            .to_asn1_integer()
            .expect("serial"),
    )
    .expect("serial");
    b.set_subject_name(&name).expect("subject");
    b.set_issuer_name(&name).expect("issuer");
    b.set_pubkey(&key).expect("pubkey");
    b.set_not_before(&Asn1Time::days_from_now(0).expect("t"))
        .expect("nb");
    b.set_not_after(&Asn1Time::days_from_now(days).expect("t"))
        .expect("na");
    b.sign(&key, MessageDigest::sha256()).expect("sign");
    STANDARD.encode(b.build().to_der().expect("der"))
}

fn metadata(certs: &[(&str, &str)], sso: &str) -> String {
    let keys: String = certs
        .iter()
        .map(|(usage, c)| {
            let attr = if usage.is_empty() { String::new() } else { format!(r#" use="{usage}""#) };
            format!(r#"<md:KeyDescriptor{attr}><ds:KeyInfo><ds:X509Data><ds:X509Certificate>{c}</ds:X509Certificate></ds:X509Data></ds:KeyInfo></md:KeyDescriptor>"#)
        })
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?><md:EntityDescriptor xmlns:md="urn:oasis:names:tc:SAML:2.0:metadata" xmlns:ds="http://www.w3.org/2000/09/xmldsig#" entityID="https://sts.windows.net/tenant-id/"><md:IDPSSODescriptor protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">{keys}<md:NameIDFormat>urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress</md:NameIDFormat><md:SingleSignOnService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST" Location="https://login.example.test/post"/><md:SingleSignOnService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect" Location="{sso}"/></md:IDPSSODescriptor></md:EntityDescriptor>"#
    )
}

#[test]
fn entra_like_metadata_parses() {
    let (a, b) = (cert(365), cert(30));
    let m = parse_metadata(&metadata(
        &[("signing", &a), ("signing", &b), ("encryption", &cert(10))],
        "https://login.example.test/saml2",
    ))
    .expect("parses");
    assert_eq!(m.entity_id, "https://sts.windows.net/tenant-id/");
    assert_eq!(m.sso_redirect_url, "https://login.example.test/saml2");
    assert_eq!(
        m.signing_certs.len(),
        2,
        "encryption keys are not signing keys"
    );
    assert!(m.signing_certs[1].not_after < m.signing_certs[0].not_after);
    assert_eq!(
        m.name_id_formats,
        vec!["urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress"]
    );
}

#[test]
fn keys_without_use_count_when_none_is_marked_signing() {
    let c = cert(90);
    let m = parse_metadata(&metadata(&[("", &c)], "https://idp.test/sso")).expect("parses");
    assert_eq!(m.signing_certs.len(), 1);
}

#[test]
fn unusable_metadata_is_refused() {
    let c = cert(90);
    assert_eq!(
        parse_metadata(&metadata(&[("signing", &c)], "http://idp.test/sso")),
        Err(Refusal::Metadata),
        "sign-on must be https"
    );
    assert_eq!(
        parse_metadata(&metadata(&[], "https://idp.test/sso")),
        Err(Refusal::NoCertificate)
    );
    assert_eq!(
        parse_metadata(&metadata(
            &[("signing", "!!notbase64")],
            "https://idp.test/sso"
        )),
        Err(Refusal::NoCertificate)
    );
    let doctype = metadata(&[("signing", &c)], "https://idp.test/sso").replacen(
        "<md:EntityDescriptor",
        "<!DOCTYPE x [<!ENTITY e SYSTEM \"file:///etc/passwd\">]><md:EntityDescriptor",
        1,
    );
    assert_eq!(parse_metadata(&doctype), Err(Refusal::Doctype));
    assert_eq!(
        parse_metadata(&"x".repeat(1024 * 1024 + 1)),
        Err(Refusal::TooLarge)
    );
    assert_eq!(parse_metadata("<notmetadata/>"), Err(Refusal::Metadata));
}
