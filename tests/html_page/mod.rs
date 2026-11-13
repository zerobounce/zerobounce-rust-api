//! Each marker of the HTML sniff on its own. A page with no other HTML served
//! as `text/csv` must not be saved as a result file: the download is an error
//! with the status (standard.md §3, downloads).

use zero_bounce::utility::ENDPOINT_FILE_RESULT;
use zero_bounce::utility::ZBError;

use crate::common::{endpoint_matcher, instantiate};

/// One marker per body, so removing any one of them from the sniff fails here.
const PAGES: [&str; 5] = ["<!DOCTYPE html>", "<!doctype html>", "<html>", "<head>", "<body>"];

#[test]
fn each_html_marker_alone_makes_a_csv_download_an_error() {
    let mut failures = Vec::new();
    for marker in PAGES {
        let (mut server, zb) = instantiate();
        let mock = server
            .mock("GET", endpoint_matcher(ENDPOINT_FILE_RESULT))
            .with_status(200)
            .with_header("content-type", "text/csv")
            .with_body(format!("{marker}\nemail\na@b.c\n"))
            .create();
        let outcome = zb.bulk_validation_result_fetch("file_id");
        mock.assert();
        match outcome {
            Err(ZBError::ApiError { status: 200, .. }) => {}
            other => failures.push(format!("{marker}: {other:?}")),
        }
    }
    assert!(failures.is_empty(), "saved as a file:\n{}", failures.join("\n"));
}
