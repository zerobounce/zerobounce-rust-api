use zero_bounce::utility::ZBResult;
use zero_bounce::ZeroBounce;

const ZERO_BOUNCE_API_KEY: &str = "YOUR API KEY";

fn main() -> ZBResult<()> {
    let emails_and_ips = vec![
        (String::from("valid@example.com"),     String::from("99.110.204.1")),
        (String::from("example@example.com"),   String::from("")),
    ];
    let activity_data = ZeroBounce::new(ZERO_BOUNCE_API_KEY)
        .batch_validate(emails_and_ips);

    println!("Activity data: {:#?}", activity_data);

    Ok(())
}
