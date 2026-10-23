#[derive(Clone, Debug, Default)]
pub struct ZBValidateOptions {
    pub timeout: Option<u16>,
    pub activity_data: Option<bool>,
    pub verify_plus: Option<bool>,
    /// Adds `domain_website_exists` and `domain_registrant_company_name` to the response.
    pub domain_info: Option<bool>,
    /// Processes a greylisted address again instead of returning it as unknown.
    pub greylist_processing: Option<bool>,
    /// Where the result is sent once ready. Only valid with `greylist_processing: Some(true)`.
    pub callback_url: Option<String>,
    pub use_post: bool,
}
