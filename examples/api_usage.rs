use chrono::Utc;

use zero_bounce::ZeroBounce;
use zero_bounce::utility::ZBResult;

const ZERO_BOUNCE_API_KEY: &str = "YOUR API KEY";

fn main() -> ZBResult<()> {
    let zb_instance = ZeroBounce::new(ZERO_BOUNCE_API_KEY);
    let today_usage = zb_instance.get_api_usage(
        Utc::now().date_naive(), Utc::now().date_naive()
    )?;
    let overall_usage = zb_instance.get_api_usage_overall()?;

    println!("Total calls today: {}", today_usage.total);
    println!("Total calls overall: {}", overall_usage.total);

    Ok(())
}
