#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//extern crate windows_service;

use std::env::current_exe;
use std::error::Error;
use std::string::FromUtf8Error;
use thiserror::Error;
use tokio::process::Command;

use aeapp::core::launch::BootstrapLaunchArgs;
use aeapp::util::native::NativeError;
#[cfg(windows)]
use aeapp::util::service::{is_service_valid, register_service};
use aeapp::util::BackgroundServiceControlError;
use base64::{engine::general_purpose, Engine};
use url::Url;

#[cfg(target_os = "macos")]
mod macos;

#[derive(Error, Debug)]
pub(crate) enum RunError {
    #[error(transparent)]
    BackgroundService(#[from] BackgroundServiceControlError),
    #[error(transparent)]
    Base64(#[from] base64::DecodeError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Native(#[from] NativeError),
    #[error(transparent)]
    ParseUrl(#[from] url::ParseError),
    #[error(transparent)]
    ParseUtf8(#[from] FromUtf8Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("invalid login callback")]
    InvalidCallback,
}

#[cfg(not(target_os = "macos"))]
#[tokio::main]
async fn main() -> Result<(), RunError> {
    let _ = handle_launch_args().await?;

    Ok(())
}

#[cfg(target_os = "macos")]
#[tokio::main]
async fn main() -> Result<()> {
    use cacao::appkit::App;

    use crate::macos::AeAppBootstrapApp;

    let handle = tokio::runtime::Handle::current();
    App::new(
        "io.github.slaveitgr.aeapp.bootstrap",
        AeAppBootstrapApp::new(handle),
    )
    .run();

    Ok(())
}

async fn handle_launch_args() -> Result<bool, RunError> {
    let mut args: Vec<String> = std::env::args().collect();
    args.remove(0);

    let result = run(&args).await;
    if cfg!(debug_assertions) || std::env::var("AEAPP_DEBUG").is_ok() {
        if args.iter().any(|arg| arg.starts_with("qrc:")) {
            println!("Args: [login callback redacted]");
        } else {
            println!("Args: {:?}", &args);
        }

        let str_result = result
            .as_ref()
            .map_err(|e| {
                let source = e.source();
                let error_str = if source.is_some() {
                    source.unwrap().to_string()
                } else {
                    e.to_string()
                };

                error_str
            })
            .err()
            .unwrap_or("Success".to_string());
        println!("Result: {}", str_result);

        // Pause terminal
        //std::io::Read::read(&mut std::io::stdin(), &mut [0]).unwrap();
    }

    result
}

#[cfg(windows)]
fn service_setup() -> Result<(), BackgroundServiceControlError> {
    if is_service_valid()? {
        return Ok(());
    }

    register_service()?;

    Ok(())
}

#[cfg(not(windows))]
fn service_setup() -> Result<(), BackgroundServiceControlError> {
    Ok(())
}

#[cfg(windows)]
async fn platform_launch(args: BootstrapLaunchArgs) -> Result<(), NativeError> {
    let mut binding = Command::new(args.path);
    let child = binding.args(args.args);

    let status = child.spawn()?.wait().await?;
    // bail!("{}", status.code().unwrap());
    Ok(())
}

#[cfg(unix)]
async fn platform_launch(args: BootstrapLaunchArgs) -> Result<(), NativeError> {
    use aeapp::unix::wine::run_wine_command_with_options;
    use aeapp::unix::wine::CommandType;

    run_wine_command_with_options(
        args.path,
        Some(args.args),
        None,
        false,
        CommandType::WaitForExitAndRun,
        &args.proton,
    )
    .await?;

    Ok(())
}

async fn run(args: &[String]) -> Result<bool, RunError> {
    let len = args.len();
    if len == 1 {
        let arg = &args[0];

        if arg == "--noop" {
            return Ok(true);
        }

        if arg.starts_with("link2ea") {
            todo!();
        }

        if arg.starts_with("origin2") {
            let url = Url::parse(arg)?;
            let query = querystring::querify(url.query().unwrap());
            let _offer_id = query.iter().find(|(x, _)| *x == "offerIds").unwrap().1;
            let cmd_params = query.iter().find(|(x, _)| *x == "cmdParams").unwrap().1;

            let mut child = Command::new(current_exe()?.with_file_name("aeapp-cli.exe"));
            child.env(
                "AEAPP_LAUNCH_ARGS",
                urlencoding::decode(cmd_params)?
                    .into_owned()
                    .replace("\\\"", "\""),
            );
            println!(
                "{}",
                urlencoding::decode(cmd_params)?
                    .into_owned()
                    .replace("\\\"", "\"")
            );
            child.env("KYBER_INTERFACE_PORT", "3005");
            child.args(["--mode", "launch", "--offer-id", "Origin.OFR.50.0002148"]);
            child.spawn()?.wait().await?;

            return Ok(true);
        }

        if arg.starts_with("qrc:") {
            let callback = oauth_callback_url(arg)?;
            reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(10))
                .build()?
                .get(callback)
                .send()
                .await
                .map_err(|error| RunError::Reqwest(error.without_url()))?
                .error_for_status()
                .map_err(|error| RunError::Reqwest(error.without_url()))?;
            return Ok(true);
        }

        return Ok(false);
    }

    if len > 1 {
        let command = &args[0];
        let handled = match command.as_str() {
            "launch" => {
                let decoded = general_purpose::STANDARD.decode(&args[1])?;
                let launch_args: BootstrapLaunchArgs = serde_json::from_slice(&decoded)?;
                platform_launch(launch_args).await?;

                true
            }
            _ => false,
        };
        return Ok(handled);
    }

    service_setup()?;

    Ok(false)
}

fn oauth_callback_url(value: &str) -> Result<Url, RunError> {
    let source = Url::parse(value).map_err(|_| RunError::InvalidCallback)?;
    if source.scheme() != "qrc"
        || source.host_str().is_some()
        || source.path() != "/html/login_successful.html"
    {
        return Err(RunError::InvalidCallback);
    }
    let mut fields: Vec<_> = source
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    if let Some(fragment) = source.fragment() {
        fields.extend(
            url::form_urlencoded::parse(fragment.as_bytes())
                .map(|(key, value)| (key.into_owned(), value.into_owned())),
        );
    }
    let mut callbacks = fields
        .iter()
        .filter(|(key, _)| key == "code" || key == "error");
    let (key, value) = callbacks.next().ok_or(RunError::InvalidCallback)?;
    if callbacks.next().is_some() || value.is_empty() || value.len() > 4096 {
        return Err(RunError::InvalidCallback);
    }
    let mut target = Url::parse("http://127.0.0.1:31033/auth")?;
    target.query_pairs_mut().append_pair(key, value);
    Ok(target)
}

#[cfg(test)]
mod callback_tests {
    use super::*;
    #[test]
    fn callback_decodes_once_and_forwards_only_to_loopback() {
        for source in [
            "qrc:///html/login_successful.html?code=a%2Bb%2Fc%3D",
            "qrc:/html/login_successful.html#code=a%2Bb%2Fc%3D",
        ] {
            let target = oauth_callback_url(source).unwrap();
            assert_eq!(target.host_str(), Some("127.0.0.1"));
            assert_eq!(target.port(), Some(31033));
            assert_eq!(
                target.query_pairs().collect::<Vec<_>>(),
                vec![("code".into(), "a+b/c=".into())]
            );
        }
        assert_eq!(
            oauth_callback_url("qrc:/html/login_successful.html?code=%252B")
                .unwrap()
                .query_pairs()
                .next()
                .unwrap()
                .1,
            "%2B"
        );
    }
    #[test]
    fn callback_rejects_invalid_and_ambiguous_uris_without_exposing_codes() {
        for source in [
            "qrcx:/html/login_successful.html?code=secret",
            "qrc://outside/html/login_successful.html?code=secret",
            "qrc:/other?code=secret",
            "qrc:/html/login_successful.html?code=",
            "qrc:/html/login_successful.html?code=one&code=two",
            "qrc:/html/login_successful.html?code=one#code=two",
        ] {
            assert_eq!(
                oauth_callback_url(source).unwrap_err().to_string(),
                "invalid login callback"
            );
        }
    }
}
