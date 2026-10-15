//! An S3-compatible backend (BE-1), built from primitives: an HTTP client and a hand-written
//! AWS Signature Version 4 signer. No SDK, so nothing here tracks a vendor's release train
//! (IND-1). Path-style addressing, which works with AWS S3 and MinIO alike.

use std::io::Read;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use crate::backend::{check_key, Backend};
use crate::util;

#[derive(Clone)]
pub struct S3Config {
    /// `https://host[:port]`, no trailing slash, no bucket.
    pub endpoint: String,
    pub bucket: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
}

pub struct S3Backend {
    cfg: S3Config,
    host: String,
    agent: ureq::Agent,
}

struct Response {
    status: u16,
    body: Vec<u8>,
}

impl S3Backend {
    pub fn new(mut cfg: S3Config) -> Result<Self> {
        cfg.endpoint = cfg.endpoint.trim_end_matches('/').to_string();
        let rest = cfg
            .endpoint
            .strip_prefix("https://")
            .or_else(|| cfg.endpoint.strip_prefix("http://"))
            .ok_or_else(|| anyhow!("S3 endpoint must start with https:// or http://"))?;
        if rest.is_empty() || rest.contains('/') {
            bail!("S3 endpoint must be scheme://host[:port], without a path");
        }
        let https = cfg.endpoint.starts_with("https://");
        let host = match rest.rsplit_once(':') {
            Some((h, "443")) if https => h.to_string(),
            Some((h, "80")) if !https => h.to_string(),
            _ => rest.to_string(),
        };
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout(Duration::from_secs(300))
            .build();
        Ok(Self { cfg, host, agent })
    }

    /// Creates the bucket. Succeeds if this user already owns it.
    pub fn create_bucket(&self) -> Result<()> {
        let r = self.request("PUT", None, &[], &[], &[])?;
        match r.status {
            200 => Ok(()),
            409 if String::from_utf8_lossy(&r.body).contains("BucketAlreadyOwnedByYou") => Ok(()),
            s => bail!("create bucket: HTTP {s}: {}", error_text(&r.body)),
        }
    }

    fn request(
        &self,
        method: &str,
        key: Option<&str>,
        query: &[(&str, &str)],
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<Response> {
        let mut uri = format!("/{}", uri_encode(&self.cfg.bucket, true));
        if let Some(k) = key {
            uri.push('/');
            uri.push_str(&uri_encode(k, false));
        }
        let mut q: Vec<(String, String)> = query
            .iter()
            .map(|(k, v)| (uri_encode(k, true), uri_encode(v, true)))
            .collect();
        q.sort();
        let query_string = q
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("&");

        let now = util::now_unix();
        let amz_date = util::amz_date(now);
        let date = &amz_date[..8];
        let payload_hash = hex::encode(Sha256::digest(body));
        let signed_headers = "host;x-amz-content-sha256;x-amz-date";
        let canonical_request = format!(
            "{method}\n{uri}\n{query_string}\nhost:{}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\n\n{signed_headers}\n{payload_hash}",
            self.host
        );
        let scope = format!("{date}/{}/s3/aws4_request", self.cfg.region);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
            hex::encode(Sha256::digest(canonical_request.as_bytes()))
        );
        let k_date = hmac(
            format!("AWS4{}", self.cfg.secret_access_key).as_bytes(),
            date.as_bytes(),
        );
        let k_region = hmac(&k_date, self.cfg.region.as_bytes());
        let k_service = hmac(&k_region, b"s3");
        let k_signing = hmac(&k_service, b"aws4_request");
        let signature = hex::encode(hmac(&k_signing, string_to_sign.as_bytes()));
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
            self.cfg.access_key_id
        );

        let mut url = format!("{}{uri}", self.cfg.endpoint);
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }
        let mut req = self
            .agent
            .request(method, &url)
            .set("x-amz-date", &amz_date)
            .set("x-amz-content-sha256", &payload_hash)
            .set("authorization", &authorization);
        for (k, v) in headers {
            req = req.set(k, v);
        }
        let result = if body.is_empty() && method != "PUT" {
            req.call()
        } else {
            req.send_bytes(body)
        };
        let resp = match result {
            Ok(r) => r,
            Err(ureq::Error::Status(_, r)) => r,
            Err(e) => return Err(anyhow!("{method} {url}: {e}")),
        };
        let status = resp.status();
        let mut body = Vec::new();
        resp.into_reader()
            .read_to_end(&mut body)
            .with_context(|| format!("{method} {url}: reading response"))?;
        Ok(Response { status, body })
    }
}

impl Backend for S3Backend {
    fn put(&self, key: &str, data: &[u8]) -> Result<()> {
        check_key(key)?;
        let r = self.request("PUT", Some(key), &[], &[], data)?;
        if r.status != 200 {
            bail!("PUT {key}: HTTP {}: {}", r.status, error_text(&r.body));
        }
        Ok(())
    }

    fn put_if_absent(&self, key: &str, data: &[u8]) -> Result<bool> {
        check_key(key)?;
        let r = self.request("PUT", Some(key), &[], &[("if-none-match", "*")], data)?;
        match r.status {
            200 => Ok(true),
            // 412: the object exists. 409: a concurrent conditional write to the same key is in
            // flight; the caller treats both as "someone else got there first".
            412 | 409 => Ok(false),
            s => bail!("conditional PUT {key}: HTTP {s}: {}", error_text(&r.body)),
        }
    }

    fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        check_key(key)?;
        let r = self.request("GET", Some(key), &[], &[], &[])?;
        match r.status {
            200 => Ok(Some(r.body)),
            404 => Ok(None),
            s => bail!("GET {key}: HTTP {s}: {}", error_text(&r.body)),
        }
    }

    fn delete(&self, key: &str) -> Result<()> {
        check_key(key)?;
        let r = self.request("DELETE", Some(key), &[], &[], &[])?;
        match r.status {
            200 | 204 | 404 => Ok(()),
            s => bail!("DELETE {key}: HTTP {s}: {}", error_text(&r.body)),
        }
    }

    fn list(&self, prefix: &str, start_after: Option<&str>) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        let mut token: Option<String> = None;
        loop {
            let mut q = vec![("list-type", "2"), ("prefix", prefix)];
            if let Some(t) = token.as_deref() {
                q.push(("continuation-token", t));
            } else if let Some(s) = start_after {
                q.push(("start-after", s));
            }
            let r = self.request("GET", None, &q, &[], &[])?;
            if r.status != 200 {
                bail!("LIST {prefix}: HTTP {}: {}", r.status, error_text(&r.body));
            }
            let xml = String::from_utf8_lossy(&r.body);
            for contents in tag_values(&xml, "Contents") {
                if let Some(k) = tag_values(contents, "Key").first() {
                    keys.push(xml_unescape(k));
                }
            }
            let truncated = tag_values(&xml, "IsTruncated").first() == Some(&"true");
            token = tag_values(&xml, "NextContinuationToken")
                .first()
                .map(|t| xml_unescape(t));
            if !truncated || token.is_none() {
                break;
            }
        }
        keys.sort();
        Ok(keys)
    }

    fn describe(&self) -> String {
        format!("s3 {}/{}", self.cfg.endpoint, self.cfg.bucket)
    }
}

fn hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut m = <Hmac<Sha256> as Mac>::new_from_slice(key).expect("HMAC accepts any key length");
    m.update(data);
    m.finalize().into_bytes().to_vec()
}

/// AWS URI encoding: everything except unreserved characters is percent-encoded; `/` is kept
/// in object key paths.
fn uri_encode(s: &str, encode_slash: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b'/' if !encode_slash => out.push('/'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The text inside each `<tag>…</tag>`. Enough XML for S3 listing and error responses.
fn tag_values<'a>(xml: &'a str, tag: &str) -> Vec<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(i) = rest.find(&open) {
        let after = &rest[i + open.len()..];
        match after.find(&close) {
            Some(j) => {
                out.push(&after[..j]);
                rest = &after[j + close.len()..];
            }
            None => break,
        }
    }
    out
}

fn xml_unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn error_text(body: &[u8]) -> String {
    let xml = String::from_utf8_lossy(body);
    let code = tag_values(&xml, "Code").first().copied().unwrap_or("");
    let msg = tag_values(&xml, "Message").first().copied().unwrap_or("");
    if code.is_empty() && msg.is_empty() {
        xml.chars().take(200).collect()
    } else {
        format!("{code}: {msg}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoding() {
        assert_eq!(uri_encode("a b/c~", false), "a%20b/c~");
        assert_eq!(uri_encode("a/b", true), "a%2Fb");
    }

    #[test]
    fn signing_key_matches_aws_example() {
        // From the AWS SigV4 documentation's signing-key example.
        let k_date = hmac(b"AWS4wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY", b"20120215");
        let k_region = hmac(&k_date, b"us-east-1");
        let k_service = hmac(&k_region, b"iam");
        let k_signing = hmac(&k_service, b"aws4_request");
        assert_eq!(
            hex::encode(k_signing),
            "f4780e2d9f65fa895f9c67b32ce1baf0b0d8a43505a000a1a9e090d414db404d"
        );
    }

    #[test]
    fn list_parsing() {
        let xml = "<ListBucketResult><IsTruncated>false</IsTruncated><Contents><Key>a&amp;b</Key><Size>1</Size></Contents><Contents><Key>c</Key></Contents></ListBucketResult>";
        let keys: Vec<String> = tag_values(xml, "Contents")
            .iter()
            .map(|c| xml_unescape(tag_values(c, "Key")[0]))
            .collect();
        assert_eq!(keys, vec!["a&b", "c"]);
    }
}
