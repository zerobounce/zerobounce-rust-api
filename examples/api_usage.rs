use std::env;
use chrono::Utc;

use zero_bounce::ZeroBounce;
use zero_bounce::utility::ZBResult;

fn main() -> ZBResult<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    // Get API key from environment variable
    let api_key = env::var("ZERO_BOUNCE_API_KEY")
        .expect("ZERO_BOUNCE_API_KEY must be set in .env file");
    
    let zb_instance = ZeroBounce::new(&api_key);
    let today_usage = zb_instance.get_api_usage(
        Utc::now().date_naive(), Utc::now().date_naive()
    )?;
    let overall_usage = zb_instance.get_api_usage_overall()?;

    println!("Total calls today: {}", today_usage.total);
    println!("Total calls overall: {}", overall_usage.total);

    Ok(())
}
