//! Who holds Navidrome's administrator, and claiming it: the administrator lemonfiber
//! minted, made by Navidrome's first-run setup while it holds no account at all.

use async_trait::async_trait;
use lemonfiber_ports::http::Method;
use lemonfiber_ports::service::{Failure, Owner, Ownership};
use serde::Deserialize;

use super::{Navidrome, Session};

const APP: &str = "/app/";

const CREATE_ADMIN: &str = "/auth/createAdmin";

const CONFIGURED: &str = "__APP_CONFIG__ = ";

const UNREAD: &str = "whether Navidrome has its first administrator could not be read";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Configured {
    first_time: bool,
}

/// Whether the page Navidrome serves its app with says its first run is still to come,
/// read from the configuration the page carries as a JSON string.
pub(super) fn first_time(page: &str) -> Option<bool> {
    let (_, after) = page.split_once(CONFIGURED)?;
    let carried = serde_json::Deserializer::from_str(after)
        .into_iter::<String>()
        .next()?
        .ok()?;
    serde_json::from_str::<Configured>(&carried)
        .ok()
        .map(|configured| configured.first_time)
}

impl Navidrome {
    async fn unclaimed(&self) -> Result<bool, Failure> {
        let response = self
            .endpoint
            .send(&self.request(Method::Get, APP, None, None))
            .await?;
        self.endpoint.expect_success(&response)?;
        first_time(&response.body).ok_or_else(|| self.endpoint.refused(UNREAD))
    }
}

#[async_trait]
impl Ownership for Navidrome {
    async fn owner(&self) -> Result<Owner, Failure> {
        if self.unclaimed().await? {
            return Ok(Owner::Open);
        }
        match self.opened().await {
            Ok(session) if session.is_admin => Ok(Owner::Ours),
            Ok(_) | Err(Failure::Unauthorised { .. }) => Ok(Owner::Theirs),
            Err(failure) => Err(failure),
        }
    }

    async fn claim(&self) -> Result<Owner, Failure> {
        let held = self.owner().await?;
        if held != Owner::Open {
            return Ok(held);
        }
        let response = self
            .endpoint
            .send(&self.request(
                Method::Post,
                CREATE_ADMIN,
                Some(self.admin()?.to_string()),
                None,
            ))
            .await?;
        if response.is_success() {
            let session: Session = self.endpoint.decode(
                &response,
                "the administrator Navidrome made could not be read",
            )?;
            self.held(session)?;
            return Ok(Owner::Ours);
        }
        match self.owner().await? {
            Owner::Open => Err(self.endpoint.refusal(&response)),
            held => Ok(held),
        }
    }
}
