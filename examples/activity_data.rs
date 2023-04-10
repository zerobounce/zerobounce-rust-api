use zero_bounce::utility::ZBResult;
use zero_bounce::ZeroBounce;

const ZERO_BOUNCE_API_KEY: &str = "YOUR API KEY";

fn main() -> ZBResult<()> {
    let activity_data = ZeroBounce::new(ZERO_BOUNCE_API_KEY)
        .get_activity_data("valid@example.com")?;

    println!("Activity data: {:#?}", activity_data);

    Ok(())
}