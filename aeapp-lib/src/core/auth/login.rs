use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use super::context::AuthContext;
use crate::core::{auth::storage::AuthError, clients::JUNO_PC_CLIENT_ID};

fn callback_fields(line: &str) -> Option<Vec<(String, String)>> {
    let mut fields = line.split_whitespace();
    if fields.next()? != "GET" {
        return None;
    }
    let target = fields.next()?;
    if !matches!(fields.next()?, "HTTP/1.0" | "HTTP/1.1") || fields.next().is_some() {
        return None;
    }
    if !target.starts_with("/auth?") {
        return None;
    }
    let url = reqwest::Url::parse(&format!("http://127.0.0.1{target}")).ok()?;
    if url.path() != "/auth" {
        return None;
    }
    Some(
        url.query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect(),
    )
}

fn callback_code(line: &str) -> Option<String> {
    let fields = callback_fields(line)?;
    let mut callbacks = fields
        .into_iter()
        .filter(|(key, _)| key == "code" || key == "error");
    let (key, code) = callbacks.next()?;
    if key != "code" || callbacks.next().is_some() {
        return None;
    }
    (!code.is_empty() && code.len() <= 4096).then_some(code)
}

fn callback_denied(line: &str) -> bool {
    let Some(fields) = callback_fields(line) else {
        return false;
    };
    let mut callbacks = fields
        .into_iter()
        .filter(|(key, _)| key == "code" || key == "error");
    let Some((key, value)) = callbacks.next() else {
        return false;
    };
    key == "error" && !value.is_empty() && callbacks.next().is_none()
}

async fn receive_oauth_code(listener: TcpListener) -> Result<String, AuthError> {
    loop {
        let (mut socket, _) = listener.accept().await?;
        let (read, mut write) = socket.split();
        let mut reader = BufReader::new(read.take(8192));
        let mut line = String::new();
        match tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut line)).await {
            Ok(Ok(length)) if length > 0 && line.ends_with('\n') => {}
            _ => continue,
        }
        let code = callback_code(&line);
        let denied = callback_denied(&line);
        let (status, body) = if code.is_some() {
            ("200 OK", "You can return to aeapp.")
        } else {
            ("400 Bad Request", "Invalid login callback.")
        };
        let response = format!("HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        let _ = tokio::time::timeout(Duration::from_secs(5), write.write_all(response.as_bytes()))
            .await;
        if let Some(code) = code {
            return Ok(code);
        }
        if denied {
            return Err(AuthError::NoAuthCode);
        }
    }
}

pub async fn begin_oauth_login_flow<'a>(context: &mut AuthContext<'a>) -> Result<(), AuthError> {
    // Be ready before the browser can redirect a cached session back to the client.
    let listener = TcpListener::bind("127.0.0.1:31033").await?;
    open::that(context.nucleus_auth_url(JUNO_PC_CLIENT_ID, "code")?)?;
    let code = receive_oauth_code(listener).await?;
    context.set_code(&code);
    Ok(())
}

#[deprecated(note = "This method of login was patched and this function will be removed soon")]
pub async fn manual_login(_persona: &str, _password: &str) -> Result<String, AuthError> {
    unimplemented!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpStream;

    #[test]
    fn callback_requires_exact_path_method_and_nonempty_code() {
        assert_eq!(
            callback_code("GET /auth?code=local%2Bfixture HTTP/1.1\r\n"),
            Some("local+fixture".into())
        );
        for line in [
            "GET /auth?code= HTTP/1.1",
            "GET /auth-other?code=x HTTP/1.1",
            "POST /auth?code=x HTTP/1.1",
            "GET http://outside/auth?code=x HTTP/1.1",
            "GET /auth?error=denied HTTP/1.1",
            "GET /auth?code=one&code=two HTTP/1.1",
        ] {
            assert_eq!(callback_code(line), None);
        }
    }

    #[test]
    fn denied_login_is_recognized_only_on_the_callback_route() {
        assert!(callback_denied("GET /auth?error=access_denied HTTP/1.1"));
        assert!(!callback_denied("GET /other?error=access_denied HTTP/1.1"));
        assert!(!callback_denied(
            "GET /auth?code=x&error=access_denied HTTP/1.1"
        ));
    }

    #[tokio::test]
    async fn callback_acknowledges_http_and_ignores_unrelated_requests() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let worker = tokio::spawn(receive_oauth_code(listener));
        let mut unrelated = TcpStream::connect(address).await.unwrap();
        unrelated
            .write_all(b"GET /favicon.ico HTTP/1.1\r\n\r\n")
            .await
            .unwrap();
        let mut response = Vec::new();
        unrelated.read_to_end(&mut response).await.unwrap();
        assert!(response.starts_with(b"HTTP/1.1 400"));
        let mut callback = TcpStream::connect(address).await.unwrap();
        callback
            .write_all(b"GET /auth?code=offline-fixture HTTP/1.1\r\n\r\n")
            .await
            .unwrap();
        response.clear();
        callback.read_to_end(&mut response).await.unwrap();
        assert!(response.starts_with(b"HTTP/1.1 200 OK\r\n"));
        assert!(response.ends_with(b"You can return to aeapp."));
        assert_eq!(worker.await.unwrap().unwrap(), "offline-fixture");
    }
}
