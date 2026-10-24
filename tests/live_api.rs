//! Opt-in live API suite: calls every SDK method against the real API to check
//! payloads and response shapes.
//!
//! It is behind the `live` feature, so `cargo test` and CI never compile or run
//! it. Run it explicitly:
//!
//! ```text
//! export ZEROBOUNCE_API_KEY=...          # validation API (skipped without it)
//! export ZEROBOUNCE_WARMUP_API_KEY=...   # Warmup API (skipped without it)
//! cargo test --features live --test live_api -- --nocapture
//! ```
//!
//! By default only free calls run: sandbox addresses (no credits), reads, and
//! requests expected to fail (unknown file IDs, S3/SFTP without an integration)
//! that still prove the API accepts the payload. The free tier checks that the
//! credit balance did not change. Two flags opt in to more:
//!
//! * `ZEROBOUNCE_LIVE_SPEND=1` spends credits: scoring, activity, email finder,
//!   and a bulk round trip whose job is deleted again.
//! * `ZEROBOUNCE_LIVE_CHANGES=1` reversible account changes, each undone in
//!   cleanup even when a test fails: a test filter, and a self-managed Warmup
//!   sending account.
//!
//! Never called, because the API cannot undo them: creating an Inbox Placement
//! test (it cannot be deleted while running), setting the Inbox Placement
//! webhook (it cannot be cleared), and ZB-managed Warmup onboarding (it starts
//! mailbox provisioning).
//!
//! No key is ever written to a file or printed. Keys come from the environment.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use zero_bounce::api::inbox_placement::ListInboxPlacementTestsOptions;
use zero_bounce::utility::structures::validate_options::ZBValidateOptions;
use zero_bounce::{OnboardingModel, ZBFile, ZeroBounce, ZeroBounceWarmup};

/// Well-formed but matching no file: a malformed id can be Cloudflare-blocked
/// instead of answered.
const UNKNOWN_ID: &str = "0d1b7a4e-1111-2222-3333-444455556666";
const CALLBACK: &str = "https://example.com/zerobounce-sdk-live-test";
const SANDBOX: &str = "valid@example.com";

static PASSED: AtomicUsize = AtomicUsize::new(0);
static FAILED: AtomicUsize = AtomicUsize::new(0);
static SKIPPED: AtomicUsize = AtomicUsize::new(0);

fn api_key() -> String {
    std::env::var("ZEROBOUNCE_API_KEY").unwrap_or_default().trim().to_string()
}

fn warmup_key() -> String {
    std::env::var("ZEROBOUNCE_WARMUP_API_KEY").unwrap_or_default().trim().to_string()
}

fn flag(name: &str) -> bool {
    std::env::var(name).map(|value| value == "1").unwrap_or(false)
}

fn redact(text: &str) -> String {
    let mut value = text.to_string();
    for secret in [api_key(), warmup_key()] {
        if !secret.is_empty() {
            value = value.replace(&secret, "***");
        }
    }
    value
}

fn check(name: &str, result: Result<(), String>) {
    match result {
        Ok(()) => {
            PASSED.fetch_add(1, Ordering::Relaxed);
            println!("  + {name}");
        }
        Err(message) => {
            FAILED.fetch_add(1, Ordering::Relaxed);
            println!("  X {name}\n      {}", redact(&message));
        }
    }
}

fn skip(name: &str, reason: &str) {
    SKIPPED.fetch_add(1, Ordering::Relaxed);
    println!("  - SKIP {name} ({reason})");
}

/// Run `call`, expecting it to fail, and hand back the (redacted) message.
fn expect_failure<T>(what: &str, call: impl FnOnce() -> zero_bounce::ZBResult<T>) -> Result<String, String> {
    match call() {
        Ok(_) => Err(format!("{what} unexpectedly succeeded")),
        Err(error) => Ok(redact(&error.to_string())),
    }
}

fn sdk() -> ZeroBounce {
    ZeroBounce::new(&api_key())
}

#[test]
fn live_api_suite() {
    let key = api_key();
    let warmup = warmup_key();
    let spend = flag("ZEROBOUNCE_LIVE_SPEND");
    let changes = flag("ZEROBOUNCE_LIVE_CHANGES");

    println!("ZeroBounce live API suite");
    println!("  validation key: {}", if key.is_empty() { "MISSING" } else { "set" });
    println!("  warmup key:     {}", if warmup.is_empty() { "MISSING" } else { "set" });
    println!(
        "  flags:          SPEND={} CHANGES={}\n",
        spend as u8, changes as u8
    );

    if !key.is_empty() {
        validation_free();
        if spend {
            validation_spend();
        }
        if changes {
            validation_changes();
        }
    }
    if !warmup.is_empty() {
        warmup_free();
        if changes {
            warmup_changes();
        }
    }

    let failed = FAILED.load(Ordering::Relaxed);
    println!(
        "\n{} passed, {failed} failed, {} skipped",
        PASSED.load(Ordering::Relaxed),
        SKIPPED.load(Ordering::Relaxed)
    );
    assert_eq!(failed, 0, "{failed} live checks failed");
}

// -- validation API, free -----------------------------------------------------

fn validation_free() {
    println!("Validation API (free calls)");
    let zb = sdk();
    let before = zb.get_credits().unwrap_or(-2);

    check(
        "get_credits returns a balance",
        if before >= -1 {
            Ok(())
        } else {
            Err("could not read the credit balance".into())
        },
    );

    check("get_api_usage parses the documented date formats", || -> Result<(), String> {
        let end = Utc::now().date_naive();
        let start = end - ChronoDuration::days(7);
        zb.get_api_usage(start, end).map_err(|e| e.to_string())?;
        Ok(())
    }());

    check("validate_email on a sandbox address spends nothing", || -> Result<(), String> {
        let response = zb.validate_email(SANDBOX).map_err(|e| e.to_string())?;
        if response.address != SANDBOX {
            return Err(format!("address did not echo: {}", response.address));
        }
        Ok(())
    }());

    check("validate_email works over POST as well as GET", || -> Result<(), String> {
        // No billable add-on here: activity_data, verify_plus and domain_info
        // are charged even on a sandbox address, so they live in the SPEND tier.
        let options = ZBValidateOptions {
            timeout: Some(30),
            use_post: true,
            ..Default::default()
        };
        zb.validate_email_with_options(SANDBOX, "", &options)
            .map_err(|e| e.to_string())?;
        Ok(())
    }());

    check("LIVE: validate_batch accepts timeout as a string", || -> Result<(), String> {
        // Sent as a JSON number the API rejects the whole batch with the
        // account-level "Invalid API Key or your account ran out of credits".
        let options = ZBValidateOptions {
            timeout: Some(30),
            ..Default::default()
        };
        let response = zb
            .batch_validate_with_options(vec![(SANDBOX.to_string(), String::new())], &options)
            .map_err(|e| e.to_string())?;
        if response
            .errors
            .iter()
            .any(|error| error.email_address.eq_ignore_ascii_case("all"))
        {
            return Err(format!("account-level error: {:?}", response.errors));
        }
        Ok(())
    }());

    check("an unknown file id is a short, clean error", || -> Result<(), String> {
        let message = expect_failure("bulk_validation_file_status_check", || {
            zb.bulk_validation_file_status_check(UNKNOWN_ID)
        })?;
        if message.len() >= 500 || message.contains("<html") {
            return Err(format!("message not short and clean: {message}"));
        }
        Ok(())
    }());

    check("list_s3_files answers (empty without an S3 integration)", || -> Result<(), String> {
        match zb.list_s3_files(None, None) {
            Ok(_) => Ok(()),
            // No S3 integration configured is an expected, documented failure.
            Err(error) if error.to_string().len() < 500 => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }());

    check("LIVE: S3/SFTP jobs reject a payload without return_url before sending", || -> Result<(), String> {
        let message = expect_failure("email_finder_s3_file without return_url", || {
            zb.email_finder_s3_file("f.csv", true, None, "")
        })?;
        if !message.contains("return_url") {
            return Err(format!("unexpected message: {message}"));
        }
        Ok(())
    }());

    check("LIVE: a complete S3 job payload is accepted by the API", || -> Result<(), String> {
        // Without an S3 integration this fails on the integration, not the payload.
        match zb.email_finder_s3_file("f.csv", true, None, CALLBACK) {
            Ok(_) => Ok(()),
            Err(error) => {
                let message = redact(&error.to_string());
                if message.contains("return URL is not permitted")
                    || message.contains("This field is required")
                {
                    return Err(format!("payload rejected: {message}"));
                }
                Ok(())
            }
        }
    }());

    check("filters list", zb.list_filters(false).map(|_| ()).map_err(|e| e.to_string()));

    check(
        "LIVE: the Inbox Placement Tester accepts a Bearer key",
        zb.get_inbox_placement_providers().map(|_| ()).map_err(|e| e.to_string()),
    );
    check(
        "inbox placement test types",
        zb.get_inbox_placement_test_types().map(|_| ()).map_err(|e| e.to_string()),
    );
    check("LIVE: inbox placement tests are listed from /tests", || -> Result<(), String> {
        // The listing is GET /tests (plural); /test is the single-test path and
        // answers "Method \"GET\" not allowed".
        let options = ListInboxPlacementTestsOptions {
            page: Some(1),
            page_size: Some(10),
            ..Default::default()
        };
        zb.list_inbox_placement_tests(&options).map_err(|e| e.to_string())?;
        zb.get_inbox_placement_webhook().map_err(|e| e.to_string())?;
        Ok(())
    }());

    check("list evaluator result for an unknown id fails cleanly", || -> Result<(), String> {
        let message = expect_failure("get_list_evaluator_result", || {
            zb.get_list_evaluator_result(UNKNOWN_ID)
        })?;
        if message.len() >= 500 {
            return Err(format!("message too long: {}", message.len()));
        }
        Ok(())
    }());

    check("LIVE: send_evaluated_file posts the file id as a form field", || -> Result<(), String> {
        let message = expect_failure("send_evaluated_file", || zb.send_evaluated_file(UNKNOWN_ID))?;
        if message.to_ascii_lowercase().contains("not allowed") {
            return Err(format!("wrong method or path: {message}"));
        }
        Ok(())
    }());

    check("the free calls did not change the credit balance", || -> Result<(), String> {
        let after = sdk().get_credits().map_err(|e| e.to_string())?;
        if after != before {
            return Err(format!("credits went from {before} to {after}"));
        }
        Ok(())
    }());
}

// -- validation API, spends credits -------------------------------------------

fn validation_spend() {
    println!("\nValidation API (ZEROBOUNCE_LIVE_SPEND=1)");
    let zb = sdk();

    check("validate_email accepts every documented option", || -> Result<(), String> {
        // LIVE: activity_data, verify_plus and domain_info are billable add-ons
        // and are charged even for a sandbox address, so this is not free.
        let options = ZBValidateOptions {
            timeout: Some(30),
            activity_data: Some(true),
            verify_plus: Some(true),
            domain_info: Some(true),
            greylist_processing: Some(true),
            callback_url: Some(CALLBACK.to_string()),
            use_post: false,
        };
        zb.validate_email_with_options(SANDBOX, "", &options)
            .map_err(|e| e.to_string())?;
        Ok(())
    }());

    check("score_email", zb.score_email(SANDBOX).map(|_| ()).map_err(|e| e.to_string()));
    check("score_email over POST", zb.score_email_post(SANDBOX).map(|_| ()).map_err(|e| e.to_string()));
    check("get_activity_data", zb.get_activity_data(SANDBOX).map(|_| ()).map_err(|e| e.to_string()));
    check(
        "email finder",
        zb.find_email_v2()
            .first_name("John")
            .domain("example.com")
            .last_name("Doe")
            .call()
            .map(|_| ())
            .map_err(|e| e.to_string()),
    );
    check(
        "domain search",
        zb.domain_search_v2().domain("example.com").call().map(|_| ()).map_err(|e| e.to_string()),
    );

    let mut file_id = None;
    check("bulk validation round trip: upload, status, delete", || -> Result<(), String> {
        let csv = b"email\nvalid@example.com\ninvalid@example.com\n".to_vec();
        let file = ZBFile::from_content_with_filename(csv, "zb-sdk-live.csv");
        let sent = zb.bulk_validation_file_submit(&file).map_err(|e| e.to_string())?;
        let Some(id) = sent.file_id.clone() else {
            return Err(format!("no file_id in the upload response: {sent:?}"));
        };
        let status = zb.bulk_validation_file_status_check(&id).map_err(|e| e.to_string())?;
        if status.file_id != id {
            return Err("status did not echo the file id".into());
        }
        file_id = Some(id);
        Ok(())
    }());

    match file_id {
        None => skip(
            "the uploaded validation job is deleted again",
            "upload did not produce a file id",
        ),
        Some(id) => check("the uploaded validation job is deleted again", || -> Result<(), String> {
            // LIVE: a just-accepted job answers "File cannot be found" for a
            // short while, so the delete is retried rather than leaving it behind.
            let mut last = String::new();
            for _ in 0..12 {
                match zb.bulk_validation_result_delete(&id) {
                    Ok(_) => return Ok(()),
                    Err(error) => {
                        last = redact(&error.to_string());
                        std::thread::sleep(Duration::from_secs(10));
                    }
                }
            }
            Err(format!("could not delete {id} after retrying: {last}"))
        }()),
    }
}

// -- validation API, reversible account changes -------------------------------

fn validation_changes() {
    println!("\nValidation API (ZEROBOUNCE_LIVE_CHANGES=1)");
    let zb = sdk();
    let value = format!("zb-sdk-live-{}.example.com", Utc::now().timestamp());

    let added = zb.add_filter("allow", "domain", &value);
    check(
        "add_filter creates a reversible allow rule",
        added.as_ref().map(|_| ()).map_err(|e| e.to_string()),
    );
    if added.is_ok() {
        check(
            "the test filter is removed again",
            zb.delete_filter("allow", "domain", &value).map(|_| ()).map_err(|e| e.to_string()),
        );
    } else {
        skip("the test filter is removed again", "filter was not created");
    }
}

// -- Warmup API ---------------------------------------------------------------

fn warmup_client() -> ZeroBounceWarmup {
    ZeroBounceWarmup::new(&warmup_key()).expect("warmup key")
}

fn warmup_free() {
    println!("\nWarmup API (reads)");
    let zb = warmup_client();

    check("list_sending_accounts", zb.list_sending_accounts().map(|_| ()).map_err(|e| e.to_string()));
    check("list_from_addresses", zb.list_from_addresses(None, None, None).map(|_| ()).map_err(|e| e.to_string()));
    check("get_engagement_rules", zb.get_engagement_rules().map(|_| ()).map_err(|e| e.to_string()));
    check("get_seeds", zb.get_seeds().map(|_| ()).map_err(|e| e.to_string()));
    check("get_seed_count", zb.get_seed_count().map(|_| ()).map_err(|e| e.to_string()));
    check("get_progress_today", zb.get_progress_today().map(|_| ()).map_err(|e| e.to_string()));
    check("get_sent_today", zb.get_sent_today().map(|_| ()).map_err(|e| e.to_string()));
    check("get_emails_per_day", zb.get_emails_per_day().map(|_| ()).map_err(|e| e.to_string()));

    check("download_seeds returns CSV text", || -> Result<(), String> {
        let csv = zb.download_seeds().map_err(|e| e.to_string())?;
        if csv.trim().starts_with('{') {
            return Err("got JSON instead of CSV".into());
        }
        Ok(())
    }());

    check("LIVE: check_domain_dmarc rejects a bare domain before sending", || -> Result<(), String> {
        let message = expect_failure("check_domain_dmarc with a domain", || {
            zb.check_domain_dmarc("example.com", None)
        })?;
        if !message.to_ascii_lowercase().contains("email address") {
            return Err(format!("unexpected message: {message}"));
        }
        Ok(())
    }());

    check(
        "LIVE: check_domain_dmarc accepts an email address",
        zb.check_domain_dmarc("sender@example.com", None).map(|_| ()).map_err(|e| e.to_string()),
    );

    // The statistics endpoints need a from-address this team actually has; the
    // API answers "Sending account ... was not found" for anything else.
    let known = zb.list_sending_accounts().ok().flatten().and_then(|value| {
        let list = value
            .get("data")
            .and_then(|data| data.as_array())
            .cloned()
            .or_else(|| value.as_array().cloned())?;
        let first = list.first()?.clone();
        ["from_address", "c_from", "fromAddress"]
            .iter()
            .find_map(|key| first.get(*key).and_then(|v| v.as_str()).map(str::to_string))
    });

    match known {
        None => {
            for name in [
                "statistics for a registered from-address",
                "statistics per day over a short range",
                "detailed statistics and the CSV download for that address",
                "from-address statistics",
            ] {
                skip(name, "this team has no sending account");
            }
        }
        Some(address) => {
            check(
                "statistics for a registered from-address",
                zb.get_statistics(&address).map(|_| ()).map_err(|e| e.to_string()),
            );
            let end = Utc::now().date_naive();
            check(
                "statistics per day over a short range",
                zb.get_statistics_per_day(&address, end - ChronoDuration::days(6), end)
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
            );
            check("detailed statistics and the CSV download for that address", || -> Result<(), String> {
                zb.get_statistics_detailed(&address).map_err(|e| e.to_string())?;
                zb.download_statistics(&address).map_err(|e| e.to_string())?;
                Ok(())
            }());
            check(
                "from-address statistics",
                zb.get_from_address_statistics(&address).map(|_| ()).map_err(|e| e.to_string()),
            );
        }
    }

    check("an unknown Warmup resource fails with a status code", || -> Result<(), String> {
        match zb.get_custom_engagement("no-such-address@example.com") {
            Ok(_) => Ok(()),
            Err(error) => {
                if error.status_code().is_none() {
                    return Err("no status code on the error".into());
                }
                if error.to_string().contains(&warmup_key()) {
                    return Err("the key leaked into the message".into());
                }
                Ok(())
            }
        }
    }());
}

fn warmup_changes() {
    println!("\nWarmup API (ZEROBOUNCE_LIVE_CHANGES=1)");
    let zb = warmup_client();
    let address = format!("zb-sdk-live-{}@example.com", Utc::now().timestamp());

    let added = zb.add_sending_account(
        &address,
        OnboardingModel::SelfManaged,
        Some(0),
        Some(true),
        None,
        None,
    );
    check(
        "a self-managed sending account can be added",
        added.as_ref().map(|_| ()).map_err(|e| e.to_string()),
    );
    if added.is_ok() {
        check(
            "the test sending account is removed again",
            zb.delete_sending_account(&address).map(|_| ()).map_err(|e| e.to_string()),
        );
    } else {
        skip("the test sending account is removed again", "account was not created");
    }
}
