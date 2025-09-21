use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::Error as HashError,
};
use fedi_wplace_application::error::{AppError, AppResult};
use fedi_wplace_application::infrastructure_config::Argon2Config;
use fedi_wplace_application::ports::outgoing::password_hasher::PasswordHasherPort;
use password_hash::{SaltString, rand_core::OsRng};

pub struct Argon2PasswordHasher {
    argon2: Argon2<'static>,
}

impl Argon2PasswordHasher {
    pub fn new() -> Self {
        let argon2 = Argon2::default();
        Self { argon2 }
    }

    pub fn from_config(config: &Argon2Config) -> AppResult<Self> {
        let output_length = config.output_length.unwrap_or(32);

        let params = Params::new(
            config.memory_cost,
            config.time_cost,
            config.parallelism,
            Some(output_length),
        )
        .map_err(|e| {
            tracing::warn!(
                "Invalid Argon2 config parameters, falling back to defaults: {}",
                e
            );
            AppError::ValidationError {
                message: format!("Invalid Argon2 parameters: {}", e),
            }
        })?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        Ok(Self { argon2 })
    }

    pub fn from_config_or_default(config: &Argon2Config) -> Self {
        Self::from_config(config).unwrap_or_else(|_| {
            tracing::info!("Using Argon2 default parameters due to invalid configuration");
            Self::new()
        })
    }

    pub fn with_custom_params(
        memory_cost: u32,
        time_cost: u32,
        parallelism: u32,
    ) -> AppResult<Self> {
        let params = Params::new(memory_cost, time_cost, parallelism, Some(32)).map_err(|e| {
            AppError::ValidationError {
                message: format!("Invalid Argon2 parameters: {}", e),
            }
        })?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        Ok(Self { argon2 })
    }
}

impl Default for Argon2PasswordHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl PasswordHasherPort for Argon2PasswordHasher {
    fn hash(&self, password: &str) -> AppResult<String> {
        if password.is_empty() {
            return Err(AppError::ValidationError {
                message: "Password cannot be empty".to_string(),
            });
        }

        let salt = SaltString::generate(&mut OsRng);
        let password_hash = self
            .argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::ValidationError {
                message: format!("Failed to hash password: {}", e),
            })?;

        Ok(password_hash.to_string())
    }

    fn verify(&self, password: &str, password_hash: &str) -> AppResult<bool> {
        if password.is_empty() {
            return Err(AppError::ValidationError {
                message: "Password cannot be empty".to_string(),
            });
        }

        if password_hash.is_empty() {
            return Err(AppError::ValidationError {
                message: "Password hash cannot be empty".to_string(),
            });
        }

        let parsed_hash =
            PasswordHash::new(password_hash).map_err(|e| AppError::ValidationError {
                message: format!("Invalid password hash format: {}", e),
            })?;

        match self
            .argon2
            .verify_password(password.as_bytes(), &parsed_hash)
        {
            Ok(()) => Ok(true),
            Err(HashError::Password) => Ok(false),
            Err(e) => Err(AppError::ValidationError {
                message: format!("Password verification failed: {}", e),
            }),
        }
    }
}
