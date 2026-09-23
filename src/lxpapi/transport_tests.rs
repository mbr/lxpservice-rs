//! Exercises wire contracts against isolated loopback servers.

use std::num::NonZeroU64;

use serde_json::Value;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};

use super::{client, Error, LxpApi};
use crate::lxptypes::{ApiMode, Letter, Specification};

/// Serves a single synthetic HTTP exchange and captures the request.
async fn server(response: String) -> (LxpApi, JoinHandle<(String, Value)>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test server");
    let address = listener.local_addr().expect("test server address");
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept request");
        let mut bytes = Vec::new();
        let mut chunk = [0; 4096];
        let (header_end, length) = loop {
            let count = socket.read(&mut chunk).await.expect("read headers");
            assert_ne!(count, 0);
            bytes.extend_from_slice(&chunk[..count]);
            if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..end]);
                let length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().expect("content length"))
                    })
                    .expect("JSON body length");
                break (end + 4, length);
            }
        };
        while bytes.len() < header_end + length {
            let count = socket.read(&mut chunk).await.expect("read body");
            assert_ne!(count, 0);
            bytes.extend_from_slice(&chunk[..count]);
        }
        let headers = String::from_utf8_lossy(&bytes[..header_end]).into_owned();
        let body =
            serde_json::from_slice(&bytes[header_end..header_end + length]).expect("JSON body");
        socket
            .write_all(response.as_bytes())
            .await
            .expect("write response");
        (headers, body)
    });
    let api = LxpApi {
        base_url: format!("http://{address}/v3"),
        username: "dummy-user".into(),
        apikey: "dummy-secret".to_string().into(),
        mode: ApiMode::Test,
        client: client(false).expect("test client"),
    };
    (api, task)
}

/// Builds a synthetic response with a bounded connection lifetime.
fn reply(status: &str, body: &str) -> String {
    format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
}

/// Exercises authenticated GET, encoded POST and body-less DELETE over HTTP.
#[tokio::test]
async fn transport_contract() {
    let (api, task) = server(reply(
        "200 OK",
        r#"{"status":200,"data":{"balance":5,"currency":"EUR"}}"#,
    ))
    .await;
    assert_eq!(api.balance().await.expect("balance response").balance, 5.0);
    let (headers, body) = task.await.expect("server succeeds");
    assert!(headers.starts_with("GET /v3/balance HTTP/1.1"));
    assert_eq!(body["auth"]["apikey"], "dummy-secret");
    assert_eq!(body["auth"]["mode"], "test");
    assert!(body.get("letter").is_none());

    let (api, task) = server(reply(
        "200 OK",
        r#"{"status":200,"data":{"id":7,"status":"draft","items":[]}}"#,
    ))
    .await;
    let letter = Letter::from_pdf(
        b"%PDF-1.7",
        "test.pdf".into(),
        Specification::default(),
        None,
    )
    .expect("valid header");
    assert_eq!(
        api.send(letter, false)
            .await
            .expect("accepted job")
            .id
            .get(),
        7
    );
    let (headers, body) = task.await.expect("server succeeds");
    assert!(headers.starts_with("POST /v3/printjobs HTTP/1.1"));
    assert_eq!(body["auth"]["mode"], "test");
    assert!(body["letter"]["base64_file_checksum"].is_string());

    let (api, task) = server(reply("200 OK", r#"{"status":200,"message":"deleted"}"#)).await;
    api.cancel(NonZeroU64::new(7).expect("positive id"))
        .await
        .expect("cancellation");
    assert!(task
        .await
        .expect("server succeeds")
        .0
        .starts_with("DELETE /v3/printjobs/7 HTTP/1.1"));
}

/// Keeps rejected or undecodable uploads unconfirmed rather than retrying.
#[tokio::test]
async fn unconfirmed_upload_and_redirect() {
    let (api, task) = server(reply("200 OK", "invalid JSON")).await;
    let letter = Letter::from_pdf(
        b"%PDF-1.7",
        "test.pdf".into(),
        Specification::default(),
        None,
    )
    .expect("valid header");
    assert!(matches!(
        api.send(letter, false).await,
        Err(Error::Unconfirmed { .. })
    ));
    task.await.expect("server succeeds");
    let (api, task) = server("HTTP/1.1 307 Temporary Redirect\r\nLocation: https://example.invalid/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()).await;
    assert!(matches!(api.balance().await, Err(Error::Http { status }) if status.as_u16() == 307));
    task.await.expect("server succeeds");
}
