use std::env;
use zero_bounce::ZeroBounce;
use zero_bounce::utility::ZBResult;

fn main() -> ZBResult<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    // Get API key from environment variable
    let api_key = env::var("ZERO_BOUNCE_API_KEY")
        .expect("ZERO_BOUNCE_API_KEY must be set in .env file");
    
    let zb = ZeroBounce::new(&api_key);
    
    // Example 1: Using domain
    println!("=== Example 1: Using domain ===");
    let domain_search = zb.domain_search_v2()
        .domain("example.com")
        .call()?;
    
    println!("Response: {:#?}", domain_search);
    println!();
    
    // Example 2: Using company name
    println!("=== Example 2: Using company name ===");
    let domain_search = zb.domain_search_v2()
        .company_name("Example Inc")
        .call()?;
    
    println!("Response: {:#?}", domain_search);
    
    Ok(())
}
