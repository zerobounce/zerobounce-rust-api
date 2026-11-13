# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `ZeroBounceWarmup::add_zb_managed_sending_account(from_address, ignore_conflict, email_content, managed_sending)`, with `ZbManagedEmailContent` (`company_name`, `company_website`, `company_description`, `types_of_emails`, `example_email`) and `ZbManagedSending` (`weekdays_only`, `timezone`, `timezone_offset` as an `f64` from -11.0 to 14.0, `warmup_start_date`): the `zb_managed` payload as the API documents it. Each field is sent only when set. Both structs are `#[non_exhaustive]`: build them from `Default::default()` and assign fields. A `timezone_offset` outside -11.0 to 14.0, or NaN, is `ZBError::ExplicitError("timezone_offset must be -11.0 to 14.0")` before a request.

### Deprecated

- `EmailContent` and `ManagedSending`, the `zb_managed` arguments of `add_sending_account`. They cannot carry `company_description`, `example_email`, `weekdays_only`, `timezone` or a fractional `timezone_offset` such as 5.5, and adding fields to them would break struct literals, so new structs replace them. `add_sending_account` itself is not deprecated (it is the `self_managed` call) and still accepts them. Upgrading: use `add_zb_managed_sending_account` with `ZbManagedEmailContent` and `ZbManagedSending`.

### Fixed

- The bulk downloads (`bulk_validation_result_fetch[_with_options]`, `ai_scoring_result_fetch[_with_options]`, `email_finder_result_fetch`, `domain_search_result_fetch`) returned an empty HTTP 200 body (or one of only whitespace) as `Ok(ZBBulkResponse::Content)` with no bytes, whatever its content type. It is now `ZBError::ApiError` with status 200 and `Empty response from ZeroBounce (HTTP 200)`. Affects 3.0.0 to 4.0.1.
- A 2xx JSON call whose body starts with `{` or `[` but does not parse (`{"success": tru`) returned `ZBError::JsonError` with no status. It is now `ZBError::ApiError` with that status and `ZeroBounce returned a response that is not valid JSON.`, like other non-JSON 2xx bodies. A body that parses but does not fit the response type is covered below. Affects 3.0.0 to 4.0.1.
- `add_sending_account` with `OnboardingModel::ZbManaged` sent `email_content.website`, `email_content.industry` and `managed_sending.daily_limit`, none of which the API reads, so the website was lost. `EmailContent::website` is now sent as `company_website`, and `industry` and `daily_limit` are no longer sent (the API does not apply a daily limit). Affects 3.0.0 to 4.0.1.
- `ZeroBounceWarmup::download_seeds` and `download_statistics` returned an HTML page (a `text/html` 200, or an HTML body under any content type) and a JSON body sent with a non-JSON content type as the CSV text. They now return `ZBError::ApiError` with the status (200), like an empty body or a JSON content type already did. Affects 3.0.0 to 4.0.1.
- The `ZeroBounceWarmup` JSON calls returned a `text/html` 2xx whose body parsed as JSON (`{"a":1}`) as the API's answer. It is now `ZBError::ApiError` with that status and `ZeroBounce returned an HTML error page instead of an API response.`, like every `ZeroBounce` call. Affects 3.0.0 to 4.0.1.
- `delete_inbox_placement_test` answered with an empty 2xx (`204 No Content`) now returns `Ok(Value::Null)`. It returned `ZBError::ApiError` with status 204 and `Empty response from ZeroBounce (HTTP 204)`, although the test was deleted. The Go SDK already treats it as a success. The other Inbox Placement calls must return data, so an empty 2xx on them is still an error. Affects 3.0.0 to 4.0.1. A body of only whitespace is not empty: it stays `ZBError::ApiError` with its status, like on the other calls.
- The 2xx JSON check trimmed the body with `str::trim`, which also drops Unicode spaces, before parsing it, so JSON padded with U+00A0, U+2003 or a vertical tab passed the check and then failed to parse as `ZBError::JsonError` with no status. Every JSON call now parses the body as received, as strict RFC 8259 JSON (only space, tab, CR and LF around the value; no BOM, comment, trailing comma or trailing value, `NaN` or other extension), and such a body is `ZBError::ApiError` with that status and `ZeroBounce returned a response that is not valid JSON.`. Affects 3.0.0 to 4.0.1.
- A 2xx body of only whitespace (`"   "`) was `Ok(None)` on every `ZeroBounceWarmup` JSON call. It is not JSON, so it is now `ZBError::ApiError` with that status. A zero-length body (and a 204) is still the documented `Ok(None)`. Affects 3.0.0 to 4.0.1.
- A 2xx body that is valid JSON but not an object or list (`"x"`, `0`) was refused as text, `ZBError::ApiError` with the body as the message. It is now taken as JSON: the `ZeroBounceWarmup` calls and the raw-JSON calls return it. Affects 3.0.0 to 4.0.1.
- A 2xx body that is valid JSON but the wrong shape for the call (`0`, `"x"`, `null`, `[1]`, a field of the wrong type, `{}` without `Credits`) returned `ZBError::JsonError`, `ExplicitError("ZeroBounce returned a response that is not valid JSON.")`, `ExplicitError("credits value not in json")` or `IntParseError` depending on the call, none with a status. Every typed call now returns `ZBError::ApiError` with status 200 and `ZeroBounce returned JSON that is not the expected response.` (`http_error::UNEXPECTED_JSON_SHAPE_MESSAGE`). `JsonError` and `IntParseError` are kept but no SDK call returns them now. Affects 3.0.0 to 4.0.1.
- A response over a size cap (16 MiB for JSON, 256 MiB for a bulk `getfile` download) returned `ZBError::ExplicitError` without a status. It is now `ZBError::ApiError` with the response's status (a 500 stays 500, a 200 is status 200) and the same message, on both clients. Affects 3.0.0 to 4.0.1.
- `validate_email`, `validate_email_and_ip` and `validate_email_with_options` returned a 200 without `address` or `status` (`{}`, `{"address":"a@b.c"}`) as `Ok` with that field set to `""`, and `batch_validate[_with_options]` did the same for such a row. It is now `ZBError::ApiError` with status 200 and `ZeroBounce returned JSON that is not the expected response.`, like a `null` or a non-string there already was. The greylist answer `{"Message": "..."}` is still `Ok`, with `address` and `status` empty. Affects 3.0.0 to 4.0.1.

### Security

- An error built from a response body no longer carries the API key. A body that echoed the key (`{"error": "Invalid API key <key>"}` or `{"success": false, "message": "... <key>"}` on an HTTP 200, a 400 quoting the request URL with `?api_key=<key>`, a JSON error on a 4xx/5xx, a bulk getfile error, `batch_validate`'s error for address `all`) was passed on as the `ZBError::ApiError` message by every `ZeroBounce` request method. The key is now masked as `***` in such a message, as sent, percent-encoded and JSON-escaped, and so is any `api_key=` value. `ZeroBounceWarmup` masked only the key as sent, and not at all in the error for a JSON body in place of a CSV download (`download_seeds`, `download_statistics`); it now masks every form there too. Affects 3.0.0 to 4.0.1.
- Redaction is now closed under encoding. After the known spellings of a secret are masked, the rest of an error message is decoded up to four layers deep in every order (the SDK's own request encoding plus three echo layers) (percent-decoding, percent-decoding with `+` as a space, JSON-unescaping including `\/` and `\uXXXX`); if any decoded view still holds the API key, the Warmup key or the SMTP username or password, the whole message is replaced with `ZeroBounce's response was omitted from this error because it contained a credential.` A body that echoed a secret in a spelling the masking did not generate (lowercase percent escapes, a URL encoding of a URL encoding, a fourth layer) used to put it in `ZBError::ApiError`, on every call of both clients.
- Secrets (the API key, and the SMTP password of `test_smtp_configuration`) are now also masked as the query and strict percent-encoding of each JSON-escaped form (with and without `\/`), which is how a server echoes a URL-encoded JSON request body, and as each JSON-escaped form escaped again (a JSON error body quoting the JSON request body). Longer forms are masked first, so no fragment of a secret is left behind.
- `ZBError::JsonError` no longer carries response-body content. When a 2xx body had a field of the wrong type (`get_api_usage`, `get_activity_data`, `score_email`, `score_email_post`, `find_email`, `domain_search`, `domain_search_v2`), serde's error quoted the value (`invalid type: string "<value>", expected u64`), so a body that echoed the API key there put it in the error's `Debug` output and in the `serde_json::Error` it carries. The inner error now says only `invalid JSON`, `unexpected JSON data`, `unexpected end of JSON` or `I/O error` with the line and column (`line()` and `column()` are kept); its `classify()` is now always `Category::Data`. The `Display` text (`ZeroBounce returned a response that is not valid JSON.`) is unchanged. Affects 3.0.0 to 4.0.1.

## [4.0.1] - 2026-11-11

### Changed

- If the HTTP client cannot be built (for example, the TLS backend fails to initialise), `ZeroBounce::with_base_url`, `ZeroBounceWarmup::new` and `ZeroBounceWarmup::with_base_url` return `ZBError::ExplicitError("could not build the HTTP client: ...")`, and `ZeroBounce::new` panics with that message. 3.0.0 and 4.0.0 fell back to `reqwest::blocking::Client::default()`, which follows redirects and itself panics when the same client cannot be built. No client that follows redirects is used any more.

### Fixed

- A response body with a multi-byte UTF-8 character spanning byte 2000 panicked with `end byte index 2000 is not a char boundary` instead of returning a result or an error. Every JSON response (`validate_email`, credits, bulk status and the rest), every bulk file download (`bulk_validation_result_fetch` and the other getfile calls) and every `ZeroBounceWarmup` error body goes through the HTML check that sliced at that byte. It now cuts at a character boundary. Affects 3.0.0 and 4.0.0.

## [4.0.0] - 2026-11-12

### Breaking

- A main-API key with surrounding whitespace or control characters is rejected before a request is sent, with `ZBError::ExplicitError("api_key has surrounding whitespace or control characters")`, as `ZeroBounceWarmup` already did. 3.0.0 sent it. Upgrading: trim a key read from a file or the environment (a trailing newline is the usual cause).
- `zero_bounce::utility::mock_constants` is no longer public. It held the crate's own test fixtures and is now compiled only for its tests. Upgrading: copy the constants you used into your own tests.

### Added

- `Debug` for `ZeroBounce`, naming the type and every field, with the API key shown as `***`.

### Changed

- The crate is published with `license = "MIT"` instead of `license-file`, with `keywords` and `categories`, and ships only `src`, `README.md`, `LICENSE` and `CHANGELOG.md`. 3.0.0 also shipped the tests (the live suite included), examples, CI workflows, `Dockerfile`, `.env.example` and the contributor files.

### Fixed

- Requests went to `.../v2//<endpoint>`: the built-in base URLs end in `/v2/` and every endpoint starts with `/`. `ZBUrlProvider::url_of`, `bulk_url_of`, `list_evaluator_url_of` and `inbox_placement_url_of`, and the list evaluator calls, now drop trailing slashes from the base URL, whether it is built in (`ZeroBounce::new`, every `ApiBaseUrl`) or your own, so a request goes to `.../v2/validate`. The stored `base_url` and `url_provider` values are unchanged.
- A 3xx answer to the main client is `ZBError::ApiError` with the 3xx status and the message `ZeroBounce returned a redirect, which this client does not follow.`, the same as `ZeroBounceWarmup`. 3.0.0 refused the redirect as well but reported it as `Empty response from ZeroBounce (HTTP 302)` or as an HTML error page.

### Security

- `ZeroBounceWarmup`'s `Debug` output shows the API key as `***`. 3.0.0 derived `Debug` and printed the key.

## [3.0.0] - 2026-11-11

### Breaking

Upgrading from 2.x: each change below can stop 2.1.3 code from compiling, or change what it gets back, and says what to do.

- `ZeroBounce::with_base_url` returns `ZBResult<ZeroBounce>` instead of `ZeroBounce`. It returns an error, rather than accepting the URL as 2.1.3 did, when the base URL is not `https://`; `http://` is accepted only for the exact hosts `127.0.0.1`, `localhost` and `[::1]`, for local tests. Add `?` (or handle the error) at each call. `ZeroBounce::new` is unchanged.
- Every request applies the same rule to its final URL, including URLs built from `url_provider` fields you set yourself. A plain `http://` URL on any other host fails with `ZBError::ExplicitError` before a connection is opened; 2.1.3 sent such requests, API key included, over plain http.
- `ZBError` has a new variant, `ApiError { status, message }`, and is now `#[non_exhaustive]`. An exhaustive `match` on `ZBError` no longer compiles (E0004): add a `_` arm. A variant added later will not be a breaking change.
- A failed API response is `ZBError::ApiError`, where 2.1.3 returned `ExplicitError` with the raw body: a non-2xx status, an HTML error page, an error body on an HTTP 200 (`{"success": false}`, or the `{"error": ...}` an invalid or drained key gets), a bulk `getfile` error, and the `batch_validate` account-level error. Match `ApiError`, or read `ZBError::status_code()`, where you matched `ExplicitError` for these.
- `validate_email`, `validate_email_and_ip`, `batch_validate`, and the bulk validation and AI scoring submit, status and delete calls report a 2xx body they cannot parse as `ExplicitError("ZeroBounce returned a response that is not valid JSON.")`; 2.1.3 returned `ZBError::JsonError` with serde's message.
- `batch_validate` returns `Err` for an account-level failure (an error for the address `all`, such as an invalid key or no credits). 2.1.3 returned `Ok`, with the failure in `errors`. An error for one address still comes back on the `Ok` result.
- `ZBValidateSubStatus` has a new variant, `AiAgentMailbox`, and both `ZBValidateSubStatus` and `ZBValidateStatus` are now `#[non_exhaustive]`: an exhaustive `match` needs a `_` arm. The sub-status `"ai_agent_mailbox"` parsed to `UnknownValue("ai_agent_mailbox")` in 2.1.3 and parses to `AiAgentMailbox` now, so code that matched the `UnknownValue` form must match the new variant.
- `ApiBaseUrl` is now `#[non_exhaustive]`: a `match` on it needs a `_` arm, so a region added later is not a breaking change.
- `ZBUrlProvider` has two new public fields, `list_evaluator_url` and `inbox_placement_url`, so a 2.1.3 struct literal fails with E0063. It is also now `#[non_exhaustive]`, so it cannot be built with a struct literal outside the crate at all: start from `ZBUrlProvider::default()` and assign the fields you change. A base URL added later will not be a breaking change. It now derives `Clone` and `Debug`.
- `ZBValidation` has new public fields (`active_in_days`, `active_first_seen`, `domain_website_exists`, `domain_registrant_company_name`, `message`), and `ApiUsage` has new counters (`sub_status_leading_period_removed`, `sub_status_alias_address`). Both are now `#[non_exhaustive]`, so neither can be built with a struct literal outside the crate; build them with `serde_json` in tests. A field added later will not be a breaking change.
- Responses have size limits: a JSON response over 16 MiB fails with "response body exceeds size limit", and a bulk `getfile` download over 256 MiB fails with "getfile response exceeds size limit". Reading stops one byte past the limit. 2.1.3 read a response of any size.

### Added

- `validate_email_with_options` and `ZBValidateOptions`: `timeout` (3–60 seconds), `activity_data`, `verify_plus`, `domain_info`, `greylist_processing`, `callback_url`, and `use_post` (form-encoded `POST /v2/validate`). `callback_url` needs `greylist_processing: Some(true)` and must not be blank; both are checked before a request is sent.
- `batch_validate_with_options`, taking the same `ZBValidateOptions` (`timeout` 10–120 seconds, `activity_data`, `verify_plus`). `timeout` is sent as a JSON string: sent as a number, the live API rejects the whole batch with an account-level error.
- `ZBValidation` fields: `active_in_days`, `active_first_seen`, `domain_website_exists` and `domain_registrant_company_name` (with `domain_info`), and `message`, read from the API's `Message` key. A repeated `greylist_processing` validate answers with nothing but `{"Message": "Greylist validation still in progress. Please try again later."}`, so `address`, `status`, `sub_status`, `free_email` and `processed_at` default when absent and that answer is a `ZBValidation` whose `message` says to try again.
- `score_email` (`GET /v2/scoring`) and `score_email_post` (form-encoded `POST /v2/scoring`).
- Bulk email finder and bulk domain search files, on the bulk host without the `/v2` path: `send_email_finder_file`, `email_finder_file_status`, `email_finder_result_fetch`, `email_finder_result_delete`, `send_domain_search_file`, `domain_search_file_status`, `domain_search_result_fetch` and `domain_search_result_delete`. `send_email_finder_file` takes a `ZBEmailFinderFileOptions` (`first_name_column`, `full_name_column`, `middle_name_column`, `last_name_column`, `has_header_row`) and requires one of `first_name_column` or `full_name_column`, because the API rejects a file that carries neither. `send_domain_search_file` takes `has_header_row` and omits it when `None`, so the account default applies. The `getfile` and `deletefile` calls follow the same rules as `bulk_validation_result_fetch` and `bulk_validation_result_delete`.
- List evaluator: `send_list_evaluator_file` and `get_list_evaluator_result`, on `ZBUrlProvider::list_evaluator_url` (the file id is URL-encoded as one path segment), and `send_evaluated_file(file_id)` on the bulk base. `send_evaluated_file` posts `file_id` as a form field, because the endpoint rejects the file upload its docs describe.
- Filters: `add_filter`, `list_filters` and `delete_filter`.
- Amazon S3 bulk: `list_s3_files`, `validate_s3_file`, `score_s3_file`, `email_finder_s3_file`, `domain_search_s3_file` and `export_s3_file`. The four jobs take `file_name`, `has_header_row`, `remove_duplicates` and `return_url`; `file_name` and `return_url` are required `&str`s, because the API answers "This field is required" without them. `export_s3_file` takes `export_bucket` as an `Option<&str>`: `None` uses the export bucket from your ZeroBounce settings, and `Some("")` is rejected before a request.
- SFTP/FTP bulk: `validate_sftp_file`, `score_sftp_file`, `email_finder_sftp_file`, `domain_search_sftp_file` and `export_sftp_file`. The server, path and protocol come from your ZeroBounce settings. The jobs take the same arguments as the S3 ones; `validate_sftp_file` requires `file_name` too, although the docs list it as optional for that endpoint, because the API rejects a request without it. `export_sftp_file` takes `export_path` rather than the `export_bucket` `export_s3_file` uses.
- Inbox Placement Tester on `https://bulkapi.zerobounce.net/api/ipt`: `get_inbox_placement_providers`, `get_inbox_placement_test_types`, `create_inbox_placement_test`, `get_inbox_placement_test`, `list_inbox_placement_tests`, `delete_inbox_placement_test`, `get_inbox_placement_webhook` and `set_inbox_placement_webhook`. It sends the key in an `Authorization: Bearer` header, never in a query string, and an error body's `detail` text becomes the error message.
- `ZBUrlProvider::list_evaluator_url` and `inbox_placement_url`, both overridable, with `list_evaluator_url_of` and `inbox_placement_url_of`.
- `ZeroBounceWarmup`, a client for the Warmup API. It is a separate product with its own host (`https://warmup-api.zerobounce.net`), its own team-scoped key sent in an `API-Key` header, and all 26 documented endpoints, including the seed-list and statistics CSV downloads. It rejects a key with surrounding whitespace or control characters, which the HTTP library would otherwise quote unredacted in its own error, and `test_smtp_configuration` masks the username and password out of every error it raises.
- `ZBError::ApiError { status, message }` and `ZBError::status_code()`, so callers can tell a Cloudflare 403 from an API 400/404 and a Warmup 429. `status_code()` is `Some` for every failed API response, including bulk `getfile` errors and the failures the API reports with an HTTP 200, such as the `batch_validate` account-level error. It is `None` for an argument or URL rejected before a request, a transport error, and a 2xx body that could not be parsed.
- `Display` and `std::error::Error` for `ZBError`, so it works with `?` into `Box<dyn Error>` and with error-handling crates. 2.1.3 implemented only `Debug`.
- An opt-in live API suite behind the `live` cargo feature. `cargo test` and CI never compile it; running it takes `cargo test --features live --test live_api` with keys in the environment, and it only spends credits or changes the account behind `ZEROBOUNCE_LIVE_SPEND=1` and `ZEROBOUNCE_LIVE_CHANGES=1`.

### Changed

- `find_email_v2` no longer requires `first_name`. The API treats it as optional and answers with the domain's address formats without it. An explicitly empty `first_name` is still rejected, and `domain_search_v2` is unchanged.
- The client that `ZeroBounce::new` and `ZeroBounce::with_base_url` build times out after 120 seconds and does not follow redirects. 2.1.3 used reqwest's default blocking client, which times out after 30 seconds and follows up to 10 redirects. The `client` field is public: a client you build and assign yourself keeps the settings you give it.
- An empty API key is rejected before a request is sent.
- The crate declares `rust-version = "1.88"`, the oldest Rust its current dependencies build with: recent `encoding_rs` and `icu_*` releases, pulled in through `reqwest`, require 1.88. Builds and tests pass on 1.88.0, and Cargo refuses 1.87.0 with "rustc 1.87.0 is not supported by the following packages".

### Fixed

- An invalid or drained API key reported a misleading parse error, or no error at all. The API answers one with an HTTP 200 and no `success` field: `{"error": "Invalid API key or your account ran out of credits"}` from `validate_email` and `get_activity_data`, `{"error": "Invalid API key"}` from `get_api_usage`, and `{"Message": ...}` from the email finder and domain search. 2.1.3 passed these to the deserializer, which failed on the unexpected shape with a `ZBError::JsonError` naming a missing field (for example missing field `address` from `validate_email`), and `domain_search_v2` returned an empty `DomainSearchResponseV2`. Each is now a `ZBError::ApiError` carrying the API's message; the filters, new in this release, get the same treatment for their `{"Error": ...}` answer. `message`, `Message` and `detail` decide failure only for the email finder and domain search, because successful responses use them too: a bulk submit answers `{"success": true, "message": "File Accepted"}`, and a filter change answers with a bare `Message`. `get_credits` still returns `-1` for an invalid key, as documented.
- `get_api_usage` no longer fails outright. The API returns only the counters that apply to the account and the date range asked for, and an ordinary response carries neither `sub_status_gold` nor `sub_status_role_based_accept_all`; 2.1.3 required both, so the whole call failed with a `ZBError::JsonError` naming the missing field. Every counter on `ApiUsage` now defaults to 0 when absent, so a counter the API adds or drops later is no longer fatal. `sub_status_leading_period_removed` and `sub_status_alias_address`, which the API does return, are now read.
- `{"success": false}` on an HTTP 200 is an error instead of a result. The API answers an unknown bulk file id that way; 2.1.3 returned it from `deletefile` as an `Ok` `ZBFileFeedback` with `success: false`, and from `filestatus` as a `JsonError`.
- An HTML "Access denied" page is returned as a short error. An HTML page with status 200 is an error as well, including on bulk `getfile`, where 2.1.3 returned the page as file bytes.
- Empty and non-JSON bodies are a short error. The message is no longer the raw body or a serde parse dump, and a repeated `Error:` / `[Error]:` prefix is removed.
- AI scoring `sendfile` no longer sends the name, gender, or IP columns. (`allow_phase_2` was already sent for validation `sendfile` only.)

### Security

- Plain `http://` is refused on every request path, including bulk file submit, bulk `getfile`, the finder, domain search and list evaluator uploads, and the Inbox Placement Tester, so an `http://` override on `url_provider` can no longer send the API key in cleartext. See Breaking.
- Redirects are disabled, so a 3xx response cannot send the API key to another host.
- The API key is replaced with `REDACTED` in the URL a `ZBError::RequestError` carries, so printing a transport error no longer prints the key.
- Debug builds no longer print every raw API response to stderr. 2.1.3 did, with `eprintln!`.
- Error values no longer include the raw HTML access-denied page.
- JSON and bulk `getfile` responses are read only up to their size limits, so an oversized body cannot exhaust memory. See Breaking.

## [2.1.3] - 2026-08-14

No library code changes from 2.1.2; only the crate version changed.

### Changed

- README: Docker test instructions run from the `sdk-docs/` folder of the SDKs monorepo, and the Publish section lists the release steps (bump `Cargo.toml`, tag, run the Publish workflow) instead of linking to the sdk-docs crates.io guide.
- CI: `actions/checkout` v7 and CodeQL actions 4.36.3.

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

[Unreleased]: https://github.com/zerobounce/zerobounce-rust-api/compare/v4.0.1...HEAD
[4.0.1]: https://github.com/zerobounce/zerobounce-rust-api/compare/v4.0.0...v4.0.1
[4.0.0]: https://github.com/zerobounce/zerobounce-rust-api/compare/v3.0.0...v4.0.0
[3.0.0]: https://github.com/zerobounce/zerobounce-rust-api/compare/v2.1.3...v3.0.0
[2.1.3]: https://github.com/zerobounce/zerobounce-rust-api/compare/v2.1.2...v2.1.3
[2.1.2]: https://github.com/zerobounce/zerobounce-rust-api/compare/v2.1.0...v2.1.2
[2.1.0]: https://github.com/zerobounce/zerobounce-rust-api/compare/v2.0.11...v2.1.0
[2.0.11]: https://github.com/zerobounce/zerobounce-rust-api/compare/v2.0.0...v2.0.11
[2.0.0]: https://github.com/zerobounce/zerobounce-rust-api/compare/v1.3.0...v2.0.0
[1.3.0]: https://github.com/zerobounce/zerobounce-rust-api/compare/v1.1.0...v1.3.0
[1.1.0]: https://github.com/zerobounce/zerobounce-rust-api/releases/tag/v1.1.0
