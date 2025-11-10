use std::env;
use zero_bounce::{ZeroBounce, ApiBaseUrl};
use zero_bounce::utility::ZBResult;

fn main() -> ZBResult<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    // Get API key from environment variable
    let api_key = env::var("ZERO_BOUNCE_API_KEY")
        .expect("ZERO_BOUNCE_API_KEY must be set in .env file");
    
    // Example 1: Using default URL (no parameter)
    println!("=== Example 1: Default URL (no parameter) ===");
    let zb_default = ZeroBounce::new(&api_key);
    println!("Base URL: {}", zb_default.base_url);
    
    let find_email_response = zb_default.find_email_v2()
        .first_name("John")
        .domain("example.com")
        .last_name("Doe")
        .call()?;
    
    println!();
    
    // Example 2: Using enum - USA region
    println!("=== Example 2: Using enum - USA region ===");
    let zb_usa = ZeroBounce::with_base_url(&api_key, ApiBaseUrl::USA);
    println!("Base URL: {}", zb_usa.base_url);
    
    let find_email_response = zb_usa.find_email_v2()
        .first_name("John")
        .domain("example.com")
        .last_name("Doe")
        .call()?;
    
    println!();
    
    // Example 3: Using enum - EU region
    println!("=== Example 3: Using enum - EU region ===");
    let zb_eu = ZeroBounce::with_base_url(&api_key, ApiBaseUrl::EU);
    println!("Base URL: {}", zb_eu.base_url);
    
    let find_email_response = zb_eu.find_email_v2()
        .first_name("John")
        .domain("example.com")
        .last_name("Doe")
        .call()?;
    
    println!();
    
    // Example 4: Using direct string
    println!("=== Example 4: Using direct string ===");
    let custom_url = "https://api.zerobounce.net/v2/";
    let zb_custom = ZeroBounce::with_base_url(&api_key, custom_url);
    println!("Base URL: {}", zb_custom.base_url);
    
    let find_email_response = zb_custom.find_email_v2()
        .first_name("John")
        .domain("example.com")
        .last_name("Doe")
        .call()?;
    
    println!();
    
    // Example 5: Using enum - Default (explicit)
    println!("=== Example 5: Using enum - Default (explicit) ===");
    let zb_default_explicit = ZeroBounce::with_base_url(&api_key, ApiBaseUrl::Default);
    println!("Base URL: {}", zb_default_explicit.base_url);
    
    let find_email_response = zb_default_explicit.find_email_v2()
        .first_name("John")
        .domain("example.com")
        .last_name("Doe")
        .call()?;
    
    
    Ok(())
}

