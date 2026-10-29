use std::sync::Arc;

use lemonfiber_fixtures::http::{Answer, Fake};
use lemonfiber_ports::http::Method;
use lemonfiber_ports::service::{Encrypting as _, Failure, Library as _, Owner, Ownership as _};

use super::owning::first_time;
use super::{an_id, Navidrome};

mod screening;

const BASE: &str = "http://navidrome:4533";

const USERNAME: &str = "qrstuvwx";

const PASSWORD: &str = "ABCDEFGHIJKLMNOP";

const TOKEN: &str = "the-session-token";

const SIGNED_IN: &str = r#"{"id":"u1","isAdmin":true,"name":"Qrstuvwx","subsonicSalt":"a1b2c3","subsonicToken":"the-subsonic-token","token":"the-session-token","username":"qrstuvwx"}"#;

const NOT_ADMIN: &str = r#"{"id":"u2","isAdmin":false,"name":"Q","subsonicSalt":"a1b2c3","subsonicToken":"the-subsonic-token","token":"the-session-token","username":"qrstuvwx"}"#;

const FIRST_TIME: &str = r#"<html><script>window.__APP_CONFIG__ = "{\"baseURL\":\"\",\"firstTime\":true,\"version\":\"0.64.2\"}";</script></html>"#;

const SET_UP: &str = r#"<html><script>window.__APP_CONFIG__ = "{\"baseURL\":\"\",\"firstTime\":false,\"version\":\"0.64.2\"}";</script></html>"#;

const SCANNING: &str =
    r#"{"subsonic-response":{"status":"ok","version":"1.16.1","scanStatus":{"scanning":true}}}"#;

const WRONG_PASSWORD: &str = r#"{"subsonic-response":{"status":"failed","version":"1.16.1","error":{"code":40,"message":"Wrong username or password"}}}"#;

const NOT_AUTHORISED: &str = r#"{"subsonic-response":{"status":"failed","version":"1.16.1","error":{"code":50,"message":"User not authorized"}}}"#;

struct Missing;

impl lemonfiber_adapter_kit::credential::Holding for Missing {
    fn key(&self) -> Result<String, String> {
        Err("upstream.json could not be read: not found".to_owned())
    }
}

fn navidrome(fake: &Arc<Fake>) -> Navidrome {
    Navidrome::new(
        fake.clone(),
        BASE,
        Arc::new(USERNAME.to_owned()),
        Arc::new(PASSWORD.to_owned()),
    )
}

fn signed_in_and(mut routes: Vec<(Method, &'static str, Answer)>) -> Arc<Fake> {
    routes.insert(
        0,
        (Method::Post, "/auth/login", Answer::reply(200, SIGNED_IN)),
    );
    Fake::by_route(routes)
}

fn asked(fake: &Fake) -> Vec<(Method, String)> {
    fake.requests()
        .into_iter()
        .map(|one| {
            let path = one.url.trim_start_matches(BASE);
            let path = path.split_once('?').map_or(path, |(path, _)| path);
            (one.method, path.to_owned())
        })
        .collect()
}

fn url_of(fake: &Fake, path: &str) -> Option<String> {
    fake.requests()
        .into_iter()
        .find(|one| one.url.contains(path))
        .map(|one| one.url)
}

fn body_of(fake: &Fake, path: &str) -> Option<serde_json::Value> {
    fake.requests()
        .into_iter()
        .find(|one| one.url.ends_with(path) && one.body.is_some())
        .and_then(|one| one.body)
        .and_then(|body| serde_json::from_str(&body).ok())
}

fn refused_saying(outcome: Result<impl std::fmt::Debug, Failure>, said: &str) -> bool {
    matches!(outcome, Err(Failure::Refused { detail, .. }) if detail.contains(said))
}

#[tokio::test]
async fn it_answers_while_it_says_it_is_up() {
    let up = Fake::always(Answer::reply(200, "."));
    assert!(navidrome(&up).answers().await);
    assert_eq!(asked(&up), [(Method::Get, "/ping".to_owned())]);
    for down in [Fake::silent(), Fake::always(Answer::reply(500, ""))] {
        assert!(!navidrome(&down).answers().await);
    }
}

#[tokio::test]
async fn one_session_is_opened_as_the_administrator_and_carried_on_every_read_after() {
    let fake = signed_in_and(vec![(Method::Get, "/api/album", Answer::reply(200, "[]"))]);
    let navidrome = navidrome(&fake);
    assert!(navidrome
        .read::<Vec<()>>("/api/album", "albums")
        .await
        .is_ok());
    assert!(navidrome
        .read::<Vec<()>>("/api/album", "albums")
        .await
        .is_ok());
    assert_eq!(
        asked(&fake),
        [
            (Method::Post, "/auth/login".to_owned()),
            (Method::Get, "/api/album".to_owned()),
            (Method::Get, "/api/album".to_owned()),
        ]
    );
    assert_eq!(
        body_of(&fake, "/auth/login"),
        Some(serde_json::json!({ "username": USERNAME, "password": PASSWORD }))
    );
    assert!(fake
        .requests()
        .iter()
        .filter(|one| one.url.contains("/api/"))
        .all(|one| one
            .headers
            .contains(&("X-ND-Authorization".to_owned(), format!("Bearer {TOKEN}")))));
    assert!(fake
        .requests()
        .iter()
        .filter(|one| one.url.contains("/auth/login"))
        .all(|one| one
            .headers
            .contains(&("Content-Type".to_owned(), "application/json".to_owned()))));
}

#[tokio::test]
async fn a_session_navidrome_refuses_is_opened_again_once() {
    let fake = Fake::by_route_in_turn(vec![
        (
            Method::Post,
            "/auth/login",
            vec![Answer::reply(200, SIGNED_IN)],
        ),
        (
            Method::Get,
            "/api/album",
            vec![Answer::reply(401, "Unauthorized"), Answer::reply(200, "[]")],
        ),
    ]);
    assert!(navidrome(&fake)
        .read::<Vec<()>>("/api/album", "albums")
        .await
        .is_ok());
    assert_eq!(
        asked(&fake),
        [
            (Method::Post, "/auth/login".to_owned()),
            (Method::Get, "/api/album".to_owned()),
            (Method::Post, "/auth/login".to_owned()),
            (Method::Get, "/api/album".to_owned()),
        ]
    );
}

#[tokio::test]
async fn a_session_with_no_token_is_refused() {
    let fake = Fake::always(Answer::reply(
        200,
        r#"{"token":"","username":"q","subsonicToken":"","subsonicSalt":""}"#,
    ));
    assert!(refused_saying(
        navidrome(&fake)
            .read::<Vec<()>>("/api/album", "albums")
            .await,
        "no token"
    ));
}

#[tokio::test]
async fn an_administrator_it_does_not_hold_is_refused_before_anything_is_asked() {
    for (username, password) in [
        (
            Arc::new(Missing) as Arc<dyn lemonfiber_adapter_kit::credential::Holding>,
            Arc::new(PASSWORD.to_owned()) as Arc<dyn lemonfiber_adapter_kit::credential::Holding>,
        ),
        (Arc::new(USERNAME.to_owned()), Arc::new(Missing)),
    ] {
        let fake = Fake::silent();
        let navidrome = Navidrome::new(fake.clone(), BASE, username, password);
        assert!(refused_saying(
            navidrome.read::<Vec<()>>("/api/album", "albums").await,
            "holds no administrator"
        ));
        assert!(fake.requests().is_empty());
    }
}

#[test]
fn an_identifier_is_letters_and_digits_alone() {
    assert!(an_id("7orvCZZyWRqsduCdqXoguY"));
    assert!(an_id("0123456789abcdef0123456789abcdef"));
    assert!(!an_id(""));
    assert!(!an_id(&"a".repeat(65)));
    assert!(!an_id("../../api/user"));
    assert!(!an_id("7orvCZZy?x=1"));
}

#[tokio::test]
async fn a_subsonic_call_is_signed_with_the_session_s_token_and_salt() {
    let fake = signed_in_and(vec![(
        Method::Get,
        "/rest/startScan",
        Answer::reply(200, SCANNING),
    )]);
    assert!(navidrome(&fake).rescan().await.is_ok());
    assert_eq!(
        url_of(&fake, "/rest/startScan").as_deref(),
        Some(
            "http://navidrome:4533/rest/startScan?u=qrstuvwx&t=the-subsonic-token&s=a1b2c3&v=1.16.1&c=lemonfiber&f=json"
        )
    );
}

#[tokio::test]
async fn a_subsonic_call_refused_for_its_password_opens_the_session_again_once() {
    let fake = Fake::by_route_in_turn(vec![
        (
            Method::Post,
            "/auth/login",
            vec![Answer::reply(200, SIGNED_IN)],
        ),
        (
            Method::Get,
            "/rest/startScan",
            vec![
                Answer::reply(200, WRONG_PASSWORD),
                Answer::reply(200, SCANNING),
            ],
        ),
    ]);
    assert!(navidrome(&fake).rescan().await.is_ok());
    assert_eq!(
        asked(&fake),
        [
            (Method::Post, "/auth/login".to_owned()),
            (Method::Get, "/rest/startScan".to_owned()),
            (Method::Post, "/auth/login".to_owned()),
            (Method::Get, "/rest/startScan".to_owned()),
        ]
    );
}

#[tokio::test]
async fn a_subsonic_call_refused_again_is_unauthorised() {
    let fake = signed_in_and(vec![(
        Method::Get,
        "/rest/startScan",
        Answer::reply(200, WRONG_PASSWORD),
    )]);
    assert!(matches!(
        navidrome(&fake).rescan().await,
        Err(Failure::Unauthorised { .. })
    ));
}

#[tokio::test]
async fn a_subsonic_call_refused_otherwise_is_a_refusal_naming_the_code_alone() {
    let fake = signed_in_and(vec![(
        Method::Get,
        "/rest/startScan",
        Answer::reply(200, NOT_AUTHORISED),
    )]);
    let outcome = navidrome(&fake).rescan().await;
    assert!(refused_saying(outcome, "Subsonic error 50"));
    assert_eq!(fake.requests().len(), 2);
}

#[tokio::test]
async fn a_subsonic_answer_neither_ok_nor_an_error_is_refused() {
    let fake = signed_in_and(vec![(
        Method::Get,
        "/rest/startScan",
        Answer::reply(200, r#"{"subsonic-response":{"status":"failed"}}"#),
    )]);
    assert!(refused_saying(
        navidrome(&fake).rescan().await,
        "Navidrome answered failed"
    ));
}

#[tokio::test]
async fn a_subsonic_answer_not_in_the_shape_asked_for_is_refused() {
    let fake = signed_in_and(vec![(
        Method::Get,
        "/rest/getNowPlaying",
        Answer::reply(
            200,
            r#"{"subsonic-response":{"status":"ok","nowPlaying":7}}"#,
        ),
    )]);
    assert!(refused_saying(
        lemonfiber_ports::service::Screening::playing(&navidrome(&fake), None).await,
        "not in the shape expected"
    ));
}

#[test]
fn the_first_run_is_read_from_the_configuration_the_app_s_page_carries() {
    assert_eq!(first_time(FIRST_TIME), Some(true));
    assert_eq!(first_time(SET_UP), Some(false));
    assert_eq!(first_time("<html></html>"), None);
    assert_eq!(first_time("__APP_CONFIG__ = 7"), None);
    assert_eq!(first_time(r#"__APP_CONFIG__ = "{\"version\":1}""#), None);
}

#[tokio::test]
async fn nobody_holds_it_while_its_first_run_is_still_to_come() {
    let fake = Fake::by_route(vec![(Method::Get, "/app/", Answer::reply(200, FIRST_TIME))]);
    assert_eq!(navidrome(&fake).owner().await.ok(), Some(Owner::Open));
    assert_eq!(asked(&fake), [(Method::Get, "/app/".to_owned())]);
}

#[tokio::test]
async fn it_is_ours_where_the_minted_administrator_signs_in() {
    let fake = signed_in_and(vec![(Method::Get, "/app/", Answer::reply(200, SET_UP))]);
    assert_eq!(navidrome(&fake).owner().await.ok(), Some(Owner::Ours));
    assert_eq!(
        asked(&fake),
        [
            (Method::Get, "/app/".to_owned()),
            (Method::Post, "/auth/login".to_owned()),
        ]
    );
}

#[tokio::test]
async fn it_is_theirs_where_the_minted_account_is_refused_or_administers_nothing() {
    for login in [
        Answer::reply(401, r#"{"error":"Invalid username or password"}"#),
        Answer::reply(200, NOT_ADMIN),
    ] {
        let fake = Fake::by_route(vec![
            (Method::Get, "/app/", Answer::reply(200, SET_UP)),
            (Method::Post, "/auth/login", login),
        ]);
        assert_eq!(navidrome(&fake).owner().await.ok(), Some(Owner::Theirs));
    }
}

#[tokio::test]
async fn an_owner_it_cannot_read_is_refused() {
    let unreadable = Fake::always(Answer::reply(200, "<html>"));
    assert!(refused_saying(
        navidrome(&unreadable).owner().await,
        "first administrator could not be read"
    ));
    let failing = Fake::always(Answer::reply(500, ""));
    assert!(refused_saying(
        navidrome(&failing).owner().await,
        "HTTP 500"
    ));
    let login_failing = Fake::by_route(vec![
        (Method::Get, "/app/", Answer::reply(200, SET_UP)),
        (Method::Post, "/auth/login", Answer::reply(500, "")),
    ]);
    assert!(refused_saying(
        navidrome(&login_failing).owner().await,
        "HTTP 500"
    ));
}

#[tokio::test]
async fn a_claim_makes_the_minted_administrator_while_nobody_holds_it_and_keeps_its_session() {
    let fake = Fake::by_route(vec![
        (Method::Get, "/app/", Answer::reply(200, FIRST_TIME)),
        (
            Method::Post,
            "/auth/createAdmin",
            Answer::reply(200, SIGNED_IN),
        ),
        (Method::Get, "/api/album", Answer::reply(200, "[]")),
    ]);
    let navidrome = navidrome(&fake);
    assert_eq!(navidrome.claim().await.ok(), Some(Owner::Ours));
    assert!(navidrome
        .read::<Vec<()>>("/api/album", "albums")
        .await
        .is_ok());
    assert_eq!(
        asked(&fake),
        [
            (Method::Get, "/app/".to_owned()),
            (Method::Post, "/auth/createAdmin".to_owned()),
            (Method::Get, "/api/album".to_owned()),
        ]
    );
    assert_eq!(
        body_of(&fake, "/auth/createAdmin"),
        Some(serde_json::json!({ "username": USERNAME, "password": PASSWORD }))
    );
}

#[tokio::test]
async fn a_claim_whose_administrator_cannot_be_read_is_refused() {
    let fake = Fake::by_route(vec![
        (Method::Get, "/app/", Answer::reply(200, FIRST_TIME)),
        (Method::Post, "/auth/createAdmin", Answer::reply(200, "{}")),
    ]);
    assert!(refused_saying(
        navidrome(&fake).claim().await,
        "administrator Navidrome made could not be read"
    ));
}

#[tokio::test]
async fn a_claim_changes_nothing_somebody_already_holds() {
    let fake = signed_in_and(vec![(Method::Get, "/app/", Answer::reply(200, SET_UP))]);
    assert_eq!(navidrome(&fake).claim().await.ok(), Some(Owner::Ours));
    assert!(!fake.asked_for("/auth/createAdmin"));
}

#[tokio::test]
async fn a_claim_another_first_run_beat_answers_who_holds_it_now() {
    let fake = Fake::by_route_in_turn(vec![
        (
            Method::Get,
            "/app/",
            vec![Answer::reply(200, FIRST_TIME), Answer::reply(200, SET_UP)],
        ),
        (
            Method::Post,
            "/auth/createAdmin",
            vec![Answer::reply(
                403,
                r#"{"error":"Cannot create another first admin"}"#,
            )],
        ),
        (
            Method::Post,
            "/auth/login",
            vec![Answer::reply(
                401,
                r#"{"error":"Invalid username or password"}"#,
            )],
        ),
    ]);
    assert_eq!(navidrome(&fake).claim().await.ok(), Some(Owner::Theirs));
}

#[tokio::test]
async fn a_claim_refused_while_nobody_holds_it_is_a_refusal() {
    let fake = Fake::by_route(vec![
        (Method::Get, "/app/", Answer::reply(200, FIRST_TIME)),
        (Method::Post, "/auth/createAdmin", Answer::reply(500, "")),
    ]);
    assert!(refused_saying(navidrome(&fake).claim().await, "HTTP 500"));
}

#[tokio::test]
async fn it_serves_no_internal_tls_port_and_asks_nothing() {
    let fake = Fake::silent();
    let outcome = navidrome(&fake)
        .serve_tls("/config/internal.p12", "pin")
        .await;
    assert!(matches!(outcome, Err(Failure::Unsupported { .. })));
    assert!(fake.requests().is_empty());
}
