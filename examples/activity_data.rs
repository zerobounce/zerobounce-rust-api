use std::env;
use zero_bounce::utility::ZBResult;
use zero_bounce::ZeroBounce;

fn main() -> ZBResult<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    // Get API key from environment variable
    let api_key = env::var("ZERO_BOUNCE_API_KEY")
        .expect("ZERO_BOUNCE_API_KEY must be set in .env file");
    
    let activity_data = ZeroBounce::new(&api_key)
        .get_activity_data("valid@example.com")?;

    println!("Activity data: {:#?}", activity_data);

    Ok(())
}