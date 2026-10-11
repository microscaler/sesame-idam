//! Checks on the raw response before any signature code sees it (ADR-014 §3, DESIGN-saml-sp.md §5).
//!
//! A streaming pass over the document with quick-xml (namespace-aware). It refuses whatever SAML from the three
//! supported IdPs never needs and attacks commonly use, and records the few unsigned facts the post-checks compare
//! against (the single assertion's ID, the response status, destination and `InResponseTo`).

use quick_xml::events::{BytesStart, Event};
use quick_xml::name::{Namespace, ResolveResult};
use quick_xml::NsReader;

use crate::Refusal;

const NS_PROTOCOL: &[u8] = b"urn:oasis:names:tc:SAML:2.0:protocol";
const NS_ASSERTION: &[u8] = b"urn:oasis:names:tc:SAML:2.0:assertion";
const NS_DSIG: &[u8] = b"http://www.w3.org/2000/09/xmldsig#";

/// Canonicalisation and transform algorithms allowed in `SignedInfo` (no comments variants, no XPath).
const ALLOWED_TRANSFORMS: &[&str] = &[
    "http://www.w3.org/2000/09/xmldsig#enveloped-signature",
    "http://www.w3.org/2001/10/xml-exc-c14n#",
    "http://www.w3.org/TR/2001/REC-xml-c14n-20010315",
];
/// At most a response signature and an assertion signature.
const MAX_SIGNATURES: usize = 2;
/// Deepest element nesting accepted (SAML responses are shallow; deep nesting is a parser attack).
const MAX_DEPTH: usize = 32;

/// Unsigned facts about the document, compared with the verified result afterwards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    /// `ID` of the only `saml:Assertion`.
    pub assertion_id: String,
    /// `Value` of `samlp:Response/samlp:Status/samlp:StatusCode` (top level).
    pub status: Option<String>,
    /// `Destination` on `samlp:Response`.
    pub destination: Option<String>,
    /// `InResponseTo` on `samlp:Response`.
    pub in_response_to: Option<String>,
}

fn in_ns(ns: &ResolveResult<'_>, uri: &[u8]) -> bool {
    matches!(ns, ResolveResult::Bound(Namespace(n)) if *n == uri)
}

fn attr(e: &BytesStart<'_>, name: &[u8]) -> Result<Option<String>, Refusal> {
    for a in e.attributes() {
        let a = a.map_err(|_| Refusal::Malformed)?;
        if a.key.as_ref() == name {
            let v = a
                .normalized_value(quick_xml::XmlVersion::Explicit1_0)
                .map_err(|_| Refusal::Malformed)?;
            return Ok(Some(v.into_owned()));
        }
    }
    Ok(None)
}

/// Run the pre-checks over the decoded response XML (one pass, rules in document order).
///
/// # Errors
///
/// A [`Refusal`] naming the first rule broken.
#[allow(clippy::too_many_lines)] // one streaming pass; splitting it would scatter the rules
pub fn precheck(xml: &str) -> Result<Shape, Refusal> {
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().check_end_names = true;
    reader.config_mut().expand_empty_elements = true;

    let mut depth = 0usize;
    let mut stack: Vec<(bool, bool, bool)> = Vec::new(); // (is protocol Status, is assertion, is signature)
    let mut root_seen = false;
    let mut assertion_ids: Vec<String> = Vec::new();
    let mut signatures = 0usize;
    let mut shape = Shape {
        assertion_id: String::new(),
        status: None,
        destination: None,
        in_response_to: None,
    };

    loop {
        let (ns, event) = reader
            .read_resolved_event()
            .map_err(|_| Refusal::Malformed)?;
        match event {
            Event::Eof => break,
            Event::DocType(_) => return Err(Refusal::Doctype),
            Event::Comment(_) => return Err(Refusal::Comment),
            Event::PI(_) => return Err(Refusal::ProcessingInstruction),
            Event::CData(_) => return Err(Refusal::Cdata),
            Event::Decl(_) | Event::Text(_) | Event::GeneralRef(_) => {}
            Event::Start(e) => {
                depth += 1;
                if depth > MAX_DEPTH {
                    return Err(Refusal::Malformed);
                }
                let local = e.local_name();
                let local = local.as_ref();
                if !root_seen {
                    root_seen = true;
                    if !(in_ns(&ns, NS_PROTOCOL) && local == b"Response") {
                        return Err(Refusal::NotAResponse);
                    }
                    shape.destination = attr(&e, b"Destination")?;
                    shape.in_response_to = attr(&e, b"InResponseTo")?;
                } else if in_ns(&ns, NS_PROTOCOL) && local == b"Response" {
                    return Err(Refusal::NotAResponse);
                }
                if local == b"EncryptedAssertion" || local == b"EncryptedID" {
                    return Err(Refusal::Encrypted);
                }
                // Any element named Assertion counts, whatever its namespace: a namespace-unaware parser
                // downstream must never see a second one (parser differential).
                let is_assertion = local == b"Assertion";
                if is_assertion && !in_ns(&ns, NS_ASSERTION) {
                    return Err(Refusal::AssertionCount);
                }
                if is_assertion {
                    assertion_ids.push(attr(&e, b"ID")?.unwrap_or_default());
                }
                let is_signature = in_ns(&ns, NS_DSIG) && local == b"Signature";
                if is_signature {
                    signatures += 1;
                    if signatures > MAX_SIGNATURES {
                        return Err(Refusal::SignatureShape);
                    }
                }
                if in_ns(&ns, NS_DSIG) {
                    match local {
                        b"Object" | b"Manifest" | b"SignatureProperties" | b"RetrievalMethod" => {
                            return Err(Refusal::SignatureShape);
                        }
                        b"Transform" | b"CanonicalizationMethod" => {
                            let alg = attr(&e, b"Algorithm")?.unwrap_or_default();
                            if !ALLOWED_TRANSFORMS.contains(&alg.as_str()) {
                                return Err(Refusal::SignatureShape);
                            }
                        }
                        _ => {}
                    }
                }
                // Top-level status: Response (depth 1) > Status (depth 2) > StatusCode (depth 3).
                let parent_is_top_status = stack.last().is_some_and(|(status, _, _)| *status);
                if parent_is_top_status
                    && in_ns(&ns, NS_PROTOCOL)
                    && local == b"StatusCode"
                    && shape.status.is_none()
                {
                    shape.status = attr(&e, b"Value")?;
                }
                let is_top_status = depth == 2 && in_ns(&ns, NS_PROTOCOL) && local == b"Status";
                stack.push((is_top_status, is_assertion, is_signature));
            }
            Event::End(_) => {
                depth = depth.saturating_sub(1);
                stack.pop();
            }
            Event::Empty(_) => return Err(Refusal::Malformed), // expand_empty_elements makes this unreachable
        }
    }

    if !root_seen {
        return Err(Refusal::NotAResponse);
    }
    if assertion_ids.len() != 1 {
        return Err(Refusal::AssertionCount);
    }
    shape.assertion_id = assertion_ids.remove(0);
    if shape.assertion_id.is_empty() {
        return Err(Refusal::AssertionCount);
    }
    Ok(shape)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = r#"<?xml version="1.0"?>
<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" ID="r1" Destination="https://sp/acs" InResponseTo="req-1">
  <samlp:Status><samlp:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"/></samlp:Status>
  <saml:Assertion ID="a1"><saml:Issuer>idp</saml:Issuer></saml:Assertion>
</samlp:Response>"#;

    #[test]
    fn records_the_shape() {
        let shape = precheck(OK).expect("ok");
        assert_eq!(shape.assertion_id, "a1");
        assert_eq!(
            shape.status.as_deref(),
            Some("urn:oasis:names:tc:SAML:2.0:status:Success")
        );
        assert_eq!(shape.destination.as_deref(), Some("https://sp/acs"));
        assert_eq!(shape.in_response_to.as_deref(), Some("req-1"));
    }

    fn with(insert: &str) -> String {
        OK.replace(
            "<saml:Assertion ID=\"a1\">",
            &format!("{insert}<saml:Assertion ID=\"a1\">"),
        )
    }

    #[test]
    fn refuses_each_forbidden_construct() {
        let doctype = OK.replace(
            "<?xml version=\"1.0\"?>",
            "<?xml version=\"1.0\"?><!DOCTYPE r [<!ENTITY x \"y\">]>",
        );
        assert_eq!(precheck(&doctype), Err(Refusal::Doctype));
        assert_eq!(precheck(&with("<!-- c -->")), Err(Refusal::Comment));
        assert_eq!(
            precheck(&with("<?pi x?>")),
            Err(Refusal::ProcessingInstruction)
        );
        assert_eq!(precheck(&with("<x><![CDATA[y]]></x>")), Err(Refusal::Cdata));
        assert_eq!(
            precheck(&with("<saml:EncryptedAssertion/>")),
            Err(Refusal::Encrypted)
        );
        assert_eq!(
            precheck(&with("<saml:Assertion ID=\"a0\"/>")),
            Err(Refusal::AssertionCount)
        );
        let none = OK.replace(
            "<saml:Assertion ID=\"a1\"><saml:Issuer>idp</saml:Issuer></saml:Assertion>",
            "",
        );
        assert_eq!(precheck(&none), Err(Refusal::AssertionCount));
        assert_eq!(precheck("<foo/>"), Err(Refusal::NotAResponse));
        assert!(precheck("not xml <").is_err());
        assert_eq!(
            precheck(&with(
                r#"<ds:Signature xmlns:ds="http://www.w3.org/2000/09/xmldsig#"><ds:Object/></ds:Signature>"#
            )),
            Err(Refusal::SignatureShape)
        );
        assert_eq!(
            precheck(&with(
                r#"<ds:Transform xmlns:ds="http://www.w3.org/2000/09/xmldsig#" Algorithm="http://www.w3.org/TR/1999/REC-xpath-19991116"/>"#
            )),
            Err(Refusal::SignatureShape)
        );
        let deep = format!("{}{}", "<a>".repeat(40), "</a>".repeat(40));
        assert_eq!(precheck(&with(&deep)), Err(Refusal::Malformed));
    }

    #[test]
    fn an_assertion_in_another_namespace_is_refused() {
        // A namespace-unaware parser could take it for the assertion: never let one through.
        let decoy = with(r#"<x:Assertion xmlns:x="urn:example" ID="decoy"/>"#);
        assert_eq!(precheck(&decoy), Err(Refusal::AssertionCount));
    }
}
