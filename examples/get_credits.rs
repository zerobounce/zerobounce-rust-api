use zero_bounce::ZeroBounce;
use zero_bounce::utility::ZBResult;

const ZERO_BOUNCE_API_KEY: &str = "YOUR API KEY";

fn main() -> ZBResult<()> {
    let credits = ZeroBounce::new(ZERO_BOUNCE_API_KEY)
        .get_credits()?;

    println!("Credits left: {}", credits);

    Ok(())
}