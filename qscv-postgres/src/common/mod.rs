pub mod bytestr;
pub mod url;

pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[macro_export]
macro_rules! general {
    ($($tt:tt)*) => {
        crate::common::GeneralError::new(format!($($tt)*))
    };
}

/// an error which only contain string message
pub struct GeneralError(String);

impl GeneralError {
    pub fn new(message: String) -> GeneralError {
        Self(message)
    }
}

impl std::error::Error for GeneralError { }

impl std::fmt::Display for GeneralError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::fmt::Debug for GeneralError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

