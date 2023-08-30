use zero_bounce::ZeroBounce;
use zero_bounce::utility::ZBResult;

const ZERO_BOUNCE_API_KEY: &str = "YOUR API KEY";

fn main() -> ZBResult<()> {
    let find_mail_response = ZeroBounce::new(ZERO_BOUNCE_API_KEY)
        .find_email(
            "example.com", "John", "", "Doe"
        )?;

    println!("Response: {:#?}", find_mail_response);

    Ok(())
}