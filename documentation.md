#### How to use
This crate uses the ZeroBounce API which requires an API key. Check [this guide](https://www.zerobounce.net/docs/api-dashboard#API_keys_management) to see how to grab yours.

Check the [example snippets](https://github.com/zerobounce/zerobounce-rust-api/tree/main/examples) to see how this library can be integrated in your own project.

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
