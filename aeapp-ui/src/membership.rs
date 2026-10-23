use aeapp::core::{
    service_layer::{ServiceLayerError, SERVICE_REQUEST_GETINITUSERDATA},
    LockedAeApp,
};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Membership {
    Unknown,
    Inactive,
    EaPlay,
}
impl Membership {
    pub fn description(self) -> &'static str {
        match self {
            Self::Unknown => "Membership status is not loaded",
            Self::Inactive => "No active EA Play membership",
            Self::EaPlay => "EA Play membership active",
        }
    }
}
#[derive(Deserialize)]
struct Subscription {
    status: String,
}
#[derive(Deserialize)]
struct UserSubscriptions {
    subscriptions: Vec<Subscription>,
}
impl UserSubscriptions {
    fn membership(&self) -> Membership {
        if self.subscriptions.iter().any(|s| s.status.eq_ignore_ascii_case("ACTIVE")) {
            Membership::EaPlay
        } else {
            Membership::Inactive
        }
    }
}
pub async fn load(app: LockedAeApp) -> Result<Membership, ServiceLayerError> {
    let service = app.lock().await.service_layer().clone();
    let user: UserSubscriptions =
        service.request(SERVICE_REQUEST_GETINITUSERDATA, serde_json::json!({})).await?;
    Ok(user.membership())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_expired_membership_never_displays_an_active_badge() {
        let user: UserSubscriptions = serde_json::from_str(
            r#"{"subscriptions":[{"status":"EXPIRED"},{"status":"CANCELLED"}]}"#,
        )
        .unwrap();
        assert_eq!(user.membership(), Membership::Inactive);
        let active: UserSubscriptions =
            serde_json::from_str(r#"{"subscriptions":[{"status":"EXPIRED"},{"status":"ACTIVE"}]}"#)
                .unwrap();
        assert_eq!(active.membership(), Membership::EaPlay);
    }
    #[test]
    fn missing_subscription_data_is_not_treated_as_a_non_member() {
        assert!(serde_json::from_str::<UserSubscriptions>("{}").is_err());
        assert!(serde_json::from_str::<UserSubscriptions>(r#"{"subscriptions":[{}]}"#).is_err());
    }
}
