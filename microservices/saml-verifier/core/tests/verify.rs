//! Story 17.6: the verifier against responses signed at test time with a throwaway IdP key (nothing private is
//! committed). Each refusal rule has at least one case; the full adversarial corpus is story 17.7.

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use chrono::{Duration, SecondsFormat, Utc};
use openssl::asn1::Asn1Time;
use openssl::bn::BigNum;
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::x509::{X509Builder, X509NameBuilder};
use samael::crypto::{Crypto, CryptoProvider};
use sesame_saml_verifier::{verify, Refusal, VerifyRequest};

const IDP: &str = "https://idp.example.test/entity";
const SP: &str = "https://sesame.example.test/saml/sp/conn-1";
const ACS: &str = "https://sesame.example.test/auth/saml/acs/conn-1";
const REQ: &str = "_req-0001";

struct Idp {
    key: PKey<Private>,
    cert_b64: String,
}

fn idp() -> Idp {
    let key = PKey::from_rsa(Rsa::generate(2048).expect("rsa")).expect("pkey");
    let mut name = X509NameBuilder::new().expect("name");
    name.append_entry_by_text("CN", "test-idp").expect("cn");
    let name = name.build();
    let mut b = X509Builder::new().expect("x509");
    b.set_version(2).expect("v3");
    b.set_serial_number(
        &BigNum::from_u32(1)
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
    b.set_not_after(&Asn1Time::days_from_now(30).expect("t"))
        .expect("na");
    b.sign(&key, MessageDigest::sha256()).expect("sign");
    let cert_b64 = STANDARD.encode(b.build().to_der().expect("der"));
    Idp { key, cert_b64 }
}

/// Knobs for one response.
struct Opts<'a> {
    audience: &'a str,
    recipient: &'a str,
    in_response_to: &'a str,
    status: &'a str,
    name_id: &'a str,
    not_on_or_after: chrono::DateTime<Utc>,
    authn_statement: bool,
    sig_alg: &'a str,
    digest_alg: &'a str,
}

impl Default for Opts<'_> {
    fn default() -> Self {
        Self {
            audience: SP,
            recipient: ACS,
            in_response_to: REQ,
            status: "urn:oasis:names:tc:SAML:2.0:status:Success",
            name_id: "alice@bigbank.com",
            not_on_or_after: Utc::now() + Duration::minutes(5),
            authn_statement: true,
            sig_alg: "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256",
            digest_alg: "http://www.w3.org/2001/04/xmlenc#sha256",
        }
    }
}

fn ts(t: chrono::DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// A response with a signed assertion (the Entra ID default), signed now with `idp`.
fn signed(idp: &Idp, o: &Opts<'_>) -> String {
    let now = ts(Utc::now());
    let nooa = ts(o.not_on_or_after);
    let authn = if o.authn_statement {
        format!(
            r#"<saml:AuthnStatement AuthnInstant="{now}" SessionIndex="_s1"><saml:AuthnContext><saml:AuthnContextClassRef>urn:oasis:names:tc:SAML:2.0:ac:classes:Password</saml:AuthnContextClassRef></saml:AuthnContext></saml:AuthnStatement>"#
        )
    } else {
        String::new()
    };
    let xml = format!(
        r##"<?xml version="1.0"?>
<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" ID="_resp1" Version="2.0" IssueInstant="{now}" Destination="{ACS}" InResponseTo="{irt}"><saml:Issuer>{IDP}</saml:Issuer><samlp:Status><samlp:StatusCode Value="{status}"/></samlp:Status><saml:Assertion ID="_a1" Version="2.0" IssueInstant="{now}"><saml:Issuer>{IDP}</saml:Issuer><ds:Signature xmlns:ds="http://www.w3.org/2000/09/xmldsig#"><ds:SignedInfo><ds:CanonicalizationMethod Algorithm="http://www.w3.org/2001/10/xml-exc-c14n#"/><ds:SignatureMethod Algorithm="{sig}"/><ds:Reference URI="#_a1"><ds:Transforms><ds:Transform Algorithm="http://www.w3.org/2000/09/xmldsig#enveloped-signature"/><ds:Transform Algorithm="http://www.w3.org/2001/10/xml-exc-c14n#"/></ds:Transforms><ds:DigestMethod Algorithm="{dig}"/><ds:DigestValue></ds:DigestValue></ds:Reference></ds:SignedInfo><ds:SignatureValue></ds:SignatureValue></ds:Signature><saml:Subject><saml:NameID Format="urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress">{name}</saml:NameID><saml:SubjectConfirmation Method="urn:oasis:names:tc:SAML:2.0:cm:bearer"><saml:SubjectConfirmationData NotOnOrAfter="{nooa}" Recipient="{rcpt}" InResponseTo="{irt}"/></saml:SubjectConfirmation></saml:Subject><saml:Conditions NotBefore="{now}" NotOnOrAfter="{nooa}"><saml:AudienceRestriction><saml:Audience>{aud}</saml:Audience></saml:AudienceRestriction></saml:Conditions>{authn}<saml:AttributeStatement><saml:Attribute Name="email"><saml:AttributeValue>{name}</saml:AttributeValue></saml:Attribute><saml:Attribute Name="groups"><saml:AttributeValue>admins</saml:AttributeValue><saml:AttributeValue>staff</saml:AttributeValue></saml:Attribute></saml:AttributeStatement></saml:Assertion></samlp:Response>"##,
        irt = o.in_response_to,
        status = o.status,
        sig = o.sig_alg,
        dig = o.digest_alg,
        name = o.name_id,
        rcpt = o.recipient,
        aud = o.audience,
    );
    let der = idp.key.private_key_to_der().expect("der");
    Crypto::sign_xml(xml, &der).expect("signs")
}

fn request(idp: &Idp, xml: &str) -> VerifyRequest {
    VerifyRequest {
        saml_response: STANDARD.encode(xml),
        idp_entity_id: IDP.to_string(),
        idp_certificates: vec![idp.cert_b64.clone()],
        sp_entity_id: SP.to_string(),
        acs_url: ACS.to_string(),
        expected_request_id: REQ.to_string(),
        max_clock_skew_secs: None,
    }
}

#[test]
fn a_correct_response_verifies_and_is_normalised() {
    let idp = idp();
    let xml = signed(&idp, &Opts::default());
    let v = verify(&request(&idp, &xml)).expect("verifies");
    assert_eq!(v.assertion_id, "_a1");
    assert_eq!(v.issuer, IDP);
    assert_eq!(v.name_id, "alice@bigbank.com");
    assert_eq!(v.session_index.as_deref(), Some("_s1"));
    assert_eq!(v.attributes["groups"], vec!["admins", "staff"]);
    assert_eq!(v.attributes["email"], vec!["alice@bigbank.com"]);
}

#[test]
fn certificates_are_required_and_pinned() {
    let idp = idp();
    let other = self::idp();
    let xml = signed(&idp, &Opts::default());
    let mut r = request(&idp, &xml);
    r.idp_certificates.clear();
    assert_eq!(
        verify(&r),
        Err(Refusal::NoCertificate),
        "never verified without certificates"
    );
    let mut r = request(&idp, &xml);
    r.idp_certificates = vec![other.cert_b64.clone()];
    assert_eq!(verify(&r), Err(Refusal::Signature));
    let mut r = request(&idp, &xml);
    r.idp_certificates = vec!["!!".to_string()];
    assert_eq!(verify(&r), Err(Refusal::NoCertificate));
    // Rollover: the old and the new certificate both pinned.
    let mut r = request(&idp, &xml);
    r.idp_certificates = vec![other.cert_b64.clone(), idp.cert_b64.clone()];
    assert!(verify(&r).is_ok());
}

#[test]
fn tampering_after_signing_is_refused() {
    let idp = idp();
    let xml = signed(&idp, &Opts::default());
    let tampered = xml.replace("alice@bigbank.com", "mallory@bigbank.com");
    assert_eq!(verify(&request(&idp, &tampered)), Err(Refusal::Signature));
    // Comment truncation (2018 family): refused before any signature code runs.
    let commented = xml.replacen("alice@bigbank.com", "alice@bigbank.com<!---->.evil.com", 1);
    assert_eq!(verify(&request(&idp, &commented)), Err(Refusal::Comment));
}

#[test]
fn a_second_assertion_is_refused() {
    // Signature wrapping shape: an unsigned assertion next to the signed one.
    let idp = idp();
    let xml = signed(&idp, &Opts::default());
    let evil = xml.replacen(
        "<saml:Assertion ",
        r#"<saml:Assertion ID="_evil" Version="2.0" IssueInstant="2026-01-01T00:00:00Z"><saml:Issuer>x</saml:Issuer></saml:Assertion><saml:Assertion "#,
        1,
    );
    assert_eq!(verify(&request(&idp, &evil)), Err(Refusal::AssertionCount));
}

#[test]
fn each_binding_is_enforced() {
    let idp = idp();
    let check = |o: Opts<'_>| verify(&request(&idp, &signed(&idp, &o)));
    assert_eq!(
        check(Opts {
            audience: "https://other-sp",
            ..Opts::default()
        }),
        Err(Refusal::Audience)
    );
    assert_eq!(
        check(Opts {
            recipient: "https://other/acs",
            ..Opts::default()
        }),
        Err(Refusal::Destination)
    );
    assert_eq!(
        check(Opts {
            in_response_to: "_other",
            ..Opts::default()
        }),
        Err(Refusal::InResponseTo)
    );
    assert_eq!(
        check(Opts {
            status: "urn:oasis:names:tc:SAML:2.0:status:Requester",
            ..Opts::default()
        }),
        Err(Refusal::Status)
    );
    assert_eq!(
        check(Opts {
            authn_statement: false,
            ..Opts::default()
        }),
        Err(Refusal::NoAuthnStatement)
    );
    assert_eq!(
        check(Opts {
            not_on_or_after: Utc::now() - Duration::minutes(10),
            ..Opts::default()
        }),
        Err(Refusal::Time)
    );
    // samael itself refuses some empty-NameID shapes; either way it never verifies.
    assert!(matches!(
        check(Opts {
            name_id: "",
            ..Opts::default()
        }),
        Err(Refusal::NameId | Refusal::Invalid)
    ));
}

#[test]
fn weak_algorithms_are_refused() {
    let idp = idp();
    let sha1 = Opts {
        sig_alg: "http://www.w3.org/2000/09/xmldsig#rsa-sha1",
        digest_alg: "http://www.w3.org/2000/09/xmldsig#sha1",
        ..Opts::default()
    };
    assert_eq!(
        verify(&request(&idp, &signed(&idp, &sha1))),
        Err(Refusal::Signature)
    );
}

#[test]
fn hostile_xml_is_refused_before_parsing() {
    let idp = idp();
    let xml = signed(&idp, &Opts::default());
    let xxe = xml.replacen(
        "<samlp:Response",
        "<!DOCTYPE r [<!ENTITY x SYSTEM \"file:///etc/passwd\">]><samlp:Response",
        1,
    );
    assert!(xxe.contains("<!DOCTYPE"));
    assert_eq!(verify(&request(&idp, &xxe)), Err(Refusal::Doctype));
    let mut r = request(&idp, &xml);
    r.saml_response = "%%%not base64".to_string();
    assert_eq!(verify(&r), Err(Refusal::BadBase64));
    let mut r = request(&idp, &xml);
    r.saml_response = STANDARD.encode(vec![b'a'; 300 * 1024]);
    assert_eq!(verify(&r), Err(Refusal::TooLarge));
    let mut r = request(&idp, &xml);
    r.expected_request_id.clear();
    assert_eq!(verify(&r), Err(Refusal::BadRequest));
}
