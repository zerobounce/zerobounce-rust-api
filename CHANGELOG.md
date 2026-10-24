# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `ZBValidateOptions` for single validation: `timeout`, `activity_data`, `verify_plus`, and `use_post` (form-encoded `POST /v2/validate`). The same options apply to `batch_validate_with_options`.
- `score_email` (`GET /v2/scoring`) and `score_email_post` (form-encoded `POST /v2/scoring`).
- Bulk email finder, bulk domain search, list evaluator, filters, and S3 bulk (`validate_s3_file`, `score_s3_file`, `export_s3_file`, `list_s3_files`, `email_finder_s3_file`, `domain_search_s3_file`).
- SFTP/FTP bulk: `validate_sftp_file`, `score_sftp_file`, `email_finder_sftp_file`, `domain_search_sftp_file`, and `export_sftp_file`. The server, path and protocol come from your ZeroBounce settings. `validate_sftp_file` takes `file_name` as `Option` because the docs list it as optional there alone; `export_sftp_file` takes `export_path` rather than the `export_bucket` `export_s3_file` uses.
- `ZBValidateOptions` accepts `domain_info`, `greylist_processing`, and `callback_url`. `callback_url` needs `greylist_processing: Some(true)` and is rejected without it, and when blank, before a request is sent.
- `ZBValidation` carries `domain_website_exists`, `domain_registrant_company_name`, and `message`. `message` is read from the API's `Message` key, returned while greylist processing is still running.
- Inbox Placement Tester on `https://bulkapi.zerobounce.net/api/ipt`: `get_inbox_placement_providers`, `get_inbox_placement_test_types`, `create_inbox_placement_test`, `get_inbox_placement_test`, `list_inbox_placement_tests`, `delete_inbox_placement_test`, `get_inbox_placement_webhook`, `set_inbox_placement_webhook`.
- `ZBUrlProvider` gains `list_evaluator_url` and `inbox_placement_url`, both overridable.
- `ZeroBounceWarmup`, a client for the Warmup API. It is a separate product with its own host (`https://warmup-api.zerobounce.net`), its own team-scoped key sent in an `API-Key` header, and all 26 documented endpoints, including the seed-list and statistics CSV downloads.
- `ZBError::ApiError { status, message }` and `ZBError::status_code()`, so callers can tell a Cloudflare 403 from an API 400/404 and a Warmup 429.
- An opt-in live API suite behind the `live` cargo feature. `cargo test` and CI never compile it; running it takes `cargo test --features live --test live_api` with keys in the environment, and it only spends credits or changes the account behind `ZEROBOUNCE_LIVE_SPEND=1` and `ZEROBOUNCE_LIVE_CHANGES=1`.

### Changed

- List evaluator requests use `ZBUrlProvider::list_evaluator_url` instead of building the path from the bulk base, and the file id is URL-encoded as one path segment. `send_evaluated_file` stays on the bulk base.

- API requests time out after 120 seconds and do not follow redirects.
- `batch_validate` sends `timeout` as a JSON string. Sent as a number, the live API rejects the whole batch with an account-level error.
- The Inbox Placement Tester authenticates with an `Authorization: Bearer` header instead of an `api_key` parameter, so the key no longer reaches a query string that gets logged.
- `send_evaluated_file(file_id)` posts the list evaluator's `file_id` as a form field. The endpoint rejects a file upload with "file_id: This field is required", so it no longer takes a file.
- The S3 and SFTP/FTP jobs require `file_name` and `return_url`, `validate_sftp_file` included, so both are now `&str` rather than `Option`. The API answers "This field is required" otherwise.
- A base URL must be `https://`, except `http://127.0.0.1`, `http://localhost`, and `http://[::1]` for tests.

### Fixed

- `{"success": false}` on an HTTP 200 is an error instead of a result. The API answers an unknown bulk file id that way.
- An error body whose JSON carries a `detail` string (the Inbox Placement Tester, Django REST framework) returns that text.

- An HTML "Access denied" page is returned as a short `ZBError`. An HTML page with status 200 is an error as well, including bulk `getfile`, instead of file bytes or a JSON parse.
- Empty and non-JSON bodies are a short error. The message is no longer a serde parse dump, and a repeated `Error:` / `[Error]:` prefix is removed.
- `batch_validate` returns an error for an account-level failure (HTTP 200, error for address `all`, such as an invalid API key or no credits). An error for one address stays on the successful result.
- AI scoring `sendfile` does not send name, gender, or IP columns, or `allow_phase_2`.
- An empty API key is rejected before a request is sent.
- `validate` timeout must be 3–60 seconds. `batch_validate` timeout must be 10–120 seconds.

### Security

- Redirects are disabled, so a 3xx response cannot send the API key to another host.
- Error values no longer include the raw HTML access-denied page.
- A Warmup API key with surrounding whitespace or control characters is rejected, because the transport would otherwise quote it unredacted in its own error.
- The Warmup SMTP test masks the username and password out of every error it raises.

## [2.1.2] - 2026-06-10

### Added

- `catchall_domain` on the validation response.

## [2.1.0] - 2026-04-05

### Added

- `ZBGetFileOptions` with `download_type` and `activity_data` (validation getfile only).

### Fixed

- Bulk getfile treats a JSON error body, including HTTP 200, as a failure instead of file content.

## [2.0.11] - 2026-03-03

### Changed

- License the project under MIT.

## [2.0.0] - 2026-03-03

### Changed

- Release the 2.0 line of the client.

## [1.3.0] - 2025-11-10

### Added

- `accept_all` status.
- Base URL selection (`ApiBaseUrl::Default`, `USA`, and `EU`).

### Deprecated

- `domain_search`. Use `domain_search_v2`.

## [1.1.0] - 2023-09-04

### Added

- Initial client for validation, credits, activity, and bulk file calls.

[Unreleased]: https://github.com/zerobounce/zerobounce-rust-api/compare/v2.1.3...HEAD
[2.1.2]: https://github.com/zerobounce/zerobounce-rust-api/compare/v2.1.0...v2.1.2
[2.1.0]: https://github.com/zerobounce/zerobounce-rust-api/compare/v2.0.11...v2.1.0
[2.0.11]: https://github.com/zerobounce/zerobounce-rust-api/compare/v2.0.0...v2.0.11
[2.0.0]: https://github.com/zerobounce/zerobounce-rust-api/compare/v1.3.0...v2.0.0
[1.3.0]: https://github.com/zerobounce/zerobounce-rust-api/compare/v1.1.0...v1.3.0
[1.1.0]: https://github.com/zerobounce/zerobounce-rust-api/releases/tag/v1.1.0
