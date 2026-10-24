# AGENTS.md

Guidance for AI coding agents working in this repository.

## Project

`zerobounce-rust-api` publishes the `zero-bounce` crate. Callers construct `ZeroBounce::new(api_key)`. Each value owns its key, HTTP client, and base URLs.

## Layout

- `src/lib.rs`: client, URL builder, and the shared GET and form POST helpers.
- `src/api/validation.rs`: single validate and `batch_validate` on the main API host.
- `src/api/bulk.rs`: bulk validation and AI scoring file methods on `bulkapi`.
- `src/api/priority3.rs`: bulk email finder, domain search, list evaluator, filters, and the S3 / SFTP bulk jobs. Finder and domain search paths are on the bulk host without `/v2`. The six file jobs go through `bulk_root_file_job`; two shapes differ from the rest and are easy to get wrong — `validate_sftp_file`'s `file_name` is optional (hence `Option`), and `export_sftp_file` takes `export_path`, not `export_bucket`.
- `src/api/inbox_placement.rs`: Inbox Placement Tester.
- `src/utility/http_error.rs`: turns HTTP failures, HTML pages, and empty or non-JSON bodies into a short `ZBError`.
- `tests/offline_api.rs`: offline suite. A local HTTP server records every request. The client is pointed at `http://127.0.0.1`, which the URL check allows.
- Version is `version` in `Cargo.toml`. `Cargo.lock` records the same package version. There is no runtime version constant.

## Commands

```bash
cargo test --no-fail-fast
cargo clippy --all-targets
cargo audit            # check dependencies against the advisory database
```

The live API suite is behind the `live` feature, so `cargo test` and CI
never compile or run it:

```bash
export ZEROBOUNCE_API_KEY=...          # validation API
export ZEROBOUNCE_WARMUP_API_KEY=...   # Warmup API
cargo test --features live --test live_api -- --nocapture          # free calls only
ZEROBOUNCE_LIVE_SPEND=1 cargo test --features live --test live_api -- --nocapture
ZEROBOUNCE_LIVE_CHANGES=1 cargo test --features live --test live_api -- --nocapture
```

Keys come from the environment only. Never write one into a file, a commit, a
log, a fixture or a message.

`cargo test` compiles the crate and the integration tests. That is the typecheck. CI (`.github/workflows/sdk_ci.yml`) runs `cargo clippy --all-targets` and `cargo test --no-fail-fast`. Clippy is not denied: existing tests use `assert_eq!` on booleans, and `cargo fmt --check` does not pass on the examples. Do not reformat files you did not change.

## Conventions

- **Only endpoints documented at https://www.zerobounce.net/docs/.** Do not add a method for an endpoint that the public docs do not describe, even if it exists on the server. Check before concluding something is undocumented: the S3 bulk endpoints live under https://www.zerobounce.net/docs/api-integrations/amazon-s3-bulk-api, not the main API reference.
- **The list evaluator and the Inbox Placement Tester have their own base URLs** on `ZBUrlProvider` (`list_evaluator_url`, `inbox_placement_url`). They are not built from `bulk_url`, and `send_evaluated_file` stays on the bulk base. Offline tests must point both at the fake server or they reach the live API.
- **Where the live API and the docs disagree, follow the API and say so in a comment at the call site.** The ones found so far: `validatebatch` takes `timeout` as a JSON *string*; the Inbox Placement Tester authenticates with `Authorization: Bearer`, not `api_key`; its listing is `GET /tests`, not `/test`; `send-evaluated-file` takes `file_id` as a form field, not a file upload; the S3 and SFTP jobs require `return_url` and `file_name`, `validateSFTPfile` included; the Warmup `check-domain-dmarc` takes an email address, not a domain; and a failed SMTP login in `test-smtp-configuration` comes back as an HTTP 500.
- **The Inbox Placement Tester takes the API key as an `Authorization: Bearer` header and JSON bodies with camelCase fields.** Only fields the caller set are sent. Its error bodies use `detail`.
- **`{"success": false}` on an HTTP 200 is a failure.** The API answers an unknown bulk file id that way. `ensure_json_api_body` turns it into a `ZBError::ApiError` carrying the status.
- **`src/warmup.rs` is a separate client for the Warmup API**, a different product with its own host and its own team-scoped key. It is not part of `ZeroBounce`.

- **One `ZeroBounce` per API key.** `api_key` is a field on the value. Do not share one value across users. The offline concurrency test builds a separate client per key.
- **Column indexes.** `email_address_column` is a `u32` and is sent as given, including `0`. Name, gender, and IP columns are `Option<u32>`: `None` is omitted, and `Some(0)` is sent. Bulk finder `domain_column` of `0` is rejected.
- **Optional parameters are omitted when unset.** `timeout`, `activity_data`, `verify_plus`, and `allow_phase_2` are `Option` and are not sent until the caller sets them. Explicit `false` is sent as `false`.
- **Booleans are the strings `true` and `false`.** `bool::to_string` does that. Do not send `1` or `0`. `allow_phase_2` is validation `sendfile` only. `activity_data` on getfile is validation only. AI scoring `sendfile` does not send name, gender, or IP columns.
- **Empty `ip_address` is omitted** on single validate. Batch items always include `ip_address`, and an empty string is sent.
- **`validate` timeout is 3–60 seconds. `batch_validate` timeout is 10–120 seconds.** Out-of-range values are rejected before a request.
- **Failures are `ZBError`, which implements `std::error::Error`.** The message must be short: no raw HTML, no megabyte body, no serde dump, and no `Error: Error:` prefix. An account-level batch error (`email_address` `all`) is `Err`. A per-address error stays on the `Ok` result. A failed API response is `ZBError::ApiError`, whose status is reachable through `ZBError::status_code()`; the key is never in the message.
- **Tests must not call the live API.** Point `url_provider` at `http://127.0.0.1` or use `mockito`.
- Match the surrounding style. Do not reformat unrelated code.

## Changelog

`CHANGELOG.md` follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/), and the project adheres to [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

- Add every user-visible change to `## [Unreleased]` in the same commit as the change.
- Group entries under these headings, in this order, omitting empty ones: `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`.
- Write for SDK users: name the method, option, or behaviour, and say what changed. Leave out CI-only changes, internal refactors, and fixes to code that was never released.
- At 2.y.z, a breaking change bumps the major version, a new method or option bumps the minor version, and fixes alone bump the patch version.

## Releasing

Publishing and pushing tags need a maintainer's explicit approval. Do not bump the version, tag, or publish unless they asked.

**Run the live suite by hand before every release.** The offline suite proves the SDK sends what we think it sends; only the live suite proves the API accepts it. Run it with real keys, including `ZEROBOUNCE_LIVE_SPEND=1` and `ZEROBOUNCE_LIVE_CHANGES=1`, and do not release on a failure.

1. Move `[Unreleased]` entries under `## [x.y.z] - YYYY-MM-DD`, leave an empty `[Unreleased]` section, and update the compare links at the bottom of `CHANGELOG.md`.
2. Set the same version in `Cargo.toml` `version` and in the `zero-bounce` package entry in `Cargo.lock`. The publish workflow fails if the tag and `Cargo.toml` disagree.
3. `cargo test --no-fail-fast`.
4. Tag `vx.y.z` on that commit and run the Publish workflow with that tag. It publishes to crates.io.
