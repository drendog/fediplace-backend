use sqlx::PgPool;
use time::OffsetDateTime;
use tracing::{debug, instrument};
use uuid::Uuid;

use domain::auth::{Role, RoleId, UserId, UserPublic};
use fedi_wplace_application::{
    error::{AppError, AppResult},
    ports::outgoing::user_store::UserStorePort,
};

use super::utils::{PostgresExecutor, begin_transaction, commit_transaction};

pub struct PostgresUserStoreAdapter {
    pool: PgPool,
    executor: PostgresExecutor,
}

impl PostgresUserStoreAdapter {
    pub fn new(pool: PgPool, query_timeout_secs: u64) -> Self {
        Self {
            pool,
            executor: PostgresExecutor::new(query_timeout_secs),
        }
    }

    async fn load_user_roles(&self, user_id: Uuid) -> AppResult<Vec<Role>> {
        let roles = self
            .executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                    SELECT r.id, r.name, r.description, r.created_at, r.updated_at
                    FROM roles r
                    JOIN user_roles ur ON r.id = ur.role_id
                    WHERE ur.user_id = $1
                    "#,
                        user_id
                    )
                    .fetch_all(&self.pool)
                },
                &format!("Failed to load roles for user {}", user_id),
            )
            .await?;

        Ok(roles
            .into_iter()
            .map(|row| Role {
                id: RoleId::from_uuid(row.id),
                name: row.name,
                description: row.description,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }

    async fn build_user_public(
        &self,
        id: Uuid,
        email: String,
        username: String,
        email_verified_at: Option<OffsetDateTime>,
        available_charges: i32,
        charges_updated_at: OffsetDateTime,
    ) -> AppResult<UserPublic> {
        let roles = self.load_user_roles(id).await?;
        Ok(UserPublic {
            id: UserId::from_uuid(id),
            email,
            username,
            email_verified_at,
            available_charges,
            charges_updated_at,
            roles,
        })
    }
}

#[async_trait::async_trait]
impl UserStorePort for PostgresUserStoreAdapter {
    #[instrument(skip(self, password_hash))]
    async fn create_user_with_password(
        &self,
        email: &str,
        username: &str,
        password_hash: &str,
    ) -> AppResult<UserPublic> {
        let user_id = Uuid::new_v4();

        self.executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                    INSERT INTO users (id, email, username, password_hash)
                    VALUES ($1, $2, $3, $4)
                    "#,
                        user_id,
                        email,
                        username,
                        password_hash
                    )
                    .execute(&self.pool)
                },
                &format!("Failed to create user with email {}", email),
            )
            .await?;

        debug!(
            "Successfully created user with email {} and id {}",
            email, user_id
        );

        self.build_user_public(
            user_id,
            email.to_string(),
            username.to_string(),
            None,
            30,
            time::OffsetDateTime::now_utc(),
        )
        .await
    }

    #[instrument(skip(self))]
    async fn find_user_by_email(
        &self,
        email: &str,
    ) -> AppResult<Option<(Uuid, String, String, Option<String>, Option<OffsetDateTime>)>> {
        let row = self
            .executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                    SELECT id, email, username, password_hash, email_verified_at
                    FROM users
                    WHERE email = $1
                    "#,
                        email
                    )
                    .fetch_optional(&self.pool)
                },
                &format!("Failed to find user by email {}", email),
            )
            .await?;

        if let Some(record) = row {
            debug!("Found user by email {} with id {}", email, record.id);
            Ok(Some((
                record.id,
                record.email,
                record.username,
                record.password_hash,
                record.email_verified_at,
            )))
        } else {
            debug!("User with email {} not found", email);
            Ok(None)
        }
    }

    #[instrument(skip(self))]
    async fn find_user_by_username(&self, username: &str) -> AppResult<Option<UserPublic>> {
        let row = self.executor.execute_with_timeout(
            || {
                sqlx::query!(
                    r#"
                    SELECT id, email, username, email_verified_at, available_charges, charges_updated_at
                    FROM users
                    WHERE username = $1
                    "#,
                    username
                )
                .fetch_optional(&self.pool)
            },
            &format!("Failed to find user by username {}", username),

        )
        .await?;

        if let Some(record) = row {
            debug!("Found user by username {} with id {}", username, record.id);
            Ok(Some(
                self.build_user_public(
                    record.id,
                    record.email,
                    record.username,
                    record.email_verified_at,
                    record.available_charges,
                    record.charges_updated_at,
                )
                .await?,
            ))
        } else {
            debug!("User with username {} not found", username);
            Ok(None)
        }
    }

    #[instrument(skip(self))]
    async fn find_user_by_id(&self, id: Uuid) -> AppResult<Option<UserPublic>> {
        let row = self.executor.execute_with_timeout(
            || {
                sqlx::query!(
                    r#"
                    SELECT id, email, username, email_verified_at, available_charges, charges_updated_at
                    FROM users
                    WHERE id = $1
                    "#,
                    id
                )
                .fetch_optional(&self.pool)
            },
            &format!("Failed to find user by id {}", id),

        )
        .await?;

        if let Some(record) = row {
            debug!("Found user by id {}", id);
            Ok(Some(
                self.build_user_public(
                    record.id,
                    record.email,
                    record.username,
                    record.email_verified_at,
                    record.available_charges,
                    record.charges_updated_at,
                )
                .await?,
            ))
        } else {
            debug!("User with id {} not found", id);
            Ok(None)
        }
    }

    #[instrument(skip(self))]
    async fn create_or_get_social_user(
        &self,
        provider: &str,
        provider_user_id: &str,
        email: Option<&str>,
        username: Option<&str>,
    ) -> AppResult<UserPublic> {
        let existing_identity = self.executor.execute_with_timeout(
            || {
                sqlx::query!(
                    r#"
                    SELECT ui.user_id, u.email, u.username, u.email_verified_at, u.available_charges, u.charges_updated_at
                    FROM user_identities ui
                    JOIN users u ON ui.user_id = u.id
                    WHERE ui.provider = $1 AND ui.provider_user_id = $2
                    "#,
                    provider,
                    provider_user_id
                )
                .fetch_optional(&self.pool)
            },
            &format!(
                "Failed to find identity for provider {} user {}",
                provider, provider_user_id
            ),

        )
        .await?;

        if let Some(identity_record) = existing_identity {
            debug!(
                "Found existing social user for provider {} user {} with id {}",
                provider, provider_user_id, identity_record.user_id
            );
            return self
                .build_user_public(
                    identity_record.user_id,
                    identity_record.email,
                    identity_record.username,
                    identity_record.email_verified_at,
                    identity_record.available_charges,
                    identity_record.charges_updated_at,
                )
                .await;
        }

        let user_id = if let Some(email) = email {
            if let Some((existing_user_id, _, _, _, _)) = self.find_user_by_email(email).await? {
                debug!("Found existing user by email {} for social login", email);
                existing_user_id
            } else {
                let new_user_id = Uuid::new_v4();
                let default_username = format!("user_{}", &new_user_id.to_string()[..8]);
                let username = username.unwrap_or(&default_username);

                self.executor
                    .execute_with_timeout(
                        || {
                            sqlx::query!(
                                r#"
                            INSERT INTO users (id, email, username, email_verified_at)
                            VALUES ($1, $2, $3, NOW())
                            "#,
                                new_user_id,
                                email,
                                username
                            )
                            .execute(&self.pool)
                        },
                        &format!("Failed to create social user with email {}", email),
                    )
                    .await?;

                debug!(
                    "Created new social user with email {} and id {}",
                    email, new_user_id
                );
                new_user_id
            }
        } else {
            let new_user_id = Uuid::new_v4();
            let generated_email = format!("{}+{}@social.local", provider, provider_user_id);
            let default_username = format!("user_{}", &new_user_id.to_string()[..8]);
            let username = username.unwrap_or(&default_username);

            self.executor
                .execute_with_timeout(
                    || {
                        sqlx::query!(
                            r#"
                        INSERT INTO users (id, email, username, email_verified_at)
                        VALUES ($1, $2, $3, NOW())
                        "#,
                            new_user_id,
                            generated_email,
                            username
                        )
                        .execute(&self.pool)
                    },
                    "Failed to create social user without email",
                )
                .await?;

            debug!(
                "Created new social user without email, generated {} with id {}",
                generated_email, new_user_id
            );
            new_user_id
        };

        let identity_id = Uuid::new_v4();
        self.executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                    INSERT INTO user_identities (id, user_id, provider, provider_user_id)
                    VALUES ($1, $2, $3, $4)
                    "#,
                        identity_id,
                        user_id,
                        provider,
                        provider_user_id
                    )
                    .execute(&self.pool)
                },
                &format!(
                    "Failed to create identity for provider {} user {}",
                    provider, provider_user_id
                ),
            )
            .await?;

        debug!(
            "Created identity for provider {} user {} linking to user {}",
            provider, provider_user_id, user_id
        );

        self.find_user_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::DatabaseError {
                message: "User was created but could not be retrieved".to_string(),
            })
    }

    #[instrument(skip(self))]
    async fn store_verification_token(
        &self,
        user_id: Uuid,
        token: &str,
        expires_at: OffsetDateTime,
    ) -> AppResult<()> {
        self.executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                    INSERT INTO email_verification_tokens (token, user_id, expires_at)
                    VALUES ($1, $2, $3)
                    "#,
                        token,
                        user_id,
                        expires_at
                    )
                    .execute(&self.pool)
                },
                &format!("Failed to store verification token for user {}", user_id),
            )
            .await?;

        debug!(
            "Successfully stored verification token for user {}",
            user_id
        );
        Ok(())
    }

    #[instrument(skip(self))]
    async fn verify_user_by_token(&self, token: &str) -> AppResult<UserPublic> {
        let mut tx = begin_transaction(&self.pool).await?;

        let token_record = sqlx::query!(
            r#"
            SELECT user_id, expires_at
            FROM email_verification_tokens
            WHERE token = $1
            "#,
            token
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError {
            message: format!("Failed to find verification token: {}", e),
        })?;

        let token_record = token_record.ok_or(AppError::TokenNotFound)?;

        let now = OffsetDateTime::now_utc();
        if token_record.expires_at < now {
            return Err(AppError::TokenExpired);
        }

        sqlx::query!(
            r#"
            UPDATE users
            SET email_verified_at = NOW()
            WHERE id = $1
            "#,
            token_record.user_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError {
            message: format!("Failed to update user verification status: {}", e),
        })?;

        sqlx::query!(
            r#"
            DELETE FROM email_verification_tokens
            WHERE token = $1
            "#,
            token
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError {
            message: format!("Failed to delete verification token: {}", e),
        })?;

        commit_transaction(tx).await?;

        debug!(
            "Successfully verified user {} by token",
            token_record.user_id
        );

        self.find_user_by_id(token_record.user_id)
            .await?
            .ok_or_else(|| AppError::DatabaseError {
                message: "User was verified but could not be retrieved".to_string(),
            })
    }

    #[instrument(skip(self))]
    async fn update_username(&self, user_id: Uuid, new_username: &str) -> AppResult<UserPublic> {
        self.executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                    UPDATE users
                    SET username = $1
                    WHERE id = $2
                    "#,
                        new_username,
                        user_id
                    )
                    .execute(&self.pool)
                },
                &format!("Failed to update username for user {}", user_id),
            )
            .await?;

        debug!(
            "Successfully updated username for user {} to {}",
            user_id, new_username
        );

        self.find_user_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::DatabaseError {
                message: "User was updated but could not be retrieved".to_string(),
            })
    }

    #[instrument(skip(self))]
    async fn assign_role_to_user(
        &self,
        user_id: Uuid,
        role_id: Uuid,
        assigned_by: Uuid,
    ) -> AppResult<UserPublic> {
        let mut tx = begin_transaction(&self.pool).await?;

        let role_exists = sqlx::query!(
            r#"
            SELECT id FROM roles WHERE id = $1
            "#,
            role_id
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError {
            message: format!("Failed to find role {}: {}", role_id, e),
        })?
        .is_some();

        if !role_exists {
            return Err(AppError::ValidationError {
                message: format!("Role with id {} not found", role_id),
            });
        }

        let user_exists = sqlx::query!(
            r#"
            SELECT id FROM users WHERE id = $1
            "#,
            user_id
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError {
            message: format!("Failed to check if user exists: {}", e),
        })?
        .is_some();

        if !user_exists {
            return Err(AppError::ValidationError {
                message: format!("User with id {} not found", user_id),
            });
        }

        sqlx::query!(
            r#"
            INSERT INTO user_roles (user_id, role_id, assigned_by)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, role_id) DO NOTHING
            "#,
            user_id,
            role_id,
            assigned_by
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError {
            message: format!("Failed to assign role to user: {}", e),
        })?;

        commit_transaction(tx).await?;

        debug!(
            "Successfully assigned role {} to user {} by user {}",
            role_id, user_id, assigned_by
        );

        self.find_user_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::DatabaseError {
                message: "User role was assigned but user could not be retrieved".to_string(),
            })
    }
}
