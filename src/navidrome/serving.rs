//! Looking over Navidrome's library again. What it has no equivalent of is unsupported.

use async_trait::async_trait;
use lemonfiber_adapter_kit::endpoint::form_encoded;
use lemonfiber_ports::media::TitleKind;
use lemonfiber_ports::service::{
    Allowing, AppKeys, Applied, Astray, Channel, Dated, Declared, Declaring, Encrypting, Failure,
    FiledAt, Fronted, Furnished, Furnishing, Heard, Library, OutsideAccount, OutsideServer,
    Recounting, SeriesHeld, Tuned, Tuning, Upkeep,
};
use serde::Deserialize;

use super::{Navidrome, ALBUMS};

const NO_KINDS: &str = "Navidrome holds neither films nor series";

const NO_OUTSIDE: &str = "Navidrome holds its own accounts, and no outside service holds them";

const NO_EVENTS: &str = "Navidrome tells nobody what it adds or what goes wrong";

const NO_LIVE_TV: &str = "Navidrome has no live TV";

const NO_REFRESH: &str = "Navidrome reads its library afresh as a whole, never one item";

const NO_KEYS: &str = "Navidrome holds no keys an app is given";

const NO_PROXIES: &str = "Navidrome is told the addresses it trusts to name the client by its environment, and cannot be asked";

const NO_ORIGINS: &str = "Navidrome holds no list of origins a browser may read it from";

const NO_RESTART: &str = "Navidrome cannot be asked to start again";

const NO_GUARD: &str = "Navidrome serves no member through a guard of lemonfiber's";

const NO_TLS: &str = "Navidrome serves no internal TLS port";

const NO_LIBRARIES: &str = "Navidrome reads the one music folder its configuration names";

const SCAN: &str = "startScan";

#[derive(Deserialize)]
struct Named {
    #[serde(default)]
    name: String,
}

#[async_trait]
impl Library for Navidrome {
    async fn has_item(&self, kind: TitleKind, term: &str) -> Result<bool, Failure> {
        if kind != TitleKind::Music {
            return Ok(false);
        }
        let albums: Vec<Named> = self
            .read(
                &format!("{ALBUMS}?{}", form_encoded(&[("name", term)])),
                "the albums Navidrome holds could not be read",
            )
            .await?;
        let needle = term.to_lowercase();
        Ok(albums
            .iter()
            .any(|album| album.name.to_lowercase().contains(&needle)))
    }

    async fn rescan(&self) -> Result<(), Failure> {
        self.subsonic::<serde::de::IgnoredAny>(
            SCAN,
            &[],
            "whether Navidrome started looking over its library could not be read",
        )
        .await
        .map(|_| ())
    }
}

#[async_trait]
impl Upkeep for Navidrome {
    async fn series_held(&self, _most: u32) -> Result<Vec<SeriesHeld>, Failure> {
        Err(Self::unsupported(NO_KINDS))
    }

    async fn refresh(&self, _id: &str) -> Result<(), Failure> {
        Err(Self::unsupported(NO_REFRESH))
    }
}

#[async_trait]
impl AppKeys for Navidrome {
    async fn filed_as(&self, _app: &str) -> Result<Vec<String>, Failure> {
        Err(Self::unsupported(NO_KEYS))
    }

    async fn mint(&self, _app: &str) -> Result<String, Failure> {
        Err(Self::unsupported(NO_KEYS))
    }

    async fn dated(&self, _app: &str) -> Result<Vec<Dated>, Failure> {
        Err(Self::unsupported(NO_KEYS))
    }

    async fn answers_to(&self, _key: &str) -> Result<(), Failure> {
        Err(Self::unsupported(NO_KEYS))
    }

    async fn revoke(&self, _key: &str) -> Result<(), Failure> {
        Err(Self::unsupported(NO_KEYS))
    }

    async fn astray(&self, _app: &str) -> Result<Vec<Astray>, Failure> {
        Err(Self::unsupported(NO_KEYS))
    }

    async fn account(&self, _app: &str) -> Result<OutsideAccount, Failure> {
        Err(Self::unsupported(NO_OUTSIDE))
    }

    async fn listed_as(&self) -> Result<OutsideServer, Failure> {
        Err(Self::unsupported(NO_OUTSIDE))
    }
}

#[async_trait]
impl Allowing for Navidrome {
    async fn allow_origins(&self, _origins: &[String]) -> Result<(), Failure> {
        Err(Self::unsupported(NO_ORIGINS))
    }
}

#[async_trait]
impl Recounting for Navidrome {
    async fn events(
        &self,
        _address: Option<&str>,
        _pin: Option<&str>,
        _key: Option<&str>,
        _every: Option<u32>,
    ) -> Result<Option<Heard>, Failure> {
        Err(Self::unsupported(NO_EVENTS))
    }
}

#[async_trait]
impl Tuning for Navidrome {
    async fn channels(&self, _member: &str) -> Result<Vec<Channel>, Failure> {
        Err(Self::unsupported(NO_LIVE_TV))
    }

    async fn tune(&self, _member: &str, _channel: &str) -> Result<Option<Tuned>, Failure> {
        Err(Self::unsupported(NO_LIVE_TV))
    }

    async fn untune(&self, _closes_by: &str) -> Result<(), Failure> {
        Err(Self::unsupported(NO_LIVE_TV))
    }
}

#[async_trait]
impl Fronted for Navidrome {
    async fn known_proxies(&self) -> Result<Vec<String>, Failure> {
        Err(Self::unsupported(NO_PROXIES))
    }

    async fn trust_only(&self, _address: &str) -> Result<(), Failure> {
        Err(Self::unsupported(NO_PROXIES))
    }

    async fn allowed_origins(&self) -> Result<Vec<String>, Failure> {
        Err(Self::unsupported(NO_ORIGINS))
    }

    async fn allow_only(&self, _origin: &str) -> Result<(), Failure> {
        Err(Self::unsupported(NO_ORIGINS))
    }

    async fn restart(&self) -> Result<(), Failure> {
        Err(Self::unsupported(NO_RESTART))
    }
}

#[async_trait]
impl Declaring for Navidrome {
    async fn declared(&self) -> Result<Declared, Failure> {
        Err(Self::unsupported(NO_GUARD))
    }
}

#[async_trait]
impl Encrypting for Navidrome {
    async fn serve_tls(&self, _path: &str, _pin: &str) -> Result<Applied, Failure> {
        Err(Self::unsupported(NO_TLS))
    }
}

#[async_trait]
impl Furnishing for Navidrome {
    async fn furnish(&self, _filed: &[FiledAt]) -> Result<Vec<Furnished>, Failure> {
        Err(Self::unsupported(NO_LIBRARIES))
    }
}
