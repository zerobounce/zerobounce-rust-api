#[derive(Clone, Debug, Default)]
pub struct ZBValidateOptions {
    pub timeout: Option<u16>,
    pub activity_data: Option<bool>,
    pub verify_plus: Option<bool>,
}
