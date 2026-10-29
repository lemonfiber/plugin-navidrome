//! Looking over Navidrome's library again. What it has no equivalent of is unsupported.

use async_trait::async_trait;
use lemonfiber_ports::media::Kind;
use lemonfiber_ports::service::{
    AppKeys, Applied, Dated, Declared, Declaring, Encrypting, Failure, Fronted, Library,
    SeriesHeld, Upkeep,
};

use super::Navidrome;

const NO_KINDS: &str = "Navidrome holds neither films nor series";

const NO_REFRESH: &str = "Navidrome reads its library afresh as a whole, never one item";

const NO_KEYS: &str = "Navidrome holds no keys an app is given";

const NO_PROXIES: &str = "Navidrome is told the addresses it trusts to name the client by its environment, and cannot be asked";

const NO_ORIGINS: &str = "Navidrome holds no list of origins a browser may read it from";

const NO_RESTART: &str = "Navidrome cannot be asked to start again";

const NO_GUARD: &str = "Navidrome serves no member through a guard of lemonfiber's";

const NO_TLS: &str = "Navidrome serves no internal TLS port";

const SCAN: &str = "startScan";

#[async_trait]
impl Library for Navidrome {
    async fn has_item(&self, _kind: Kind, _term: &str) -> Result<bool, Failure> {
        Err(Self::unsupported(NO_KINDS))
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
