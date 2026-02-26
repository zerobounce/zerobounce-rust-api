use std::env;
use zero_bounce::ZeroBounce;
use zero_bounce::utility::ZBResult;

fn main() -> ZBResult<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    // Get API key from environment variable (standard: ZEROBOUNCE_API_KEY; legacy: ZERO_BOUNCE_API_KEY)
    let api_key = env::var("ZEROBOUNCE_API_KEY")
        .or_else(|_| env::var("ZERO_BOUNCE_API_KEY"))
        .expect("ZEROBOUNCE_API_KEY (or ZERO_BOUNCE_API_KEY) must be set in .env file");
    
    let zb = ZeroBounce::new(&api_key);
    
    // Example 1: Using domain
    println!("=== Example 1: Using domain ===");
    let find_email_response = zb.find_email_v2()
        .first_name("John")
        .domain("example.com")
        .last_name("Doe")
        .call()?;
    
    println!("Response: {:#?}", find_email_response);
    println!();
    
    // Example 2: Using company name
    println!("=== Example 2: Using company name ===");
    let find_email_response = zb.find_email_v2()
        .first_name("John")
        .company_name("Example Inc")
        .last_name("Doe")
        .call()?;
    
    println!("Response: {:#?}", find_email_response);
    println!();
    
    // Example 3: With middle name
    println!("=== Example 3: With middle name ===");
    let find_email_response = zb.find_email_v2()
        .first_name("John")
        .domain("example.com")
        .middle_name("Middle")
        .last_name("Doe")
        .call()?;
    
    println!("Response: {:#?}", find_email_response);
    
    Ok(())
}

