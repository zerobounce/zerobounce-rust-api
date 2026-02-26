use std::env;
use chrono::Utc;

use zero_bounce::ZeroBounce;
use zero_bounce::utility::ZBResult;

fn main() -> ZBResult<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    // Get API key from environment variable (standard: ZEROBOUNCE_API_KEY; legacy: ZERO_BOUNCE_API_KEY)
    let api_key = env::var("ZEROBOUNCE_API_KEY")
        .or_else(|_| env::var("ZERO_BOUNCE_API_KEY"))
        .expect("ZEROBOUNCE_API_KEY (or ZERO_BOUNCE_API_KEY) must be set in .env file");
    
    let zb_instance = ZeroBounce::new(&api_key);
    let today_usage = zb_instance.get_api_usage(
        Utc::now().date_naive(), Utc::now().date_naive()
    )?;
    let overall_usage = zb_instance.get_api_usage_overall()?;

    println!("Total calls today: {}", today_usage.total);
    println!("Total calls overall: {}", overall_usage.total);

    Ok(())
}
