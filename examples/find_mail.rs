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
    
    let find_mail_response = ZeroBounce::new(&api_key)
        .find_email(
            "example.com", "John", "", "Doe"
        )?;

    println!("Response: {:#?}", find_mail_response);

    Ok(())
}