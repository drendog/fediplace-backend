use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct CreditConfig {
    pub max_charges: i32,
    pub charge_cooldown_seconds: i32,
}

impl CreditConfig {
    pub fn new(max_charges: i32, charge_cooldown_seconds: i32) -> Self {
        Self {
            max_charges,
            charge_cooldown_seconds,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreditBalance {
    pub available_charges: i32,
    pub charges_updated_at: OffsetDateTime,
}

impl CreditBalance {
    pub fn new(available_charges: i32, charges_updated_at: OffsetDateTime) -> Self {
        Self {
            available_charges,
            charges_updated_at,
        }
    }

    pub fn calculate_current_balance(&self, now: OffsetDateTime, config: &CreditConfig) -> i32 {
        let seconds_elapsed = (now - self.charges_updated_at).whole_seconds() as i32;
        let intervals_completed = seconds_elapsed / config.charge_cooldown_seconds;
        let earned_charges = intervals_completed;
        let new_balance = self.available_charges + earned_charges;

        new_balance.min(config.max_charges)
    }

    pub fn can_afford(&self, cost: i32, now: OffsetDateTime, config: &CreditConfig) -> bool {
        self.calculate_current_balance(now, config) >= cost
    }

    pub fn spend_charges(
        &mut self,
        cost: i32,
        now: OffsetDateTime,
        config: &CreditConfig,
    ) -> Result<(), InsufficientChargesError> {
        let current_balance = self.calculate_current_balance(now, config);

        if current_balance < cost {
            return Err(InsufficientChargesError {
                required: cost,
                available: current_balance,
            });
        }

        self.available_charges = current_balance - cost;
        self.charges_updated_at = now;
        Ok(())
    }

    pub fn refill_to_current(&mut self, now: OffsetDateTime, config: &CreditConfig) {
        self.available_charges = self.calculate_current_balance(now, config);
        self.charges_updated_at = now;
    }

    pub fn seconds_until_next_charge(&self, now: OffsetDateTime, config: &CreditConfig) -> i64 {
        let seconds_since_update = (now - self.charges_updated_at).whole_seconds();
        let seconds_into_current_interval =
            seconds_since_update % i64::from(config.charge_cooldown_seconds);

        if seconds_into_current_interval == 0 {
            0
        } else {
            i64::from(config.charge_cooldown_seconds) - seconds_into_current_interval
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsufficientChargesError {
    pub required: i32,
    pub available: i32,
}

impl Display for InsufficientChargesError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "Insufficient charges: required {}, available {}",
            self.required, self.available
        )
    }
}

impl Error for InsufficientChargesError {}
