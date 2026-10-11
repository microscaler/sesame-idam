//! DNS wire format for the challenge lookup (RFC 1035, EDNS(0) RFC 6891, Extended DNS Errors RFC 8914).
//!
//! Only what `dns-probe` needs: encode a TXT query with the DO bit, and decode the answer into the TXT strings at
//! the end of the CNAME chain, the response code, the AD bit, TTLs and any Extended DNS Error. Decoding is
//! defensive: bounded name compression, label and name lengths checked, every length checked against the buffer.

use std::collections::HashSet;

/// TXT record type.
pub const TYPE_TXT: u16 = 16;
const TYPE_CNAME: u16 = 5;
const TYPE_SOA: u16 = 6;
const TYPE_OPT: u16 = 41;
const TYPE_RRSIG: u16 = 46;
const CLASS_IN: u16 = 1;
const EDNS_UDP_SIZE: u16 = 4096;
const OPTION_EDE: u16 = 15;
/// Longest CNAME chain followed.
pub const MAX_CNAME_DEPTH: usize = 8;
const MAX_POINTERS: usize = 64;

/// Response codes.
pub mod rcode {
    /// No error.
    pub const NOERROR: u8 = 0;
    /// Server failure (also how validating resolvers report DNSSEC-bogus data).
    pub const SERVFAIL: u8 = 2;
    /// Name does not exist.
    pub const NXDOMAIN: u8 = 3;
}

/// Extended DNS Error info-codes that mean DNSSEC validation failed (RFC 8914 §4).
const EDE_DNSSEC_FAILURES: &[u16] = &[1, 2, 5, 6, 7, 8, 9, 10, 11, 12];

/// Why a message could not be decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WireError {
    /// Truncated or inconsistent message.
    #[error("malformed DNS message: {0}")]
    Malformed(&'static str),
    /// A name that cannot be encoded.
    #[error("name not encodable")]
    BadName,
}

/// Encode a query for `name` and `qtype` with EDNS(0) and the DO bit. The ID is 0, as RFC 8484 recommends for
/// cache-friendly `DoH` GET.
///
/// # Errors
///
/// [`WireError::BadName`] for an empty label, a label over 63 or a name over 255 octets.
pub fn encode_query(name: &str, qtype: u16) -> Result<Vec<u8>, WireError> {
    let mut out = Vec::with_capacity(64 + name.len());
    out.extend_from_slice(&0u16.to_be_bytes()); // ID
    out.extend_from_slice(&0x0100u16.to_be_bytes()); // RD
    out.extend_from_slice(&1u16.to_be_bytes()); // QDCOUNT
    out.extend_from_slice(&0u16.to_be_bytes()); // ANCOUNT
    out.extend_from_slice(&0u16.to_be_bytes()); // NSCOUNT
    out.extend_from_slice(&1u16.to_be_bytes()); // ARCOUNT (OPT)
    let mut total = 1usize;
    for label in name.trim_end_matches('.').split('.') {
        let len = u8::try_from(label.len()).map_err(|_| WireError::BadName)?;
        if label.is_empty() || len > 63 {
            return Err(WireError::BadName);
        }
        total += label.len() + 1;
        out.push(len);
        out.extend_from_slice(label.as_bytes());
    }
    if total > 255 {
        return Err(WireError::BadName);
    }
    out.push(0);
    out.extend_from_slice(&qtype.to_be_bytes());
    out.extend_from_slice(&CLASS_IN.to_be_bytes());
    // OPT pseudo-record: root name, type OPT, class = UDP size, TTL = ext-rcode 0, version 0, flags DO.
    out.push(0);
    out.extend_from_slice(&TYPE_OPT.to_be_bytes());
    out.extend_from_slice(&EDNS_UDP_SIZE.to_be_bytes());
    out.extend_from_slice(&[0, 0, 0x80, 0x00]);
    out.extend_from_slice(&0u16.to_be_bytes());
    Ok(out)
}

/// What a resolver answered, reduced to what verification needs.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DecodedAnswer {
    /// Response code (low four bits).
    pub rcode: u8,
    /// The resolver validated the data with DNSSEC (AD bit).
    pub ad: bool,
    /// The answer carried RRSIG records (the zone is signed).
    pub signed: bool,
    /// TXT records at the end of the CNAME chain, each record's character-strings concatenated.
    pub records: Vec<String>,
    /// CNAME targets followed from the query name, in order.
    pub cname_chain: Vec<String>,
    /// Smallest TTL among the TXT records used.
    pub ttl: Option<u32>,
    /// Negative-caching TTL from the SOA in the authority section (RFC 2308), for NXDOMAIN or no data.
    pub negative_ttl: Option<u32>,
    /// Extended DNS Error info-code, if the resolver sent one.
    pub ede: Option<u16>,
}

impl DecodedAnswer {
    /// The resolver reported a DNSSEC validation failure (SERVFAIL with a DNSSEC Extended DNS Error).
    #[must_use]
    pub fn dnssec_failed(&self) -> bool {
        self.rcode == rcode::SERVFAIL && self.ede.is_some_and(|c| EDE_DNSSEC_FAILURES.contains(&c))
    }
}

struct Reader<'a> {
    msg: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn u8(&mut self) -> Result<u8, WireError> {
        let b = *self
            .msg
            .get(self.pos)
            .ok_or(WireError::Malformed("truncated"))?;
        self.pos += 1;
        Ok(b)
    }
    fn u16(&mut self) -> Result<u16, WireError> {
        Ok(u16::from_be_bytes([self.u8()?, self.u8()?]))
    }
    fn u32(&mut self) -> Result<u32, WireError> {
        Ok(u32::from_be_bytes([
            self.u8()?,
            self.u8()?,
            self.u8()?,
            self.u8()?,
        ]))
    }
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], WireError> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or(WireError::Malformed("length"))?;
        let s = self
            .msg
            .get(self.pos..end)
            .ok_or(WireError::Malformed("truncated"))?;
        self.pos = end;
        Ok(s)
    }
    /// A possibly compressed name, lower-cased, without the trailing dot.
    fn name(&mut self) -> Result<String, WireError> {
        let mut labels: Vec<String> = Vec::new();
        let mut pos = self.pos;
        let mut jumped = false;
        let mut pointers = 0usize;
        let mut total = 1usize;
        loop {
            let len = *self
                .msg
                .get(pos)
                .ok_or(WireError::Malformed("truncated name"))?;
            match len & 0xc0 {
                0x00 => {
                    pos += 1;
                    if len == 0 {
                        break;
                    }
                    let label = self
                        .msg
                        .get(pos..pos + usize::from(len))
                        .ok_or(WireError::Malformed("truncated label"))?;
                    total += usize::from(len) + 1;
                    if total > 255 {
                        return Err(WireError::Malformed("name too long"));
                    }
                    labels.push(String::from_utf8_lossy(label).to_ascii_lowercase());
                    pos += usize::from(len);
                }
                0xc0 => {
                    let low = *self
                        .msg
                        .get(pos + 1)
                        .ok_or(WireError::Malformed("truncated pointer"))?;
                    pointers += 1;
                    if pointers > MAX_POINTERS {
                        return Err(WireError::Malformed("compression loop"));
                    }
                    if !jumped {
                        self.pos = pos + 2;
                        jumped = true;
                    }
                    pos = (usize::from(len & 0x3f) << 8) | usize::from(low);
                }
                _ => return Err(WireError::Malformed("label type")),
            }
        }
        if !jumped {
            self.pos = pos;
        }
        Ok(labels.join("."))
    }
}

struct Record<'a> {
    name: String,
    rtype: u16,
    ttl: u32,
    rdata_start: usize,
    rdata: &'a [u8],
}

fn read_record<'a>(r: &mut Reader<'a>) -> Result<Record<'a>, WireError> {
    let name = r.name()?;
    let rtype = r.u16()?;
    let _class = r.u16()?;
    let ttl = r.u32()?;
    let len = usize::from(r.u16()?);
    let rdata_start = r.pos;
    let rdata = r.bytes(len)?;
    Ok(Record {
        name,
        rtype,
        ttl,
        rdata_start,
        rdata,
    })
}

fn txt_value(rdata: &[u8]) -> Result<String, WireError> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < rdata.len() {
        let n = usize::from(rdata[i]);
        let s = rdata
            .get(i + 1..i + 1 + n)
            .ok_or(WireError::Malformed("TXT string"))?;
        out.extend_from_slice(s);
        i += 1 + n;
    }
    Ok(String::from_utf8_lossy(&out).into_owned())
}

fn ede_code(rdata: &[u8]) -> Option<u16> {
    let mut i = 0;
    while i + 4 <= rdata.len() {
        let code = u16::from_be_bytes([rdata[i], rdata[i + 1]]);
        let len = usize::from(u16::from_be_bytes([rdata[i + 2], rdata[i + 3]]));
        let data = rdata.get(i + 4..i + 4 + len)?;
        if code == OPTION_EDE && data.len() >= 2 {
            return Some(u16::from_be_bytes([data[0], data[1]]));
        }
        i += 4 + len;
    }
    None
}

/// Decode a response to a TXT query for `qname` (already lower-case, no trailing dot).
///
/// # Errors
///
/// [`WireError::Malformed`] for anything inconsistent, a response that is not one, a CNAME chain longer than
/// [`MAX_CNAME_DEPTH`] or a CNAME loop.
pub fn decode_txt_response(msg: &[u8], qname: &str) -> Result<DecodedAnswer, WireError> {
    let mut r = Reader { msg, pos: 0 };
    let _id = r.u16()?;
    let flags = r.u16()?;
    if flags & 0x8000 == 0 {
        return Err(WireError::Malformed("not a response"));
    }
    if flags & 0x0200 != 0 {
        return Err(WireError::Malformed("truncated (TC)"));
    }
    let qd = r.u16()?;
    let an = r.u16()?;
    let ns = r.u16()?;
    let ar = r.u16()?;
    for _ in 0..qd {
        let _ = r.name()?;
        let _ = r.u16()?;
        let _ = r.u16()?;
    }
    let mut answer = DecodedAnswer {
        rcode: u8::try_from(flags & 0x000f).unwrap_or(0),
        ad: flags & 0x0020 != 0,
        ..DecodedAnswer::default()
    };

    let mut answers = Vec::new();
    for _ in 0..an {
        answers.push(read_record(&mut r)?);
    }
    // Follow CNAMEs from the query name.
    let mut target = qname.to_ascii_lowercase();
    let mut seen = HashSet::from([target.clone()]);
    loop {
        let next = answers
            .iter()
            .find(|rec| rec.rtype == TYPE_CNAME && rec.name == target);
        let Some(cname) = next else { break };
        let mut sub = Reader {
            msg,
            pos: cname.rdata_start,
        };
        let to = sub.name()?;
        if !seen.insert(to.clone()) {
            return Err(WireError::Malformed("CNAME loop"));
        }
        answer.cname_chain.push(to.clone());
        if answer.cname_chain.len() > MAX_CNAME_DEPTH {
            return Err(WireError::Malformed("CNAME chain too long"));
        }
        target = to;
    }
    for rec in &answers {
        if rec.rtype == TYPE_RRSIG {
            answer.signed = true;
        }
        if rec.rtype == TYPE_TXT && rec.name == target {
            answer.records.push(txt_value(rec.rdata)?);
            answer.ttl = Some(answer.ttl.map_or(rec.ttl, |t| t.min(rec.ttl)));
        }
    }
    for _ in 0..ns {
        let rec = read_record(&mut r)?;
        if rec.rtype == TYPE_RRSIG {
            answer.signed = true;
        }
        if rec.rtype == TYPE_SOA && rec.rdata.len() >= 4 {
            let tail = &rec.rdata[rec.rdata.len() - 4..];
            let minimum = u32::from_be_bytes([tail[0], tail[1], tail[2], tail[3]]);
            answer.negative_ttl = Some(rec.ttl.min(minimum));
        }
    }
    for _ in 0..ar {
        let rec = read_record(&mut r)?;
        if rec.rtype == TYPE_OPT {
            answer.ede = ede_code(rec.rdata);
        }
    }
    Ok(answer)
}

/// Building blocks for test responses (also used by `dns-probe`'s tests).
#[cfg(any(test, feature = "test-util"))]
pub mod build {
    /// A record for [`response`].
    pub struct Rr<'a> {
        /// Owner name.
        pub name: &'a str,
        /// Type.
        pub rtype: u16,
        /// TTL.
        pub ttl: u32,
        /// RDATA.
        pub rdata: Vec<u8>,
    }

    /// Encode an uncompressed name.
    #[must_use]
    pub fn name(n: &str) -> Vec<u8> {
        let mut out = Vec::new();
        for l in n.split('.').filter(|l| !l.is_empty()) {
            out.push(u8::try_from(l.len()).unwrap_or(0));
            out.extend_from_slice(l.as_bytes());
        }
        out.push(0);
        out
    }

    /// TXT RDATA from character-strings.
    #[must_use]
    pub fn txt(strings: &[&str]) -> Vec<u8> {
        let mut out = Vec::new();
        for s in strings {
            out.push(u8::try_from(s.len()).unwrap_or(0));
            out.extend_from_slice(s.as_bytes());
        }
        out
    }

    /// A response: flags (QR is added), question for `qname` TXT, answer, authority and an OPT record carrying
    /// an optional Extended DNS Error.
    #[must_use]
    pub fn response(
        flags: u16,
        qname: &str,
        answers: &[Rr<'_>],
        authority: &[Rr<'_>],
        ede: Option<u16>,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0u16.to_be_bytes());
        out.extend_from_slice(&(flags | 0x8000).to_be_bytes());
        out.extend_from_slice(&1u16.to_be_bytes());
        out.extend_from_slice(&u16::try_from(answers.len()).unwrap_or(0).to_be_bytes());
        out.extend_from_slice(&u16::try_from(authority.len()).unwrap_or(0).to_be_bytes());
        out.extend_from_slice(&1u16.to_be_bytes());
        out.extend_from_slice(&name(qname));
        out.extend_from_slice(&super::TYPE_TXT.to_be_bytes());
        out.extend_from_slice(&1u16.to_be_bytes());
        for rr in answers.iter().chain(authority) {
            out.extend_from_slice(&name(rr.name));
            out.extend_from_slice(&rr.rtype.to_be_bytes());
            out.extend_from_slice(&1u16.to_be_bytes());
            out.extend_from_slice(&rr.ttl.to_be_bytes());
            out.extend_from_slice(&u16::try_from(rr.rdata.len()).unwrap_or(0).to_be_bytes());
            out.extend_from_slice(&rr.rdata);
        }
        out.push(0);
        out.extend_from_slice(&super::TYPE_OPT.to_be_bytes());
        out.extend_from_slice(&4096u16.to_be_bytes());
        out.extend_from_slice(&[0, 0, 0x80, 0]);
        let opt = ede.map(|c| {
            let mut o = Vec::new();
            o.extend_from_slice(&15u16.to_be_bytes());
            o.extend_from_slice(&2u16.to_be_bytes());
            o.extend_from_slice(&c.to_be_bytes());
            o
        });
        let opt = opt.unwrap_or_default();
        out.extend_from_slice(&u16::try_from(opt.len()).unwrap_or(0).to_be_bytes());
        out.extend_from_slice(&opt);
        out
    }

    /// SOA RDATA with the given minimum.
    #[must_use]
    pub fn soa(minimum: u32) -> Vec<u8> {
        let mut out = name("ns.example");
        out.extend_from_slice(&name("hostmaster.example"));
        for v in [1u32, 3600, 600, 86400, minimum] {
            out.extend_from_slice(&v.to_be_bytes());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::build::{response, soa, txt, Rr};
    use super::*;

    const Q: &str = "_sesame-challenge.bigbank.com";

    #[test]
    fn query_encoding() {
        let q = encode_query(Q, TYPE_TXT).expect("encodes");
        assert_eq!(&q[..12], &[0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1]);
        assert!(q.ends_with(&[0, 0, 41, 0x10, 0, 0, 0, 0x80, 0, 0, 0]));
        assert!(encode_query("a..b", TYPE_TXT).is_err());
        assert!(encode_query(&format!("{}.com", "a".repeat(64)), TYPE_TXT).is_err());
    }

    #[test]
    fn txt_strings_are_joined_per_record_and_unrelated_records_kept_apart() {
        let msg = response(
            0x0180 | 0x0020,
            Q,
            &[
                Rr {
                    name: Q,
                    rtype: TYPE_TXT,
                    ttl: 300,
                    rdata: txt(&["sesame-domain-verify=v1;", "t=abc"]),
                },
                Rr {
                    name: Q,
                    rtype: TYPE_TXT,
                    ttl: 120,
                    rdata: txt(&["v=spf1 -all"]),
                },
                Rr {
                    name: Q,
                    rtype: TYPE_RRSIG,
                    ttl: 120,
                    rdata: vec![0; 20],
                },
            ],
            &[],
            None,
        );
        let a = decode_txt_response(&msg, Q).expect("decodes");
        assert_eq!(
            a.records,
            vec!["sesame-domain-verify=v1;t=abc", "v=spf1 -all"]
        );
        assert_eq!(a.ttl, Some(120));
        assert!(a.ad && a.signed);
        assert_eq!(a.rcode, rcode::NOERROR);
    }

    #[test]
    fn cname_is_followed_and_loops_fail() {
        let target = "_sesame.automation.example.net";
        let mut cname = super::build::name(target);
        let msg = response(
            0x0180,
            Q,
            &[
                Rr {
                    name: Q,
                    rtype: 5,
                    ttl: 60,
                    rdata: cname.clone(),
                },
                Rr {
                    name: target,
                    rtype: TYPE_TXT,
                    ttl: 60,
                    rdata: txt(&["sesame-domain-verify=v1;t=x"]),
                },
                Rr {
                    name: Q,
                    rtype: TYPE_TXT,
                    ttl: 60,
                    rdata: txt(&["ignored, not at the chain end"]),
                },
            ],
            &[],
            None,
        );
        let a = decode_txt_response(&msg, Q).expect("decodes");
        assert_eq!(a.cname_chain, vec![target]);
        assert_eq!(a.records, vec!["sesame-domain-verify=v1;t=x"]);

        cname = super::build::name(Q);
        let looped = response(
            0x0180,
            Q,
            &[
                Rr {
                    name: Q,
                    rtype: 5,
                    ttl: 60,
                    rdata: super::build::name(target),
                },
                Rr {
                    name: target,
                    rtype: 5,
                    ttl: 60,
                    rdata: cname,
                },
            ],
            &[],
            None,
        );
        assert_eq!(
            decode_txt_response(&looped, Q),
            Err(WireError::Malformed("CNAME loop"))
        );
    }

    #[test]
    fn negative_answers_carry_the_soa_ttl_and_ede() {
        let msg = response(
            0x0180 | u16::from(rcode::NXDOMAIN),
            Q,
            &[],
            &[Rr {
                name: "bigbank.com",
                rtype: 6,
                ttl: 900,
                rdata: soa(300),
            }],
            None,
        );
        let a = decode_txt_response(&msg, Q).expect("decodes");
        assert_eq!(a.rcode, rcode::NXDOMAIN);
        assert_eq!(a.negative_ttl, Some(300));

        let bogus = response(0x0180 | u16::from(rcode::SERVFAIL), Q, &[], &[], Some(6));
        let a = decode_txt_response(&bogus, Q).expect("decodes");
        assert!(a.dnssec_failed());
        let outage = response(0x0180 | u16::from(rcode::SERVFAIL), Q, &[], &[], Some(22));
        assert!(!decode_txt_response(&outage, Q)
            .expect("decodes")
            .dnssec_failed());
    }

    #[test]
    fn hostile_messages_are_refused_not_panicking() {
        assert!(decode_txt_response(&[], Q).is_err());
        // Query, not a response.
        let q = encode_query(Q, TYPE_TXT).expect("q");
        assert!(decode_txt_response(&q, Q).is_err());
        // Compression pointer to itself.
        let mut msg = vec![0, 0, 0x81, 0x80, 0, 1, 0, 0, 0, 0, 0, 0];
        msg.extend_from_slice(&[0xc0, 12, 0, 16, 0, 1]);
        assert!(decode_txt_response(&msg, Q).is_err());
        // Every truncation of a valid message fails cleanly.
        let ok = response(
            0x0180,
            Q,
            &[Rr {
                name: Q,
                rtype: TYPE_TXT,
                ttl: 1,
                rdata: txt(&["x"]),
            }],
            &[],
            None,
        );
        for n in 0..ok.len() {
            let _ = decode_txt_response(&ok[..n], Q);
        }
    }
}
