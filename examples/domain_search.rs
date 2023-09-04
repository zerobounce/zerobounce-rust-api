use zero_bounce::ZeroBounce;
use zero_bounce::utility::ZBResult;

const ZERO_BOUNCE_API_KEY: &str = "YOUR API KEY";

fn main() -> ZBResult<()> {
    let domain_search = ZeroBounce::new(ZERO_BOUNCE_API_KEY)
        .domain_search("example.com")?;

    println!("Response: {:#?}", domain_search);
    Ok(())
}