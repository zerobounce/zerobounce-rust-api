# Rust ZeroBounce API

This library is a wrapper for the ZeroBounce API v2.

For more information about the API, visit https://www.zerobounce.net/docs/.

## How to use
This crate uses the zero-bounce API which requires an API key. Check [this guide](https://www.zerobounce.net/docs/api-dashboard#API_keys_management) to see how to grab yours.

Check the [example snippets](https://github.com/zerobounce/zerobounce-rust-api/tree/main/examples) to see how this library can be integrated in your own project.

## Configuration Options

The `ZeroBounce` client can be configured with different base URLs depending on your region or requirements.

### Default Configuration

The simplest way to create a client is using the default API URL:

```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("your_api_key");
```

### Enum Values

You can specify a region using the `ApiBaseUrl` enum:

```rust
use zero_bounce::{ZeroBounce, ApiBaseUrl};

// USA region
let zb = ZeroBounce::with_base_url("your_api_key", ApiBaseUrl::USA);

// EU region
let zb = ZeroBounce::with_base_url("your_api_key", ApiBaseUrl::EU);

// Default (explicit)
let zb = ZeroBounce::with_base_url("your_api_key", ApiBaseUrl::Default);
```

### Custom URL String

You can also provide a custom base URL as a string:

```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::with_base_url("your_api_key", "https://custom-api.example.com/v2/");
```

**Available API Base URLs:**
- `ApiBaseUrl::Default` - `https://api.zerobounce.net/v2/` (default)
- `ApiBaseUrl::USA` - `https://api-us.zerobounce.net/v2/`
- `ApiBaseUrl::EU` - `https://api-eu.zerobounce.net/v2/`

See the [config_options example](https://github.com/zerobounce/zerobounce-rust-api/tree/main/examples/config_options.rs) for a complete demonstration of all configuration options.

## Email Finding Methods

### find_email_v2 (Recommended)

Find an email address using either a domain or company name. Uses the builder pattern for ergonomic API calls.

**Requirements:**
- `first_name` is mandatory
- Exactly one of `domain` or `company_name` must be provided (XOR requirement)

**Builder Methods:**
- `.first_name(name: &str)` - Set the first name (mandatory)
- `.domain(domain: &str)` - Set the domain name
- `.company_name(company: &str)` - Set the company name
- `.middle_name(name: &str)` - Set the middle name (optional)
- `.last_name(name: &str)` - Set the last name (optional)
- `.call()` - Execute the API call

**Example:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("your_api_key");

// Using domain
let result = zb.find_email_v2()
    .first_name("John")
    .domain("example.com")
    .last_name("Doe")
    .call()?;

// Using company name
let result = zb.find_email_v2()
    .first_name("John")
    .company_name("Example Inc")
    .last_name("Doe")
    .call()?;

// With middle name
let result = zb.find_email_v2()
    .first_name("John")
    .domain("example.com")
    .middle_name("Middle")
    .last_name("Doe")
    .call()?;
```

**Returns:** `FindEmailResponseV2` containing:
- `email`: The found email address
- `domain`: Domain name
- `confidence`: Email confidence level (from `email_confidence` field)
- `company_name`: Company name
- `did_you_mean`: Suggested alternative
- `failure_reason`: Reason for failure if any

### find_email (Deprecated)

⚠️ **Deprecated since version 1.2.0** - Use `find_email_v2` instead.

This method is kept for backward compatibility but will be removed in a future version. The new `find_email_v2` method supports both domain and company_name parameters.

**Example:**
```rust
let result = zb.find_email("example.com", "John", "", "Doe")?;
```

## Domain Search Methods

### domain_search_v2 (Recommended)

Search for email formats using either a domain or company name. Uses the builder pattern for ergonomic API calls.

**Requirements:**
- Exactly one of `domain` or `company_name` must be provided (XOR requirement)

**Builder Methods:**
- `.domain(domain: &str)` - Set the domain name
- `.company_name(company: &str)` - Set the company name
- `.call()` - Execute the API call

**Example:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("your_api_key");

// Using domain
let result = zb.domain_search_v2()
    .domain("example.com")
    .call()?;

// Using company name
let result = zb.domain_search_v2()
    .company_name("Example Inc")
    .call()?;
```

**Returns:** `DomainSearchResponseV2` containing:
- `domain`: Domain name
- `company_name`: Company name
- `format`: Email format found
- `confidence`: Confidence level
- `other_domain_formats`: Alternative formats with confidence levels
- `did_you_mean`: Suggested alternative
- `failure_reason`: Reason for failure if any

### domain_search (Deprecated)

⚠️ **Deprecated since version 1.2.0** - Use `domain_search_v2` instead.

This method is kept for backward compatibility but will be removed in a future version. The new `domain_search_v2` method supports both domain and company_name parameters.

**Example:**
```rust
let result = zb.domain_search("example.com")?;
```

## Account & Usage Methods

### get_credits

Get the number of credits remaining in your ZeroBounce account.

**Example:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("your_api_key");
let credits = zb.get_credits()?;
println!("Credits remaining: {}", credits);
```

**Returns:** `i64` - The number of credits remaining in your account

### get_api_usage

Get API usage statistics for a specific date range.

**Arguments:**
- `start_date: NaiveDate` - Start date for the usage period
- `end_date: NaiveDate` - End date for the usage period

**Example:**
```rust
use zero_bounce::ZeroBounce;
use chrono::NaiveDate;

let zb = ZeroBounce::new("your_api_key");
let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
let usage = zb.get_api_usage(start, end)?;
println!("Total API calls: {}", usage.total);
```

**Returns:** `ApiUsage` containing:
- `total`: Total number of API calls in the period
- `status_valid`: Number of valid email addresses
- `status_invalid`: Number of invalid email addresses
- `status_catch_all`: Number of catch-all email addresses
- `status_do_not_mail`: Number of do-not-mail addresses
- `status_spamtrap`: Number of spamtrap addresses
- `status_abuse`: Number of abuse addresses
- `status_unknown`: Number of unknown addresses
- `sub_status_antispam_system`: Number of antispam system responses
- `sub_status_greylisted`: Number of greylisted responses
- `sub_status_mail_server_temporary_error`: Number of temporary mail server errors
- `sub_status_forcible_disconnect`: Number of forcible disconnects
- `sub_status_mail_server_did_not_respond`: Number of non-responsive mail servers
- `sub_status_timeout_exceeded`: Number of timeout errors
- `sub_status_failed_smtp_connection`: Number of failed SMTP connections
- `sub_status_mailbox_quota_exceeded`: Number of mailbox quota exceeded errors
- `sub_status_exception_occurred`: Number of exceptions
- `sub_status_possible_trap`: Number of possible traps
- `sub_status_role_based`: Number of role-based addresses
- `sub_status_global_suppression`: Number of globally suppressed addresses
- `sub_status_mailbox_not_found`: Number of mailbox not found errors
- `sub_status_no_dns_entries`: Number of no DNS entries errors
- `sub_status_failed_syntax_check`: Number of failed syntax checks
- `sub_status_possible_typo`: Number of possible typos
- `sub_status_unroutable_ip_address`: Number of unroutable IP addresses
- `sub_status_leading_period_removed`: Number of leading periods removed
- `sub_status_does_not_accept_mail`: Number of addresses that don't accept mail
- `sub_status_alias_address`: Number of alias addresses
- `sub_status_role_based_catch_all`: Number of role-based catch-all addresses
- `sub_status_accept_all`: Number of accept-all addresses
- `sub_status_disposable`: Number of disposable addresses
- `sub_status_toxic`: Number of toxic addresses

### get_api_usage_overall

Get overall API usage statistics from January 1, 2000 to the current date.

**Example:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("your_api_key");
let overall_usage = zb.get_api_usage_overall()?;
println!("Total API calls overall: {}", overall_usage.total);
```

**Returns:** `ApiUsage` - Same structure as `get_api_usage`

### get_activity_data

Get activity data for a specific email address, including when it was last seen sending emails.

**Arguments:**
- `email: &str` - The email address to check

**Example:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("your_api_key");
let activity = zb.get_activity_data("valid@example.com")?;
println!("Found: {}", activity.found);
if activity.found {
    println!("Active status: {}", activity.active_status);
}
```

**Returns:** `ActivityData` containing:
- `found`: Whether activity data was found for the email
- `active_status`: Active status of the email address
- `active_date`: Date when the email was last active

## Email Validation Methods

### validate_email

Validate a single email address.

**Arguments:**
- `email: &str` - The email address to validate

**Example:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("your_api_key");
let validation = zb.validate_email("valid@example.com")?;
println!("Status: {}", validation.status);
println!("Sub status: {:?}", validation.sub_status);
```

**Returns:** `ZBValidation` containing:
- `address`: The email address that was validated
- `status`: Validation status (`valid`, `invalid`, `catch-all`, `unknown`, `spamtrap`, `abuse`, `do_not_mail`)
- `sub_status`: Sub-status providing more details about the validation result
- `account`: Account name (if available)
- `domain`: Domain name
- `did_you_mean`: Suggested alternative email address
- `domain_age_days`: Age of the domain in days
- `smtp_provider`: SMTP provider name
- `mx_found`: Whether MX records were found
- `mx_record`: MX record value
- `firstname`: First name associated with the email (if available)
- `lastname`: Last name associated with the email (if available)
- `gender`: Gender associated with the email (if available)
- `country`: Country associated with the email (if available)
- `region`: Region associated with the email (if available)
- `city`: City associated with the email (if available)
- `zipcode`: Zipcode associated with the email (if available)
- `processed_at`: Timestamp when the validation was processed

### validate_email_and_ip

Validate an email address with an optional IP address for more accurate validation.

**Arguments:**
- `email: &str` - The email address to validate
- `ip_address: &str` - Optional IP address (can be empty string)

**Example:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("your_api_key");
let validation = zb.validate_email_and_ip("valid@example.com", "99.110.204.1")?;
println!("Status: {}", validation.status);
```

**Returns:** `ZBValidation` - Same structure as `validate_email`

### batch_validate

Validate multiple email addresses in a single API call. More efficient than validating emails one by one.

**Arguments:**
- `emails_and_ip_addresses: Vec<(String, String)>` - Vector of tuples containing (email, ip_address) pairs. IP address can be empty string.

**Example:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("your_api_key");
let emails_and_ips = vec![
    ("valid@example.com".to_string(), "99.110.204.1".to_string()),
    ("invalid@example.com".to_string(), "".to_string()),
];
let batch_result = zb.batch_validate(emails_and_ips)?;
println!("Email batch: {:#?}", batch_result.email_batch);
```

**Returns:** `ZBBatchValidation` containing:
- `email_batch`: Vector of `ZBValidation` results for each email address

## Bulk Validation Methods

Bulk validation allows you to upload a file containing multiple email addresses for validation. The process involves submitting a file, checking its status, fetching results, and optionally deleting the file.

### bulk_validation_file_submit

Submit a file for bulk email validation.

**Arguments:**
- `zb_file: &ZBFile` - A `ZBFile` instance containing the email addresses to validate

**Example:**
```rust
use zero_bounce::{ZeroBounce, ZBFile};

let zb = ZeroBounce::new("your_api_key");
let file_content = vec![
    "email1@example.com".to_string(),
    "email2@example.com".to_string(),
];
let zb_file = ZBFile::from_content(file_content)
    .set_has_header_row(false)
    .set_remove_duplicate(true);
let submit_result = zb.bulk_validation_file_submit(&zb_file)?;
println!("File ID: {:?}", submit_result.file_id);
```

**Returns:** `ZBFileFeedback` containing:
- `success`: Whether the file submission was successful
- `message`: Status message
- `file_id`: Optional file ID to use for status checks and result fetching

### bulk_validation_file_status_check

Check the processing status of a submitted bulk validation file.

**Arguments:**
- `file_id: &str` - The file ID returned from `bulk_validation_file_submit`

**Example:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("your_api_key");
let status = zb.bulk_validation_file_status_check("file_id_here")?;
println!("Complete percentage: {}%", status.complete_percentage);
println!("Success count: {}", status.success_count);
```

**Returns:** `ZBFileStatus` containing:
- `success`: Whether the status check was successful
- `message`: Status message
- `file_id`: The file ID
- `file_name`: Name of the file
- `upload_date`: Date when the file was uploaded
- `file_status`: Current status of the file
- `complete_percentage`: Percentage of processing completed
- `return_url`: URL to fetch results
- `success_count`: Number of successfully processed emails
- `error_count`: Number of errors encountered

### bulk_validation_result_fetch

Fetch the results of a completed bulk validation file.

**Arguments:**
- `file_id: &str` - The file ID returned from `bulk_validation_file_submit`

**Example:**
```rust
use zero_bounce::ZeroBounce;
use zero_bounce::utility::structures::bulk::ZBBulkResponse;

let zb = ZeroBounce::new("your_api_key");
let result = zb.bulk_validation_result_fetch("file_id_here")?;
match result {
    ZBBulkResponse::Content(bytes) => {
        // File content (CSV format)
        println!("Received {} bytes", bytes.len());
    },
    ZBBulkResponse::Feedback(feedback) => {
        // Error or status feedback
        println!("Message: {}", feedback.message);
    },
}
```

**Returns:** `ZBBulkResponse` which can be:
- `ZBBulkResponse::Content(Vec<u8>)` - The file content as bytes (typically CSV format)
- `ZBBulkResponse::Feedback(ZBFileFeedback)` - Error or status feedback if the file is not ready or an error occurred

### bulk_validation_result_delete

Delete a bulk validation result file from the ZeroBounce servers.

**Arguments:**
- `file_id: &str` - The file ID returned from `bulk_validation_file_submit`

**Example:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("your_api_key");
let delete_result = zb.bulk_validation_result_delete("file_id_here")?;
println!("Delete successful: {}", delete_result.success);
```

**Returns:** `ZBFileFeedback` containing:
- `success`: Whether the deletion was successful
- `message`: Status message

## AI Scoring Methods

AI Scoring allows you to upload a file containing email addresses to get AI-powered quality scores. The process is similar to bulk validation: submit a file, check status, fetch results, and optionally delete the file.

### ai_scoring_file_submit

Submit a file for AI scoring analysis.

**Arguments:**
- `zb_file: &ZBFile` - A `ZBFile` instance containing the email addresses to score

**Example:**
```rust
use zero_bounce::{ZeroBounce, ZBFile};

let zb = ZeroBounce::new("your_api_key");
let file_content = vec![
    "email1@example.com".to_string(),
    "email2@example.com".to_string(),
];
let zb_file = ZBFile::from_content(file_content)
    .set_has_header_row(false)
    .set_remove_duplicate(true);
let submit_result = zb.ai_scoring_file_submit(&zb_file)?;
println!("File ID: {:?}", submit_result.file_id);
```

**Returns:** `ZBFileFeedback` - Same structure as `bulk_validation_file_submit`

### ai_scoring_file_status_check

Check the processing status of a submitted AI scoring file.

**Arguments:**
- `file_id: &str` - The file ID returned from `ai_scoring_file_submit`

**Example:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("your_api_key");
let status = zb.ai_scoring_file_status_check("file_id_here")?;
println!("Complete percentage: {}%", status.complete_percentage);
```

**Returns:** `ZBFileStatus` - Same structure as `bulk_validation_file_status_check`

### ai_scoring_result_fetch

Fetch the results of a completed AI scoring file.

**Arguments:**
- `file_id: &str` - The file ID returned from `ai_scoring_file_submit`

**Example:**
```rust
use zero_bounce::ZeroBounce;
use zero_bounce::utility::structures::bulk::ZBBulkResponse;

let zb = ZeroBounce::new("your_api_key");
let result = zb.ai_scoring_result_fetch("file_id_here")?;
match result {
    ZBBulkResponse::Content(bytes) => {
        println!("Received {} bytes", bytes.len());
    },
    ZBBulkResponse::Feedback(feedback) => {
        println!("Message: {}", feedback.message);
    },
}
```

**Returns:** `ZBBulkResponse` - Same structure as `bulk_validation_result_fetch`

### ai_scoring_result_delete

Delete an AI scoring result file from the ZeroBounce servers.

**Arguments:**
- `file_id: &str` - The file ID returned from `ai_scoring_file_submit`

**Example:**
```rust
use zero_bounce::ZeroBounce;

let zb = ZeroBounce::new("your_api_key");
let delete_result = zb.ai_scoring_result_delete("file_id_here")?;
println!("Delete successful: {}", delete_result.success);
```

**Returns:** `ZBFileFeedback` - Same structure as `bulk_validation_result_delete`

## Development

### Run tests with Docker
From the **parent repository root** (the folder that contains all SDKs and `docker-compose.yml`):

```bash
docker compose build rust
docker compose run --rm rust
```

### Local setup and tests
```bash
# install
sudo apt update
sudo apt install cargo rustc rustup
rustup update
rustup default stable
```

```bash
# run tests
cargo test
cargo test find_email_v2
cargo test --release

# output
running 83 tests
test bulk::ai_scoring::test_ai_scoring_delete_not_ok ... ok
test bulk::ai_scoring::test_ai_scoring_delete_invalid_json ... ok
...
test result: ok. 83 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.42s
```

```bash
# run examples
# WARNING: examples use live server, will consume credits
cp .env.example .env
# Edit .env and set ZEROBOUNCE_API_KEY=your_api_key_here
cargo run --example # list of available examples
cargo run --example domain_search_v2
```

```bash
# build
cargo build --release
cargo package 

# publish
cargo login <api-token>
cargo publis --dry-run
cargo publish
```