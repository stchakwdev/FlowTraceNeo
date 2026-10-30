#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use super::*;

fn proof(kind: VerificationType) -> Verification {
    Verification {
        id: "v-1".into(),
        transaction_id: Some("txn-1".into()),
        claim_id: None,
        bill_id: None,
        kind,
        object_key: Some(format!("org/o/{}", "a".repeat(64))),
        sha256: Some("a".repeat(64)),
        ocr_data: None,
        metadata: serde_json::json!({ "mime_type": "image/png", "file_name": "r.png" })
            .as_object()
            .unwrap()
            .clone(),
        status: VerificationStatus::Pending,
        verified_by: None,
        verified_at: None,
        created_at: "2026-02-04T09:00:00.000Z".into(),
        trust_points: kind.trust_points(),
        visibility_level: Visibility::Full,
        thumbnail_key: None,
        executor_name: None,
        gps_latitude: None,
        gps_longitude: None,
        gps_accuracy: None,
        location_name: None,
        description: Some("Fuel receipt".into()),
        created_by: Some("u-1".into()),
        published: false,
        visible_as: Some(Visibility::Full),
    }
}

/// `VERIFICATION_TRUST_POINTS` as `flowtrace/src/lib/types.ts` writes it, one `key: n,` per line.
fn web_trust_points() -> Vec<(String, i64)> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../flowtrace/src/lib/types.ts"
    );
    let source = std::fs::read_to_string(path).unwrap();
    let start = source
        .find("export const VERIFICATION_TRUST_POINTS")
        .unwrap();
    let block = &source[start..];
    let block = &block[block.find('{').unwrap() + 1..block.find('}').unwrap()];
    block
        .lines()
        .filter_map(|l| l.trim().trim_end_matches(',').split_once(':'))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().parse().unwrap()))
        .collect()
}

#[test]
fn trust_points_are_the_webs_for_every_type() {
    let web = web_trust_points();
    assert_eq!(web.len(), 12, "{web:?}");
    for (key, points) in web {
        let kind: VerificationType = serde_json::from_value(serde_json::json!(key)).unwrap();
        assert_eq!(kind.trust_points(), points, "{key}");
    }
}

#[test]
fn a_rows_own_points_win_over_its_types() {
    let mut v = proof(VerificationType::Receipt);
    assert_eq!(trust_points(&v), 5);
    v.trust_points = 9;
    assert_eq!(trust_points(&v), 9);
    v.trust_points = 0;
    assert_eq!(trust_points(&v), 5);
}

#[test]
fn labels_follow_the_gallery() {
    assert_eq!(type_label(VerificationType::Gps), "GPS reading");
    assert_eq!(type_label(VerificationType::Audit), "Third-party audit");
    assert_eq!(visibility_label(Visibility::Thumbnail), "Thumbnail only");
    assert_eq!(state_label(VerificationStatus::Pending), "Awaiting review");
    assert_eq!(
        visibility_description(Visibility::Summary),
        "Partners see that proof is attached, not the file."
    );
}

#[test]
fn the_attach_panel_offers_eleven_kinds_with_their_points_in_the_webs_order() {
    let choices = attach_choices();
    assert_eq!(choices.len(), 11);
    assert_eq!(choices[0].1, "Receipt (+5)");
    assert_eq!(choices[2].1, "Photo (+8)");
    assert_eq!(choices[6].1, "GPS reading (+3)");
    assert_eq!(choices[10].1, "Document (+4)");
    assert!(!ATTACH_TYPES.contains(&VerificationType::Biometric));
    let new_form = new_form_choices();
    assert_eq!(new_form[2].1, "Bank statement (+10 trust points)");
    assert_eq!(new_form.len(), 6);
}

#[test]
fn accepted_types_are_described_as_the_web_does() {
    assert_eq!(
        describe_accepted(accepted(VerificationType::Receipt)),
        "JPG, PNG, WebP or PDF"
    );
    assert_eq!(describe_accepted(accepted(VerificationType::Audit)), "PDF");
    assert_eq!(
        describe_accepted(accepted(VerificationType::Invoice)),
        "PDF, JPG or PNG"
    );
    assert_eq!(
        upload_hint(VerificationType::Photo, 0),
        "Photo: JPG, PNG or WebP, up to 10 MB each"
    );
    assert_eq!(
        upload_hint(VerificationType::Photo, 5),
        "That's the limit of 5 files at once."
    );
}

#[test]
fn file_sizes_read_as_format_file_size() {
    assert_eq!(format_file_size(0), "0 B");
    assert_eq!(format_file_size(512), "512 B");
    assert_eq!(format_file_size(1536), "1.5 KB");
    assert_eq!(format_file_size(2 * 1024 * 1024), "2 MB");
    assert_eq!(format_file_size(11 * 1024 * 1024 + 200_000), "11.2 MB");
}

#[test]
fn a_file_is_refused_in_the_webs_words() {
    let receipt = accepted(VerificationType::Receipt);
    assert_eq!(
        validate_proof_file("notes.docx", None, 10, receipt, 10),
        Some("notes.docx is not a JPG, PNG, WebP or PDF file.".into())
    );
    assert_eq!(
        validate_proof_file("scan.gif", Some("image/gif"), 10, receipt, 10),
        Some("scan.gif is not a JPG, PNG, WebP or PDF file.".into())
    );
    assert_eq!(
        validate_proof_file(
            "big.pdf",
            Some("application/pdf"),
            11 * 1024 * 1024,
            receipt,
            10
        ),
        Some("big.pdf is 11 MB; the limit is 10 MB.".into())
    );
    assert_eq!(
        validate_proof_file("r.png", Some("image/png"), 10, receipt, 10),
        None
    );
    assert_eq!(
        file_row_status(2048, true, None),
        "2 KB \u{b7} Uploading\u{2026}"
    );
    assert_eq!(file_row_status(2048, false, None), "2 KB \u{b7} Uploaded");
    assert_eq!(file_row_status(2048, false, Some("gone")), "gone");
}

#[test]
fn a_gallery_row_names_who_when_and_a_limited_audience() {
    let mut v = proof(VerificationType::Photo);
    let row = ProofRow::of(&v);
    assert_eq!(row.label, "Photo");
    assert_eq!(row.points, "+8");
    assert_eq!(row.added, "Added 4 Feb 2026");
    assert_eq!(row.limited, None);
    assert_eq!(row.media, Some(MediaKind::Image));
    assert_eq!(row.state, "Awaiting review");
    v.executor_name = Some("Achieng".into());
    v.location_name = Some("Lokichar".into());
    v.visibility_level = Visibility::Summary;
    let row = ProofRow::of(&v);
    assert_eq!(row.added, "Added 4 Feb 2026 by Achieng, Lokichar");
    assert_eq!(row.limited, Some("Summary only"));
}

#[test]
fn the_section_meta_counts_proofs_and_points() {
    assert_eq!(proofs_meta(&[]), None);
    let one = proof(VerificationType::Receipt);
    assert_eq!(
        proofs_meta(std::slice::from_ref(&one)).as_deref(),
        Some("1 proof \u{b7} +5 trust points")
    );
    let two = [one, proof(VerificationType::Photo)];
    assert_eq!(
        proofs_meta(&two).as_deref(),
        Some("2 proofs \u{b7} +13 trust points")
    );
    assert_eq!(more_line(1), "1 more proof");
    assert_eq!(more_line(3), "3 more proofs");
}

#[test]
fn proofs_list_newest_first() {
    let mut a = proof(VerificationType::Receipt);
    a.id = "a".into();
    a.created_at = "2026-01-01T00:00:00.000Z".into();
    let mut b = proof(VerificationType::Photo);
    b.id = "b".into();
    b.created_at = "2026-03-01T00:00:00.000Z".into();
    let ids: Vec<String> = newest_first(vec![a, b]).into_iter().map(|v| v.id).collect();
    assert_eq!(ids, ["b", "a"]);
}

#[test]
fn media_kind_comes_from_the_recorded_type() {
    let mut v = proof(VerificationType::Receipt);
    v.metadata
        .insert("mime_type".into(), "application/pdf".into());
    assert_eq!(media_kind(&v), Some(MediaKind::Pdf));
    v.metadata.insert("mime_type".into(), "video/mp4".into());
    assert_eq!(media_kind(&v), Some(MediaKind::Video));
    v.metadata.clear();
    assert_eq!(media_kind(&v), None);
}

#[test]
fn attaching_needs_a_file_or_for_gps_a_location() {
    let mut form = AttachForm::default();
    assert_eq!(form.points_line(), "+5 trust points each");
    assert_eq!(
        form.check(0).unwrap_err(),
        "Add at least one file before attaching."
    );
    form.kind = VerificationType::Gps;
    assert_eq!(
        form.check(0).unwrap_err(),
        "Add a file or your current location before attaching."
    );
    form.latitude = "-3.1195".into();
    form.longitude = "35.5972".into();
    form.accuracy = "12".into();
    form.place = " Lokichar ".into();
    let details = form.check(0).unwrap();
    assert_eq!(
        details.gps,
        Some(Gps {
            latitude: -3.1195,
            longitude: 35.5972,
            accuracy: Some(12.0),
            location_name: Some("Lokichar".into()),
        })
    );
    form.kind = VerificationType::Receipt;
    assert!(form.check(0).is_err(), "a location alone is not a receipt");
    assert!(form.check(1).is_ok());
}

#[test]
fn a_typed_location_is_checked() {
    let mut form = AttachForm {
        latitude: "91".into(),
        longitude: "1".into(),
        ..AttachForm::default()
    };
    assert_eq!(
        form.gps().unwrap_err(),
        "Enter the latitude as a number from -90 to 90."
    );
    form.latitude = "1".into();
    form.longitude = String::new();
    assert_eq!(
        form.gps().unwrap_err(),
        "Enter the longitude as a number from -180 to 180."
    );
    form.longitude = "2".into();
    form.accuracy = "-4".into();
    assert_eq!(
        form.gps().unwrap_err(),
        "Enter the accuracy in metres, or leave it blank."
    );
    form.accuracy = String::new();
    assert_eq!(form.gps().unwrap().unwrap().accuracy, None);
}

#[test]
fn the_attach_button_and_failure_read_as_the_web() {
    assert_eq!(attach_label(true, 3), "Attaching\u{2026}");
    assert_eq!(attach_label(false, 3), "Attach 3 proofs");
    assert_eq!(attach_label(false, 1), "Attach proof");
    assert_eq!(attach_label(false, 0), "Attach proof");
    assert_eq!(
        attach_failure("Transaction t not found."),
        "The proof was not attached: Transaction t not found.. Your files are still here; try again."
    );
}

#[test]
fn the_viewer_zooms_from_half_to_three_times() {
    let mut z = Zoom::default();
    assert_eq!(z.percent(), "100%");
    z = z.out().out().out();
    assert_eq!(z.percent(), "50%");
    assert!(!z.can_out());
    for _ in 0..20 {
        z = z.into();
    }
    assert_eq!(z.percent(), "300%");
    assert!(!z.can_in());
    assert_eq!(position(0, 1), None);
    assert_eq!(position(1, 3).as_deref(), Some("Proof 2 of 3"));
}

#[test]
fn the_record_panel_lists_its_facts() {
    let mut v = proof(VerificationType::Gps);
    v.executor_name = Some("Achieng".into());
    v.gps_latitude = Some(-3.119_512_3);
    v.gps_longitude = Some(35.597_2);
    v.gps_accuracy = Some(12.4);
    v.status = VerificationStatus::Verified;
    v.verified_at = Some("2026-02-05T10:00:00.000Z".into());
    let facts = viewer_facts(&v);
    let terms: Vec<&str> = facts.iter().map(|f| f.term).collect();
    assert_eq!(
        terms,
        [
            "Captured by",
            "Location",
            "Added",
            "Verified",
            "Who can see it"
        ]
    );
    assert_eq!(
        facts[1].note.as_deref(),
        Some("-3.11951, 35.59720 (\u{b1}12 m)")
    );
    assert_eq!(facts[2].value, "4 Feb 2026, 09:00");
    assert_eq!(facts[3].value, "5 Feb 2026");
    assert_eq!(facts[4].value, "Full file");
    assert_eq!(
        facts[4].note.as_deref(),
        Some("Board and staff see the original file.")
    );
}

#[test]
fn the_viewer_body_follows_the_file() {
    let mut v = proof(VerificationType::Receipt);
    assert_eq!(viewer_body(&v, &Fetch::Pending), ViewerBody::Opening);
    assert_eq!(viewer_body(&v, &Fetch::Ready), ViewerBody::Image);
    assert_eq!(
        viewer_body(&v, &Fetch::Failed("Gone.".into())),
        ViewerBody::Failed("Gone. Check your connection and try again.".into())
    );
    v.metadata
        .insert("mime_type".into(), "application/pdf".into());
    assert_eq!(viewer_body(&v, &Fetch::Ready), ViewerBody::NoPreview);
    v.object_key = None;
    assert_eq!(viewer_body(&v, &Fetch::Ready), ViewerBody::NoFile);
}

#[test]
fn only_a_manager_publishes_and_only_a_file() {
    let mut v = proof(VerificationType::Receipt);
    assert_eq!(publish_label(&v, false), None);
    assert_eq!(publish_label(&v, true), Some("Publish at the public link"));
    v.published = true;
    assert_eq!(publish_label(&v, true), Some("Take off the public link"));
    v.published = false;
    v.object_key = None;
    assert_eq!(publish_label(&v, true), None);
}

#[test]
fn a_file_is_read_only_when_its_kind_takes_it() {
    let dir = std::env::temp_dir().join(format!("ft-dt3-read-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let png = dir.join("site.png");
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    bytes.extend_from_slice(&[0; 40]);
    std::fs::write(&png, &bytes).unwrap();
    let file = read_proof_file(&png, VerificationType::Photo, 10).unwrap();
    assert_eq!(
        (file.name.as_str(), file.content_type),
        ("site.png", "image/png")
    );
    assert_eq!(file.bytes, bytes);
    assert_eq!(
        read_proof_file(&png, VerificationType::Audit, 10).unwrap_err(),
        "site.png is not a PDF file."
    );
    let text = dir.join("notes.txt");
    std::fs::write(&text, "hello").unwrap();
    assert_eq!(
        read_proof_file(&text, VerificationType::Receipt, 10).unwrap_err(),
        "notes.txt is not a JPG, PNG, WebP or PDF file."
    );
    let big = dir.join("big.png");
    let mut large = bytes.clone();
    large.resize(2 * 1024 * 1024, 0);
    std::fs::write(&big, &large).unwrap();
    assert_eq!(
        read_proof_file(&big, VerificationType::Photo, 1).unwrap_err(),
        "big.png is 2 MB; the limit is 1 MB."
    );
    assert!(
        read_proof_file(&dir.join("missing.png"), VerificationType::Photo, 10)
            .unwrap_err()
            .starts_with("missing.png could not be read")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_failed_upload_reads_as_the_webs() {
    assert_eq!(
        upload_failure("r.png", "The proof store could not be reached: refused."),
        "r.png did not upload: The proof store could not be reached: refused. Check your connection and try again."
    );
    assert_eq!(
        upload_failure("r.png", ""),
        "r.png did not upload. Check your connection and try again."
    );
}
