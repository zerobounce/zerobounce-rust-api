use std::env;
use zero_bounce::ZeroBounce;
use zero_bounce::utility::ZBResult;

fn main() -> ZBResult<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    // Get API key from environment variable
    let api_key = env::var("ZERO_BOUNCE_API_KEY")
        .expect("ZERO_BOUNCE_API_KEY must be set in .env file");
    
    let find_mail_response = ZeroBounce::new(&api_key)
        .find_email(
            "example.com", "John", "", "Doe"
        )?;

    println!("Response: {:#?}", find_mail_response);

    Ok(())
}