use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use domain::auth::UserPublic;
use domain::credits::{CreditBalance, CreditConfig};
use fedi_wplace_application::error::AppError;

use crate::{incoming::http_axum::dto::responses::UserResponse, shared::app_state::AppState};

pub async fn build_user_response(
    user_public: UserPublic,
    state: &AppState,
    now: OffsetDateTime,
) -> Result<UserResponse, AppError> {
    let credit_config = CreditConfig::new(
        state.config.credits.max_charges,
        state.config.credits.charge_cooldown_seconds,
    );
    let credit_balance = CreditBalance::new(
        user_public.available_charges,
        user_public.charges_updated_at,
    );
    let current_charges = credit_balance.calculate_current_balance(now, &credit_config);
    let seconds_until_next_charge = credit_balance.seconds_until_next_charge(now, &credit_config);

    let roles = user_public.roles.iter().map(|role| role.name.clone()).collect();

    let ban_status = state.ban_use_case.check_user_ban_status(&user_public.id).await?;
    let (banned, ban_reason) = match ban_status {
        Some(ban) => (true, Some(ban.reason)),
        None => (false, None),
    };

    Ok(UserResponse {
        id: *user_public.id.as_uuid(),
        email: user_public.email,
        username: user_public.username,
        email_verified: user_public.email_verified_at.is_some(),
        available_charges: current_charges,
        charges_updated_at: now.format(&Rfc3339).unwrap_or_default(),
        charge_cooldown_seconds: state.config.credits.charge_cooldown_seconds,
        seconds_until_next_charge,
        max_charges: state.config.credits.max_charges,
        roles,
        banned,
        ban_reason,
    })
}
