//! Navidrome, the music server, asked through its own API and its Subsonic API as the
//! administrator lemonfiber minted for this adapter.

use std::sync::{Arc, Mutex};

use lemonfiber_adapter_kit::credential::Holding;
use lemonfiber_adapter_kit::endpoint::{form_encoded, Endpoint};
use lemonfiber_ports::http::{Fetched, Http, Method, Request, Response, JSON};
use lemonfiber_ports::service::Failure;
use serde::Deserialize;

mod owning;
mod screening;
mod serving;

/// What every failure is blamed on.
pub const SERVICE: &str = "navidrome";

const HEALTH: &str = "/ping";

const LOGIN: &str = "/auth/login";

const SUBSONIC: &str = "/rest";

const SUBSONIC_VERSION: &str = "1.16.1";

const CLIENT: &str = "lemonfiber";

const FORMAT: &str = "json";

const AUTHORIZATION: &str = "X-ND-Authorization";

const CONTENT_TYPE: &str = "Content-Type";

const SESSION_REFUSED: u16 = 401;

const SUBSONIC_REFUSED: u32 = 40;

const SUBSONIC_OK: &str = "ok";

const ABSENT: u16 = 404;

const ID_MOST: usize = 64;

const NO_ADMIN: &str = "this adapter holds no administrator for Navidrome";

/// A client for one Navidrome, signed in as the administrator `username` and `password`
/// supply, holding the session it opened until Navidrome refuses it.
pub struct Navidrome {
    http: Arc<dyn Http>,
    endpoint: Endpoint,
    username: Arc<dyn Holding>,
    password: Arc<dyn Holding>,
    session: Mutex<Option<Session>>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    token: String,
    username: String,
    subsonic_token: String,
    subsonic_salt: String,
    #[serde(default)]
    is_admin: bool,
}

#[derive(Deserialize)]
struct Subsonic {
    #[serde(rename = "subsonic-response")]
    response: Said,
}

#[derive(Deserialize)]
struct Said {
    status: String,
    #[serde(default)]
    error: Option<Error>,
    #[serde(flatten)]
    answer: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct Error {
    code: u32,
}

impl Navidrome {
    /// The Navidrome reached at `base` over `http`, signed in as the administrator
    /// `username` and `password` supply.
    #[must_use]
    pub fn new(
        http: Arc<dyn Http>,
        base: impl Into<String>,
        username: Arc<dyn Holding>,
        password: Arc<dyn Holding>,
    ) -> Self {
        Self {
            endpoint: Endpoint::new(Arc::clone(&http), base, SERVICE),
            http,
            username,
            password,
            session: Mutex::new(None),
        }
    }

    /// Whether Navidrome says it is up, which it says to anybody.
    pub async fn answers(&self) -> bool {
        self.endpoint
            .send(&self.request(Method::Get, HEALTH, None, None))
            .await
            .is_ok_and(|response| response.is_success())
    }

    fn admin(&self) -> Result<serde_json::Value, Failure> {
        let held = |holding: &Arc<dyn Holding>| {
            holding
                .key()
                .map_err(|why| self.endpoint.refused(&format!("{NO_ADMIN}: {why}")))
        };
        Ok(serde_json::json!({
            "username": held(&self.username)?,
            "password": held(&self.password)?,
        }))
    }

    fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<String>,
        bearer: Option<&str>,
    ) -> Request {
        let mut headers = Vec::new();
        if let Some(token) = bearer {
            headers.push((AUTHORIZATION.to_owned(), format!("Bearer {token}")));
        }
        if body.is_some() {
            headers.push((CONTENT_TYPE.to_owned(), JSON.to_owned()));
        }
        Request {
            method,
            url: self.endpoint.url(path),
            headers,
            body,
            pinned: None,
        }
    }

    fn held(&self, session: Session) -> Result<Session, Failure> {
        if session.token.is_empty() || session.subsonic_token.is_empty() {
            return Err(self
                .endpoint
                .refused("Navidrome opened a session with no token"));
        }
        if let Ok(mut held) = self.session.lock() {
            held.replace(session.clone());
        }
        Ok(session)
    }

    async fn opened(&self) -> Result<Session, Failure> {
        let response = self
            .endpoint
            .send(&self.request(Method::Post, LOGIN, Some(self.admin()?.to_string()), None))
            .await?;
        let session = self
            .endpoint
            .decode(&response, "the session Navidrome opened could not be read")?;
        self.held(session)
    }

    async fn session(&self) -> Result<Session, Failure> {
        let held = self.session.lock().ok().and_then(|held| held.clone());
        match held {
            Some(session) => Ok(session),
            None => self.opened().await,
        }
    }

    async fn as_admin(&self, path: &str) -> Result<Response, Failure> {
        let session = self.session().await?;
        let response = self
            .endpoint
            .send(&self.request(Method::Get, path, None, Some(&session.token)))
            .await?;
        if response.status != SESSION_REFUSED {
            return Ok(response);
        }
        let session = self.opened().await?;
        self.endpoint
            .send(&self.request(Method::Get, path, None, Some(&session.token)))
            .await
    }

    async fn read<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        what: &str,
    ) -> Result<T, Failure> {
        let response = self.as_admin(path).await?;
        self.endpoint.decode(&response, what)
    }

    fn subsonic_path(session: &Session, call: &str, asked: &[(&str, &str)]) -> String {
        let mut fields = vec![
            ("u", session.username.as_str()),
            ("t", session.subsonic_token.as_str()),
            ("s", session.subsonic_salt.as_str()),
            ("v", SUBSONIC_VERSION),
            ("c", CLIENT),
            ("f", FORMAT),
        ];
        fields.extend_from_slice(asked);
        format!("{SUBSONIC}/{call}?{}", form_encoded(&fields))
    }

    async fn subsonic<T: serde::de::DeserializeOwned>(
        &self,
        call: &str,
        asked: &[(&str, &str)],
        what: &str,
    ) -> Result<T, Failure> {
        let session = self.session().await?;
        match self.subsonic_as(&session, call, asked, what).await? {
            Ok(answer) => return Ok(answer),
            Err(error) if error.code != SUBSONIC_REFUSED => {
                return Err(self.subsonic_refusal(&error))
            }
            Err(_) => {}
        }
        let session = self.opened().await?;
        self.subsonic_as(&session, call, asked, what)
            .await?
            .map_err(|error| self.subsonic_refusal(&error))
    }

    async fn subsonic_as<T: serde::de::DeserializeOwned>(
        &self,
        session: &Session,
        call: &str,
        asked: &[(&str, &str)],
        what: &str,
    ) -> Result<Result<T, Error>, Failure> {
        let path = Self::subsonic_path(session, call, asked);
        let response = self
            .endpoint
            .send(&self.request(Method::Get, &path, None, None))
            .await?;
        let said: Subsonic = self.endpoint.decode(&response, what)?;
        if let Some(error) = said.response.error {
            return Ok(Err(error));
        }
        if said.response.status != SUBSONIC_OK {
            return Err(self.endpoint.refused(&format!(
                "{what}: Navidrome answered {}",
                said.response.status
            )));
        }
        serde_json::from_value(serde_json::Value::Object(said.response.answer))
            .map(Ok)
            .map_err(|_| {
                self.endpoint
                    .refused(&format!("{what}: the answer is not in the shape expected"))
            })
    }

    fn subsonic_refusal(&self, error: &Error) -> Failure {
        if error.code == SUBSONIC_REFUSED {
            return self.endpoint.unauthorised();
        }
        self.endpoint.refused(&format!(
            "Navidrome refused it with Subsonic error {}",
            error.code
        ))
    }

    async fn fetched_subsonic(
        &self,
        call: &str,
        asked: &[(&str, &str)],
        most: usize,
    ) -> Result<Result<Fetched, Error>, Failure> {
        let session = self.session().await?;
        match self.fetched_as(&session, call, asked, most).await? {
            Err(error) if error.code == SUBSONIC_REFUSED => {}
            fetched => return Ok(fetched),
        }
        let session = self.opened().await?;
        self.fetched_as(&session, call, asked, most).await
    }

    async fn fetched_as(
        &self,
        session: &Session,
        call: &str,
        asked: &[(&str, &str)],
        most: usize,
    ) -> Result<Result<Fetched, Error>, Failure> {
        let path = Self::subsonic_path(session, call, asked);
        let fetched = self
            .http
            .fetch(&self.request(Method::Get, &path, None, None), most)
            .await
            .map_err(|_| Failure::Unavailable {
                service: SERVICE.to_owned(),
            })?;
        let said = fetched
            .bytes
            .as_deref()
            .and_then(|bytes| serde_json::from_slice::<Subsonic>(bytes).ok());
        Ok(match said.and_then(|said| said.response.error) {
            Some(error) => Err(error),
            None => Ok(fetched),
        })
    }

    fn unsupported(detail: &str) -> Failure {
        Failure::Unsupported {
            service: SERVICE.to_owned(),
            detail: detail.to_owned(),
        }
    }
}

/// Whether `id` is spelled as the identifiers Navidrome assigns are: letters and digits
/// alone, and no longer than any of them.
fn an_id(id: &str) -> bool {
    (1..=ID_MOST).contains(&id.len()) && id.bytes().all(|mark| mark.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests;
