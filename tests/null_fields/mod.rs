//! `null` in optional response fields reads as the type's empty value, and a
//! file job's `message` may be a string or a list of strings (joined with
//! "\n"). Both are accepts, not API errors, on the real calls.

use zero_bounce::utility::{
    CONTENT_TYPE_JSON, ENDPOINT_BATCH_VALIDATE, ENDPOINT_FILE_SEND, ENDPOINT_FILE_STATUS,
    ENDPOINT_VALIDATE,
};
use zero_bounce::utility::structures::bulk::{ZBFileFeedback, ZBFile};

use crate::common::{endpoint_matcher, instantiate, generate_zb_file};

fn mock_json(server: &mut mockito::ServerGuard, method: &str, endpoint: &str, body: &str) -> mockito::Mock {
    server
        .mock(method, endpoint_matcher(endpoint))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(body)
        .create()
}

#[test]
fn null_file_id_and_upload_date_read_as_empty() {
    let (mut server, zb) = instantiate();
    let mock = mock_json(
        &mut server,
        "GET",
        ENDPOINT_FILE_STATUS,
        r#"{"success":true,"file_id":null,"file_name":"f.csv","file_status":"Complete",
            "complete_percentage":"100%","return_url":null,"upload_date":null,"error_reason":null}"#,
    );
    let status = zb.bulk_validation_file_status_check("mock_file_id");
    mock.assert();
    let status = status.expect("null file_id and upload_date are accepted");
    assert_eq!(status.file_id, "");
    assert_eq!(status.upload_date.timestamp(), 0);
}

#[test]
fn null_batch_lists_read_as_empty() {
    let (mut server, zb) = instantiate();
    let mock = mock_json(
        &mut server,
        "POST",
        ENDPOINT_BATCH_VALIDATE,
        r#"{"email_batch":null,"errors":null}"#,
    );
    let emails = vec![("valid@example.com".to_string(), "".to_string())];
    let batch = zb.batch_validate(emails);
    mock.assert();
    let batch = batch.expect("null email_batch and errors are accepted");
    assert!(batch.email_batch.is_empty());
    assert!(batch.errors.is_empty());
}

#[test]
#[allow(deprecated)]
fn null_other_domain_formats_read_as_empty_on_every_finder_call() {
    let body = r#"{"email":"a@b.c","domain":"b.c","format":"first","status":"","sub_status":"",
        "confidence":"high","did_you_mean":"","failure_reason":"","other_domain_formats":null,
        "email_confidence":"high","company_name":""}"#;

    let (mut server, zb) = instantiate();
    let mock = server
        .mock("GET", mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(body)
        .create();
    let v1 = zb.find_email("b.c", "a", "", "b").expect("find_email accepts null formats");
    assert!(v1.other_domain_formats.is_empty());
    let v1_domain = zb.domain_search("b.c").expect("domain_search accepts null formats");
    assert!(v1_domain.other_domain_formats.is_empty());
    let v2 = zb
        .domain_search_v2()
        .domain("b.c")
        .call()
        .expect("domain_search_v2 accepts null formats");
    assert!(v2.other_domain_formats.is_empty());
    mock.expect_at_least(3).assert();
}

#[test]
fn null_processed_at_reads_as_unset_date() {
    let (mut server, zb) = instantiate();
    let mock = mock_json(
        &mut server,
        "GET",
        ENDPOINT_VALIDATE,
        r#"{"address":"a@b.c","status":"valid","sub_status":"","free_email":false,"processed_at":null}"#,
    );
    let validation = zb.validate_email("a@b.c");
    mock.assert();
    let validation = validation.expect("null processed_at is accepted");
    assert_eq!(validation.status, "valid");
}

fn submit_with_message(message_json: &str) -> zero_bounce::utility::ZBResult<ZBFileFeedback> {
    let (mut server, zb) = instantiate();
    let body = format!(r#"{{"success":true,"message":{message_json},"file_name":"f.csv","file_id":"id"}}"#);
    let mock = mock_json(&mut server, "POST", ENDPOINT_FILE_SEND, &body);
    let file: ZBFile = generate_zb_file();
    let result = zb.bulk_validation_file_submit(&file);
    mock.assert();
    result
}

#[test]
fn sendfile_message_list_is_joined_with_newlines() {
    let feedback = submit_with_message(r#"["File Accepted","Second line"]"#)
        .expect("a list of messages is accepted");
    assert_eq!(feedback.message, "File Accepted\nSecond line");
}

#[test]
fn sendfile_null_message_reads_as_empty() {
    let feedback = submit_with_message("null").expect("null message is accepted");
    assert_eq!(feedback.message, "");
}

#[test]
fn sendfile_message_of_another_json_type_is_an_api_error_200() {
    for bad in ["5", "true", "{}", "[5]", r#"["a",5]"#] {
        let err = submit_with_message(bad).expect_err(bad);
        let text = format!("{err:?}");
        assert!(text.contains("200") || text.contains("Some(200)"), "{bad}: {text}");
    }
}

/// A struct field as its source declares it.
struct SourceField {
    /// The JSON key: the field name, or its `#[serde(rename)]`.
    key: String,
    /// Whether the field can take `null` without an error by its type or by a
    /// `deserialize_with` helper. A field with neither is a bare `T`, so `null`
    /// is an error unless the field is required.
    null_safe_by_declaration: bool,
}

fn rename_of(attributes: &str) -> Option<String> {
    let start = attributes.find("rename")?;
    let rest = &attributes[start..];
    let open = rest.find('"')? + 1;
    let close = rest[open..].find('"')? + open;
    Some(rest[open..close].to_string())
}

/// The fields of `pub struct <name> {` in `source`, read from the declaration
/// itself, so a field added later cannot escape the sweep.
fn source_fields(source: &str, name: &str) -> Vec<SourceField> {
    let header = format!("pub struct {name} {{");
    let mut lines = source.lines().skip_while(|line| line.trim() != header);
    assert!(lines.next().is_some(), "struct {name} is not declared in the source");
    let mut fields = Vec::new();
    let mut attributes = String::new();
    for line in lines {
        let line = line.trim();
        if line == "}" {
            return fields;
        }
        if line.starts_with("#[") {
            attributes.push_str(line);
        } else if let Some((ident, ty)) = line.strip_prefix("pub ").and_then(|d| d.split_once(':')) {
            let key = rename_of(&attributes).unwrap_or_else(|| ident.trim().to_string());
            let null_safe_by_declaration =
                ty.trim_start().starts_with("Option<") || attributes.contains("deserialize_with");
            fields.push(SourceField { key, null_safe_by_declaration });
            attributes.clear();
        }
    }
    panic!("struct {name} has no closing brace in the source");
}

/// Checks one response type: its fixture holds every declared field, a bare
/// field is required, and `null` in each field reads as the empty value (the
/// call parses), except in the required fields, where `null` must be an error.
fn sweep_struct<T: serde::de::DeserializeOwned>(
    source: &str,
    name: &str,
    fixture: serde_json::Value,
    required: &[&str],
) {
    let fields = source_fields(source, name);
    let mut declared: Vec<&str> = fields.iter().map(|f| f.key.as_str()).collect();
    declared.sort_unstable();
    let mut in_fixture: Vec<&str> = fixture.as_object().expect("fixture is an object").keys().map(String::as_str).collect();
    in_fixture.sort_unstable();
    assert_eq!(in_fixture, declared, "{name}: the fixture must hold every declared field");
    for field in &fields {
        assert!(
            field.null_safe_by_declaration || required.contains(&field.key.as_str()),
            "{name}.{}: a bare field that is not required makes null an error; add a null_as_default",
            field.key
        );
    }
    serde_json::from_value::<T>(fixture.clone()).unwrap_or_else(|e| panic!("{name} fixture: {e}"));
    for field in &fields {
        let mut nulled = fixture.clone();
        nulled[field.key.as_str()] = serde_json::Value::Null;
        let parsed = serde_json::from_value::<T>(nulled);
        if required.contains(&field.key.as_str()) {
            assert!(parsed.is_err(), "{name}.{} is required: null must be an error", field.key);
        } else {
            assert!(parsed.is_ok(), "{name}.{} = null: {:?}", field.key, parsed.err());
        }
    }
}

/// Every field of every typed response reads `null` as its empty value, except
/// the fields a call cannot answer without (`validate`'s `address` and `status`).
/// The struct sources are read at compile time, so the declared fields, not a
/// hand list, decide what is swept.
#[test]
fn null_in_every_response_field_reads_as_empty() {
    use serde_json::{json, Map, Value};
    use zero_bounce::api::priority3::S3ListFileEntry;
    use zero_bounce::utility::structures::bulk::{ZBFileFeedback, ZBFileStatus};
    use zero_bounce::utility::structures::generic::{DomainFormats, ScoreEmailResponse};
    use zero_bounce::utility::structures::validation::{ZBBatchError, ZBBatchValidation, ZBValidation};
    use zero_bounce::utility::structures::{
        ActivityData, ApiUsage, DomainSearchResponseV2, FindEmailResponse, FindEmailResponseV2,
    };

    let generic = include_str!("../../src/utility/structures/generic.rs");
    let validation = include_str!("../../src/utility/structures/validation.rs");
    let bulk = include_str!("../../src/utility/structures/bulk.rs");
    let priority3 = include_str!("../../src/api/priority3.rs");

    sweep_struct::<ZBValidation>(validation, "ZBValidation", json!({
        "address": "a@b.c", "status": "valid", "sub_status": "", "free_email": false,
        "did_you_mean": "x", "account": "x", "domain": "x", "domain_age_days": "1",
        "active_in_days": "1", "active_first_seen": "x", "smtp_provider": "x",
        "catchall_domain": false, "mx_record": "x", "mx_found": "true", "firstname": "x",
        "lastname": "x", "gender": "x", "country": "x", "region": "x", "city": "x",
        "zipcode": "x", "domain_website_exists": "x", "domain_registrant_company_name": "x",
        "Message": "x", "processed_at": "2023-03-23 12:30:28.003"
    }), &["address", "status"]);
    sweep_struct::<ZBBatchError>(validation, "ZBBatchError", json!({"error": "x", "email_address": "x"}), &[]);
    sweep_struct::<ZBBatchValidation>(validation, "ZBBatchValidation", json!({"email_batch": [], "errors": []}), &[]);

    sweep_struct::<ZBFileFeedback>(bulk, "ZBFileFeedback", json!({
        "success": true, "message": "File Accepted", "file_name": "f.csv", "file_id": "id"
    }), &[]);
    sweep_struct::<ZBFileStatus>(bulk, "ZBFileStatus", json!({
        "success": true, "file_id": "id", "file_name": "f.csv", "file_status": "Complete",
        "error_reason": null, "file_phase_2_status": "Complete", "return_url": null,
        "upload_date": "2023-04-26T17:52:23Z", "complete_percentage": "100%"
    }), &[]);

    sweep_struct::<S3ListFileEntry>(priority3, "S3ListFileEntry", json!({
        "name": "x", "size": 1, "last_modified": "x"
    }), &[]);

    let mut usage: Map<String, Value> = source_fields(generic, "ApiUsage")
        .into_iter()
        .map(|field| (field.key, json!(1)))
        .collect();
    usage.insert("start_date".into(), json!("4/1/2023"));
    usage.insert("end_date".into(), json!("4/30/2023"));
    sweep_struct::<ApiUsage>(generic, "ApiUsage", Value::Object(usage), &[]);
    sweep_struct::<ActivityData>(generic, "ActivityData", json!({"found": true, "active_in_days": "180"}), &[]);
    sweep_struct::<ScoreEmailResponse>(generic, "ScoreEmailResponse", json!({"email": "a@b.c", "score": 10}), &[]);
    sweep_struct::<DomainFormats>(generic, "DomainFormats", json!({"format": "first", "confidence": "high"}), &[]);
    sweep_struct::<FindEmailResponse>(generic, "FindEmailResponse", json!({
        "email": "a@b.c", "domain": "b.c", "format": "first", "status": "valid",
        "sub_status": "", "confidence": "high", "did_you_mean": "", "failure_reason": "",
        "other_domain_formats": []
    }), &[]);
    sweep_struct::<FindEmailResponseV2>(generic, "FindEmailResponseV2", json!({
        "email": "a@b.c", "domain": "b.c", "email_confidence": "high", "company_name": "B",
        "did_you_mean": "", "failure_reason": ""
    }), &[]);
    sweep_struct::<DomainSearchResponseV2>(generic, "DomainSearchResponseV2", json!({
        "domain": "b.c", "company_name": "B", "format": "first", "confidence": "high",
        "did_you_mean": "", "failure_reason": "", "other_domain_formats": []
    }), &[]);
}
