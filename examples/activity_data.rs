use std::env;
use zero_bounce::utility::ZBResult;
use zero_bounce::ZeroBounce;

fn main() -> ZBResult<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    // Get API key from environment variable (standard: ZEROBOUNCE_API_KEY; legacy: ZERO_BOUNCE_API_KEY)
    let api_key = env::var("ZEROBOUNCE_API_KEY")
        .or_else(|_| env::var("ZERO_BOUNCE_API_KEY"))
        .expect("ZEROBOUNCE_API_KEY (or ZERO_BOUNCE_API_KEY) must be set in .env file");
    
    let activity_data = ZeroBounce::new(&api_key)
        .get_activity_data("valid@example.com")?;

    println!("Activity data: {:#?}", activity_data);

    Ok(())
}