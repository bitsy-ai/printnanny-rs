

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Verification Failed")]
    VerificationFailed
}