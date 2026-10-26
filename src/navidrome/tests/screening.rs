use lemonfiber_fixtures::http::{Answer, Fake};
use lemonfiber_ports::http::Method;
use lemonfiber_ports::service::{
    Failure, Holds, Image, Item, ItemDetail, Medium, Picture, Playback, Screening as _,
};

use super::super::screening::{a_date, whole_minutes};
use super::{asked, navidrome, refused_saying, signed_in_and, url_of, SIGNED_IN, WRONG_PASSWORD};

const ALBUM: &str = "7orvCZZyWRqsduCdqXoguY";

const PICTURED: &str = r#"{"id":"7orvCZZyWRqsduCdqXoguY","name":"Test Album","maxYear":2021,"date":"2021-05-03","comment":"An album of one tone","duration":3060.0,"songCount":1,"imageHash":"ba31f53919406251","genres":[{"id":"g","name":"Ambient"}]}"#;

const BARE: &str = r#"{"id":"5Bg17dfmhe0faF2LB0cotF","name":"Bare Album","maxYear":0,"date":"2021","comment":" ","songCount":0}"#;

const NOW_PLAYING: &str = r#"{"subsonic-response":{"status":"ok","nowPlaying":{"entry":[{"title":"Tone","album":"Test Album","username":"qrstuvwx","playerName":"lemonfiber"},{"title":"Low","username":"gone","playerName":"phone"}]}}}"#;

const NOTHING_PLAYING: &str = r#"{"subsonic-response":{"status":"ok","nowPlaying":{}}}"#;

const ARTWORK_ABSENT: &str = r#"{"subsonic-response":{"status":"failed","error":{"code":70,"message":"Artwork not found"}}}"#;

const NOT_AUTHORISED: &str = r#"{"subsonic-response":{"status":"failed","error":{"code":50}}}"#;

fn pictured_item() -> Item {
    Item {
        id: ALBUM.to_owned(),
        title: "Test Album".to_owned(),
        year: Some(2021),
        medium: Medium::Other,
        holds: Holds {
            poster: true,
            backdrop: false,
            plays: true,
        },
    }
}

#[tokio::test]
async fn holdings_are_its_albums_by_name_as_many_as_asked_for() {
    let fake = signed_in_and(vec![(
        Method::Get,
        "/api/album",
        Answer::reply(200, format!("[{PICTURED},{BARE}]")),
    )]);
    let held = navidrome(&fake).holdings(None, 1).await;
    assert_eq!(held.ok(), Some(vec![pictured_item()]));
    assert_eq!(
        url_of(&fake, "/api/album").as_deref(),
        Some("http://navidrome:4533/api/album?_start=0&_end=1&_sort=name&_order=ASC")
    );
}

#[tokio::test]
async fn an_album_with_no_picture_year_or_songs_holds_none_of_them() {
    let fake = signed_in_and(vec![(
        Method::Get,
        "/api/album",
        Answer::reply(200, format!("[{BARE}]")),
    )]);
    let held = navidrome(&fake)
        .holdings(None, 10)
        .await
        .unwrap_or_default();
    assert_eq!(
        held.first().map(|item| (item.year, item.holds)),
        Some((None, Holds::default()))
    );
}

#[tokio::test]
async fn a_title_is_one_album_read_whole() {
    let fake = signed_in_and(vec![(
        Method::Get,
        "/api/album/7orvCZZyWRqsduCdqXoguY",
        Answer::reply(200, PICTURED),
    )]);
    assert_eq!(
        navidrome(&fake).title(None, ALBUM).await.ok().flatten(),
        Some(ItemDetail {
            item: pictured_item(),
            overview: Some("An album of one tone".to_owned()),
            minutes: Some(51),
            genres: vec!["Ambient".to_owned()],
            certificate: None,
            released: Some("2021-05-03".to_owned()),
            seasons: Vec::new(),
        })
    );
}

#[tokio::test]
async fn a_title_with_nothing_said_of_it_says_nothing() {
    let fake = signed_in_and(vec![(
        Method::Get,
        "/api/album/5Bg17dfmhe0faF2LB0cotF",
        Answer::reply(200, BARE),
    )]);
    let detail = navidrome(&fake)
        .title(None, "5Bg17dfmhe0faF2LB0cotF")
        .await
        .ok()
        .flatten();
    assert_eq!(
        detail.map(|detail| (
            detail.overview,
            detail.minutes,
            detail.genres,
            detail.released
        )),
        Some((None, None, Vec::new(), None))
    );
}

#[tokio::test]
async fn a_title_it_does_not_hold_is_none() {
    let fake = signed_in_and(vec![(
        Method::Get,
        "/api/album/",
        Answer::reply(404, r#"{"error":"data not found"}"#),
    )]);
    assert_eq!(navidrome(&fake).title(None, ALBUM).await.ok(), Some(None));
    let silent = Fake::silent();
    assert_eq!(
        navidrome(&silent).title(None, "../api/user").await.ok(),
        Some(None)
    );
    assert!(silent.requests().is_empty());
}

#[tokio::test]
async fn a_poster_is_the_album_s_cover_signed_as_subsonic_asks() {
    let fake = signed_in_and(vec![
        (Method::Get, "/api/album/", Answer::reply(200, PICTURED)),
        (
            Method::Get,
            "/rest/getCoverArt",
            Answer::served(200, "image/jpeg", "jpeg-bytes"),
        ),
    ]);
    assert_eq!(
        navidrome(&fake)
            .picture(None, ALBUM, Picture::Poster)
            .await
            .ok()
            .flatten(),
        Some(Image {
            media_type: "image/jpeg".to_owned(),
            bytes: b"jpeg-bytes".to_vec(),
        })
    );
    assert_eq!(
        url_of(&fake, "/rest/getCoverArt").as_deref(),
        Some("http://navidrome:4533/rest/getCoverArt?u=qrstuvwx&t=the-subsonic-token&s=a1b2c3&v=1.16.1&c=lemonfiber&f=json&id=al-7orvCZZyWRqsduCdqXoguY")
    );
}

#[tokio::test]
async fn an_album_with_no_picture_has_no_poster_and_its_placeholder_is_never_asked_for() {
    for album in [Answer::reply(200, BARE), Answer::reply(404, "")] {
        let fake = signed_in_and(vec![(Method::Get, "/api/album/", album)]);
        assert_eq!(
            navidrome(&fake)
                .picture(None, ALBUM, Picture::Poster)
                .await
                .ok(),
            Some(None)
        );
        assert!(!fake.asked_for("/rest/getCoverArt"));
    }
}

#[tokio::test]
async fn a_cover_navidrome_has_not_got_or_serves_as_no_picture_is_none() {
    for cover in [
        Answer::reply(200, ARTWORK_ABSENT),
        Answer::served(200, "text/plain", "nothing"),
    ] {
        let fake = signed_in_and(vec![
            (Method::Get, "/api/album/", Answer::reply(200, PICTURED)),
            (Method::Get, "/rest/getCoverArt", cover),
        ]);
        assert_eq!(
            navidrome(&fake)
                .picture(None, ALBUM, Picture::Poster)
                .await
                .ok(),
            Some(None)
        );
    }
}

#[tokio::test]
async fn a_cover_refused_for_its_password_is_asked_for_again_once_signed_in_afresh() {
    let fake = Fake::by_route_in_turn(vec![
        (
            Method::Post,
            "/auth/login",
            vec![Answer::reply(200, SIGNED_IN)],
        ),
        (
            Method::Get,
            "/api/album/",
            vec![Answer::reply(200, PICTURED)],
        ),
        (
            Method::Get,
            "/rest/getCoverArt",
            vec![
                Answer::reply(200, WRONG_PASSWORD),
                Answer::served(200, "image/png", "png-bytes"),
            ],
        ),
    ]);
    let image = navidrome(&fake)
        .picture(None, ALBUM, Picture::Poster)
        .await
        .ok()
        .flatten();
    assert_eq!(
        image.map(|image| image.media_type),
        Some("image/png".to_owned())
    );
    assert_eq!(
        asked(&fake),
        [
            (Method::Post, "/auth/login".to_owned()),
            (Method::Get, "/api/album/7orvCZZyWRqsduCdqXoguY".to_owned()),
            (Method::Get, "/rest/getCoverArt".to_owned()),
            (Method::Post, "/auth/login".to_owned()),
            (Method::Get, "/rest/getCoverArt".to_owned()),
        ]
    );
}

#[tokio::test]
async fn a_cover_refused_otherwise_is_a_refusal() {
    for (cover, said) in [
        (Answer::reply(200, NOT_AUTHORISED), "Subsonic error 50"),
        (Answer::reply(500, ""), "would not answer the cover"),
    ] {
        let fake = signed_in_and(vec![
            (Method::Get, "/api/album/", Answer::reply(200, PICTURED)),
            (Method::Get, "/rest/getCoverArt", cover),
        ]);
        assert!(refused_saying(
            navidrome(&fake).picture(None, ALBUM, Picture::Poster).await,
            said
        ));
    }
}

#[tokio::test]
async fn a_cover_navidrome_cannot_be_reached_for_is_unavailable() {
    let fake = Fake::by_route(vec![
        (Method::Post, "/auth/login", Answer::reply(200, SIGNED_IN)),
        (Method::Get, "/api/album/", Answer::reply(200, PICTURED)),
    ]);
    assert!(matches!(
        navidrome(&fake).picture(None, ALBUM, Picture::Poster).await,
        Err(Failure::Unavailable { .. })
    ));
}

#[tokio::test]
async fn an_album_holds_no_backdrop() {
    let fake = Fake::silent();
    assert_eq!(
        navidrome(&fake)
            .picture(None, ALBUM, Picture::Backdrop)
            .await
            .ok(),
        Some(None)
    );
    assert!(fake.requests().is_empty());
}

#[tokio::test]
async fn what_is_playing_is_named_by_the_account_and_the_player() {
    let fake = signed_in_and(vec![
        (
            Method::Get,
            "/rest/getNowPlaying",
            Answer::reply(200, NOW_PLAYING),
        ),
        (
            Method::Get,
            "/api/user",
            Answer::reply(200, r#"[{"id":"u1","userName":"qrstuvwx","isAdmin":true}]"#),
        ),
    ]);
    let playing = navidrome(&fake).playing(None).await.ok();
    let played = |member_id: &str, member: &str, title: &str, device: &str| Playback {
        member_id: member_id.to_owned(),
        member: member.to_owned(),
        title: title.to_owned(),
        series: None,
        season: None,
        episode: None,
        medium: Medium::Other,
        paused: false,
        device: device.to_owned(),
    };
    assert_eq!(
        playing,
        Some(vec![
            played("u1", "qrstuvwx", "Tone", "lemonfiber"),
            played("", "gone", "Low", "phone"),
        ])
    );
}

#[tokio::test]
async fn nothing_playing_reads_no_accounts() {
    let fake = signed_in_and(vec![(
        Method::Get,
        "/rest/getNowPlaying",
        Answer::reply(200, NOTHING_PLAYING),
    )]);
    assert_eq!(navidrome(&fake).playing(None).await.ok(), Some(Vec::new()));
    assert!(!fake.asked_for("/api/user"));
}

#[test]
fn a_length_is_whole_minutes_and_a_date_is_a_calendar_day() {
    assert_eq!(whole_minutes(3060.0), Some(51));
    assert_eq!(whole_minutes(f64::NAN), None);
    assert!(a_date("2021-05-03"));
    assert!(!a_date("2021"));
    assert!(!a_date("2021/05/03"));
}
