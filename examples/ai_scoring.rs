use std::env;
use std::io::Write;

use zero_bounce::utility::ZBResult;
use zero_bounce::utility::structures::bulk::ZBBulkResponse;
use zero_bounce::{ZeroBounce, ZBFile};

fn main() -> ZBResult<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    // Get API key from environment variable
    let api_key = env::var("ZERO_BOUNCE_API_KEY")
        .expect("ZERO_BOUNCE_API_KEY must be set in .env file");

    let file_content = String::from("")
        + "invalid@example.com\n"
        + "valid@example.com\n"
        + "toxic@example.com\n"
        + "donotmail@example.com\n";

    // initialize ZBFile to be used for bulk request
    let file_content_vec = Vec::from(file_content);
    let zb_instance = ZeroBounce::new(&api_key);
    let mut zb_file = ZBFile::from_content(file_content_vec);

    // Alternatively:
    // let mut zb_file = ZBFile::from_path("/path/to/file.csv".to_string());

    // customize file
    zb_file = zb_file
        .set_remove_duplicate(false)
        .set_has_header_row(false);

    // submit file for bulk validation
    let submit_inst = zb_instance.ai_scoring_file_submit(&zb_file)?;

    // extract file_id (to use in future endpoints)
    assert!(submit_inst.file_id.is_some());
    let file_id_string = submit_inst.file_id.unwrap();
    let file_id = file_id_string.as_str();

    // check status of the submitted file
    let mut file_status = zb_instance.ai_scoring_file_status_check(file_id)?;
    println!("File status: {:#?}", file_status);

    // wait for the file to be handled
    let mut time_to_wait = 1;
    while file_status.complete_percentage != 100. {
        if time_to_wait < 10 {
            time_to_wait += 1;
        }
        std::thread::sleep(std::time::Duration::from_secs(time_to_wait));
        print!(".");
        std::io::stdout().flush().unwrap();

        file_status = zb_instance.ai_scoring_file_status_check(file_id)?;
    }
    println!("");

    // fetch the result of the bulk validation
    let bulk_result = zb_instance.ai_scoring_result_fetch(file_id)?;
    match bulk_result {
        ZBBulkResponse::Content(bytes) => {
            println!("Raw content:");
            println!("{:#?}", bytes);
        },
        ZBBulkResponse::Feedback(feedback) => {
            println!("Feedback:");
            println!("{:#?}", feedback);
        },
    };

    // delete the result file
    println!("Deleting file:");
    let delete_result = zb_instance.ai_scoring_result_delete(file_id)?;
    println!("{:#?}", delete_result);

    Ok(())
}
