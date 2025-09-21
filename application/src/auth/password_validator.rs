use crate::error::{AppError, AppResult};
use zxcvbn::{Score, zxcvbn};

pub struct PasswordValidator {
    min_score: Score,
}

impl Default for PasswordValidator {
    fn default() -> Self {
        Self {
            min_score: Score::Three,
        }
    }
}

impl PasswordValidator {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_min_score(mut self, min_score: Score) -> Self {
        self.min_score = min_score;
        self
    }

    pub fn validate(&self, password: &str) -> AppResult<()> {
        let estimate = zxcvbn(password, &[]);

        if estimate.score() < self.min_score {
            let feedback_messages = if let Some(feedback) = estimate.feedback() {
                let mut messages = Vec::new();

                if let Some(warning) = feedback.warning() {
                    messages.push(warning.to_string());
                }

                for suggestion in feedback.suggestions() {
                    messages.push(suggestion.to_string());
                }

                if messages.is_empty() {
                    vec!["Password is too weak".to_string()]
                } else {
                    messages
                }
            } else {
                vec!["Password is too weak".to_string()]
            };

            return Err(AppError::ValidationError {
                message: format!(
                    "Password strength is insufficient (score: {}/4, minimum: {}). {}",
                    estimate.score() as u8,
                    self.min_score as u8,
                    feedback_messages.join("; ")
                ),
            });
        }

        Ok(())
    }
}
