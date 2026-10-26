//! What this adapter says it is, and the kit that serves its contracts.

use std::sync::Arc;

use lemonfiber_adapter_kit::{Kit, Readiness};
use lemonfiber_contract::adapter::{About, Release};
use lemonfiber_contract::capabilities::{media, owner};

use crate::navidrome::Navidrome;

/// Where Navidrome answers, by its service id and the port it listens on.
pub const ADDRESS: &str = "http://navidrome:4533";

/// The upstream this adapter fronts, as its project names itself.
pub const UPSTREAM: &str = "Navidrome";

/// The Navidrome release this adapter's recordings were taken from.
pub const RELEASE: &str = "0.64.2";

/// The digest of the image that release's recordings were taken from.
pub const DIGEST: &str = "sha256:38dc2727bfcfd5ede290f8ada114fc90368146f265ae4701ddddbcbe2a44ee52";

/// What this adapter says it is: the contracts it speaks and the release it supports.
#[must_use]
pub fn about() -> About {
    About {
        speaks: vec![
            lemonfiber_contract::spoken(owner::claim::CAPABILITY, owner::claim::MAJOR),
            lemonfiber_contract::spoken(media::serve::CAPABILITY, media::serve::MAJOR),
        ],
        upstream: UPSTREAM.to_owned(),
        releases: vec![Release {
            version: RELEASE.to_owned(),
            digest: DIGEST.to_owned(),
        }],
    }
}

/// The contracts, served from `navidrome` under `key`, ready while Navidrome answers.
#[must_use]
pub fn kit(key: String, navidrome: Arc<Navidrome>) -> Kit {
    let asked = Arc::clone(&navidrome);
    let ready: Readiness = Arc::new(move || {
        let asked = Arc::clone(&asked);
        Box::pin(async move { asked.answers().await })
    });
    Kit::new(key, about())
        .ready_when(ready)
        .serving(owner::claim::served(Arc::clone(&navidrome)))
        .serving(media::serve::served(navidrome))
}

#[cfg(test)]
mod tests;
