use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Authentication required")]
    NotAuthenticated,
    #[error("Invalid token")]
    InvalidToken,
}

pub struct AuthManager {
    token: Option<String>,
}

impl AuthManager {
    pub fn new() -> Self {
        Self { token: None }
    }

    pub fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }
}
