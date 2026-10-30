#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

use super::*;

const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];

/// One request as the stub server saw it.
#[derive(Debug)]
struct Seen {
    request_line: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Seen {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// A server on a free local port that answers one request with `status` and `body`, and hands
/// back what it was sent.
fn stub(status: u16, body: &str) -> (String, JoinHandle<Seen>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let body = body.to_owned();
    let handle = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request_line = String::new();
        reader.read_line(&mut request_line).unwrap();
        let mut headers = Vec::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            let (k, v) = line.split_once(':').unwrap();
            headers.push((k.trim().to_owned(), v.trim().to_owned()));
        }
        let length: usize = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
            .map_or(0, |(_, v)| v.parse().unwrap());
        let mut bytes = vec![0; length];
        reader.read_exact(&mut bytes).unwrap();
        let mut stream = stream;
        write!(
            stream,
            "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        stream.flush().unwrap();
        Seen {
            request_line: request_line.trim_end().to_owned(),
            headers,
            body: bytes,
        }
    });
    (base, handle)
}

fn png(name: &str) -> ProofFile {
    check(name, PNG.to_vec()).unwrap()
}

fn uploaded_json(sha: &str) -> String {
    format!(
        r#"{{"key":"org/org-1/{sha}","sha256":"{sha}","size":12,"content_type":"image/png","thumbnail_key":null}}"#
    )
}

#[test]
fn photos_and_pdfs_are_known_by_their_first_bytes() {
    assert_eq!(sniff(PNG), Some("image/png"));
    assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
    assert_eq!(sniff(b"GIF89a.."), Some("image/gif"));
    assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
    assert_eq!(sniff(b"%PDF-1.7\n"), Some("application/pdf"));
    assert_eq!(sniff(b"\0\0\0\x18ftypheic\0\0"), Some("image/heic"));
    assert_eq!(sniff(b"\0\0\0\x18ftypmif1\0\0"), Some("image/heif"));
    assert_eq!(
        sniff(b"\0\0\0\x18ftypisom\0\0"),
        None,
        "an MP4 is not a photo"
    );
    assert_eq!(sniff(b"<html><body>"), None);
    assert_eq!(sniff(b""), None);
}

#[test]
fn an_empty_a_large_or_an_unknown_file_is_refused_before_sending() {
    assert_eq!(
        check("a.png", vec![]),
        Err(ProofsError::Empty("a.png".into()))
    );
    assert_eq!(
        check("page.png", b"<html>".to_vec()),
        Err(ProofsError::NotAllowed("page.png".into()))
    );
    let mut big = PNG.to_vec();
    big.resize(usize::try_from(MAX_BYTES).unwrap() + 1, 0);
    assert_eq!(
        check("big.png", big),
        Err(ProofsError::TooLarge("big.png".into()))
    );
    let mut limit = PNG.to_vec();
    limit.resize(usize::try_from(MAX_BYTES).unwrap(), 0);
    assert!(
        check("limit.png", limit).is_ok(),
        "exactly 25 MB is allowed"
    );
}

#[test]
fn a_file_on_disk_is_read_and_checked() {
    let dir = std::env::temp_dir().join(format!("ft-proofs-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("receipt.png");
    std::fs::write(&path, PNG).unwrap();
    let file = read_file(&path).unwrap();
    assert_eq!(
        (file.name.as_str(), file.content_type),
        ("receipt.png", "image/png")
    );
    let missing = read_file(&dir.join("missing.pdf")).unwrap_err();
    assert!(matches!(missing, ProofsError::Read { ref name, .. } if name == "missing.pdf"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_hash_is_lowercase_hex_sha256() {
    assert_eq!(
        sha256_hex(b"test"),
        "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
    );
}

#[test]
fn an_upload_sends_the_token_the_fields_and_the_file() {
    let file = png("matatu \"receipt\".png");
    let sha = sha256_hex(&file.bytes);
    let (base, server) = stub(201, &uploaded_json(&sha));
    let client = ProofsClient::new(&format!("{base}/"), "tok-123");
    let got = client
        .upload("org-1", LinkKind::Claim, "EC-0003", &file)
        .unwrap();
    let seen = server.join().unwrap();
    assert_eq!(seen.request_line, "POST /upload HTTP/1.1");
    assert_eq!(seen.header("authorization"), Some("Bearer tok-123"));
    let content_type = seen.header("content-type").unwrap();
    let boundary = content_type
        .strip_prefix("multipart/form-data; boundary=")
        .unwrap();
    let body = String::from_utf8_lossy(&seen.body);
    for (name, value) in [
        ("org", "org-1"),
        ("link_kind", "claim"),
        ("link_id", "EC-0003"),
    ] {
        assert!(
            body.contains(&format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )),
            "{name} missing in {body}"
        );
    }
    assert!(body.contains(
        "name=\"file\"; filename=\"matatu _receipt_.png\"\r\nContent-Type: image/png\r\n\r\n"
    ));
    assert!(seen.body.windows(PNG.len()).any(|w| w == PNG));
    assert!(body.ends_with(&format!("\r\n--{boundary}--\r\n")));
    assert_eq!(
        got,
        Uploaded {
            key: format!("org/org-1/{sha}"),
            sha256: sha,
            size: 12,
            content_type: "image/png".into(),
            thumbnail_key: None,
        }
    );
}

#[test]
fn the_workers_refusal_comes_back_in_its_own_words() {
    let (base, server) = stub(403, r#"{"error":"You are not a member of org-1."}"#);
    let err = ProofsClient::new(&base, "t")
        .upload("org-1", LinkKind::Bill, "bill-1", &png("a.png"))
        .unwrap_err();
    server.join().unwrap();
    assert_eq!(
        err,
        ProofsError::Refused {
            status: 403,
            message: "You are not a member of org-1.".into()
        }
    );
}

#[test]
fn a_refusal_without_a_message_names_the_status() {
    let (base, server) = stub(502, "bad gateway");
    let err = ProofsClient::new(&base, "t")
        .signed_url("org/o/k")
        .unwrap_err();
    server.join().unwrap();
    assert_eq!(
        err.to_string(),
        "The proof store refused the request (HTTP 502)."
    );
}

#[test]
fn a_hash_that_is_not_the_files_is_refused() {
    let (base, server) = stub(201, &uploaded_json(&"0".repeat(64)));
    let err = ProofsClient::new(&base, "t")
        .upload("org-1", LinkKind::Claim, "EC-1", &png("a.png"))
        .unwrap_err();
    server.join().unwrap();
    assert_eq!(err, ProofsError::HashMismatch("a.png".into()));
}

#[test]
fn a_signed_link_is_asked_for_by_key_with_the_token() {
    let (base, server) = stub(
        200,
        r#"{"url":"https://proofs.example/signed/org/org%201/abc?exp=1&sig=2","expires_at":1}"#,
    );
    let url = ProofsClient::new(&base, "tok")
        .signed_url("org/org 1/abc")
        .unwrap();
    let seen = server.join().unwrap();
    assert_eq!(seen.request_line, "GET /file/org/org%201/abc HTTP/1.1");
    assert_eq!(seen.header("authorization"), Some("Bearer tok"));
    assert_eq!(
        url,
        "https://proofs.example/signed/org/org%201/abc?exp=1&sig=2"
    );
}

#[test]
fn an_unreachable_worker_is_a_network_error() {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let err = ProofsClient::new(&format!("http://127.0.0.1:{port}"), "t")
        .signed_url("org/o/k")
        .unwrap_err();
    assert!(matches!(err, ProofsError::Network(_)), "{err:?}");
}

#[test]
fn debug_output_leaves_the_token_out() {
    let text = format!(
        "{:?}",
        ProofsClient::new("https://p.example", "secret-token")
    );
    assert!(!text.contains("secret-token"), "{text}");
}

#[test]
fn a_claim_receipt_becomes_a_pending_receipt_proof_with_key_and_hash() {
    let sha = "a".repeat(64);
    let a = Attachment {
        name: "fuel.jpg".into(),
        uploaded: Uploaded {
            key: format!("org/org-1/{sha}"),
            sha256: sha.clone(),
            size: 2048,
            content_type: "image/jpeg".into(),
            thumbnail_key: Some(format!("org/org-1/thumb/{sha}.webp")),
        },
    };
    let v = new_verification(LinkKind::Claim, "EC-0007", &a);
    assert_eq!(
        (
            v.claim_id.as_deref(),
            v.bill_id.as_deref(),
            v.transaction_id.as_deref()
        ),
        (Some("EC-0007"), None, None)
    );
    assert_eq!(v.kind, VerificationType::Receipt);
    assert_eq!(v.object_key, Some(a.uploaded.key.clone()));
    assert_eq!(v.sha256, Some(sha));
    assert_eq!(v.thumbnail_key, a.uploaded.thumbnail_key);
    assert_eq!(v.status, VerificationStatus::Pending);
    assert_eq!(v.description.as_deref(), Some("fuel.jpg"));
    assert_eq!(
        serde_json::Value::Object(v.metadata),
        serde_json::json!({
            "file_name": "fuel.jpg",
            "mime_type": "image/jpeg",
            "size": 2048,
            "object_key": a.uploaded.key,
            "sha256": "a".repeat(64),
        })
    );
    let bill = new_verification(LinkKind::Bill, "bill-1", &a);
    assert_eq!(
        (bill.claim_id, bill.bill_id.as_deref(), bill.kind),
        (None, Some("bill-1"), VerificationType::Invoice)
    );
}

fn uploaded(sha: &str) -> Uploaded {
    Uploaded {
        key: format!("org/o/{sha}"),
        sha256: sha.to_owned(),
        size: 1,
        content_type: "application/pdf".into(),
        thumbnail_key: None,
    }
}

#[test]
fn a_form_keeps_its_files_in_order_and_names_only_the_ones_stored() {
    let mut list = AttachList::default();
    let a = list.start("a.pdf");
    let b = list.start("b.jpg");
    let c = list.start("c.png");
    assert!(list.busy());
    list.finish(b, Err(ProofsError::TooLarge("b.jpg".into())));
    list.finish(a, Ok(uploaded(&"1".repeat(64))));
    assert!(list.busy(), "c is still uploading");
    list.finish(c, Ok(uploaded(&"2".repeat(64))));
    assert!(!list.busy());
    assert_eq!(list.names(), vec!["a.pdf".to_owned(), "c.png".to_owned()]);
    assert_eq!(
        list.rows()[1].state,
        AttachState::Failed("b.jpg is larger than 25 MB.".into())
    );
    list.remove(a);
    assert_eq!(list.names(), vec!["c.png".to_owned()]);
    assert_eq!(list.ready()[0].uploaded.sha256, "2".repeat(64));
}

#[test]
fn an_answer_for_a_removed_file_is_dropped() {
    let mut list = AttachList::default();
    let a = list.start("a.pdf");
    list.remove(a);
    list.finish(a, Ok(uploaded(&"1".repeat(64))));
    assert_eq!(list.rows(), []);
}

#[test]
fn a_transaction_is_a_link_kind_of_its_own() {
    assert_eq!(LinkKind::Transaction.as_str(), "transaction");
    assert_eq!(
        LinkKind::Transaction.proof_type(),
        VerificationType::Receipt
    );
    assert_eq!(SCREEN_MAX_BYTES, 10 * 1024 * 1024);
}

fn photo() -> Attachment {
    let sha = "b".repeat(64);
    Attachment {
        name: "site.png".into(),
        uploaded: Uploaded {
            key: format!("org/org-1/{sha}"),
            sha256: sha,
            size: 12,
            content_type: "image/png".into(),
            thumbnail_key: None,
        },
    }
}

#[test]
fn a_proof_of_a_transaction_carries_what_the_attach_panel_set() {
    let details = ProofDetails {
        kind: VerificationType::Photo,
        visibility: Visibility::Thumbnail,
        description: "  Pipes delivered to the site ".into(),
        executor: " Achieng ".into(),
        gps: Some(Gps {
            latitude: -3.119_5,
            longitude: 35.597_2,
            accuracy: Some(12.0),
            location_name: Some("Lokichar".into()),
        }),
    };
    let v = new_verification_for(LinkKind::Transaction, "txn-1", &details, Some(&photo()));
    assert_eq!(v.transaction_id.as_deref(), Some("txn-1"));
    assert_eq!((v.claim_id, v.bill_id), (None, None));
    assert_eq!(v.kind, VerificationType::Photo);
    assert_eq!(v.visibility_level, Visibility::Thumbnail);
    assert_eq!(
        v.description.as_deref(),
        Some("Pipes delivered to the site")
    );
    assert_eq!(v.executor_name.as_deref(), Some("Achieng"));
    assert_eq!(
        (v.gps_latitude, v.gps_longitude, v.gps_accuracy),
        (Some(-3.119_5), Some(35.597_2), Some(12.0))
    );
    assert_eq!(v.location_name.as_deref(), Some("Lokichar"));
    assert_eq!(v.object_key.as_deref(), Some(photo().uploaded.key.as_str()));
    assert_eq!(v.metadata["file_name"], "site.png");
}

#[test]
fn a_reading_without_a_file_is_a_record_with_no_key_and_no_metadata() {
    let details = ProofDetails {
        kind: VerificationType::Gps,
        visibility: Visibility::Full,
        description: String::new(),
        executor: "   ".into(),
        gps: Some(Gps {
            latitude: 1.0,
            longitude: 2.0,
            accuracy: None,
            location_name: None,
        }),
    };
    let v = new_verification_for(LinkKind::Transaction, "txn-1", &details, None);
    assert_eq!(
        (v.object_key, v.sha256, v.thumbnail_key),
        (None, None, None)
    );
    assert!(v.metadata.is_empty());
    assert_eq!((v.description, v.executor_name), (None, None));
    assert_eq!(v.status, VerificationStatus::Pending);
    assert_eq!(v.gps_latitude, Some(1.0));
}

#[test]
fn a_blank_description_falls_back_to_the_file_name() {
    let details = ProofDetails::of(VerificationType::Receipt);
    let v = new_verification_for(LinkKind::Bill, "bill-1", &details, Some(&photo()));
    assert_eq!(v.description.as_deref(), Some("site.png"));
    assert_eq!(v.bill_id.as_deref(), Some("bill-1"));
    assert_eq!(v.visibility_level, Visibility::Full);
}

#[test]
fn a_signed_links_bytes_are_fetched_without_the_token() {
    let (base, server) = stub(200, "PNGDATA");
    let bytes = ProofsClient::new(&base, "tok")
        .fetch_bytes(&format!("{base}/signed/org/o/abc?exp=1&sig=2"))
        .unwrap();
    let seen = server.join().unwrap();
    assert_eq!(
        seen.request_line,
        "GET /signed/org/o/abc?exp=1&sig=2 HTTP/1.1"
    );
    assert_eq!(seen.header("authorization"), None);
    assert_eq!(bytes, b"PNGDATA");
}

#[test]
fn a_refused_fetch_names_the_workers_reason() {
    let (base, server) = stub(403, r#"{"error":"This link has expired."}"#);
    let err = ProofsClient::new(&base, "tok")
        .fetch_bytes(&format!("{base}/signed/x"))
        .unwrap_err();
    server.join().unwrap();
    assert_eq!(
        err,
        ProofsError::Refused {
            status: 403,
            message: "This link has expired.".into()
        }
    );
}
