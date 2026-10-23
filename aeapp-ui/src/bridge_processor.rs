use crate::{
    bridge_thread::{self, BackendError},
    views::downloads_view::QueuedDownload,
    AeAppEguiApp, BackendStallState, GameDetails, GameDetailsWrapper,
};
use log::{error, info, warn};
use std::sync::mpsc::TryRecvError;

pub fn frontend_processor(app: &mut AeAppEguiApp, ctx: &egui::Context) {
    puffin::profile_function!();

    if app.critical_error.is_some() {
        return;
    }

    'outer: loop {
        match app.backend.backend_listener.try_recv() {
            Ok(result) => {
                use bridge_thread::AeAppLibResponse::*;
                match result {
                    LoginResponse(res) => {
                        let res = match res {
                            Ok(response) => response,
                            Err(error) => {
                                warn!("Login failed. {}", error);
                                app.nonfatal_errors.push(*error);
                                app.backend_state = BackendStallState::UserNeedsToLogIn;
                                ctx.request_repaint();
                                continue;
                            }
                        };

                        info!("Logged in as {}!", &res.you.display_name());
                        app.user_name = res.you.display_name().clone();
                        app.user_id = res.you.id().clone();
                        app.backend_state = BackendStallState::BingChilling;
                        app.backend
                            .backend_commander
                            .send(bridge_thread::AeAppLibRequest::GetGamesRequest)
                            .unwrap();
                        app.backend
                            .backend_commander
                            .send(bridge_thread::AeAppLibRequest::GetFriendsRequest)
                            .unwrap();
                    }
                    LoginCacheEmpty => app.backend_state = BackendStallState::UserNeedsToLogIn,
                    ServiceNeedsStarting => {
                        app.backend_state = BackendStallState::UserNeedsToInstallService
                    }
                    ServiceStarted => app.backend_state = BackendStallState::Starting,
                    LibraryReset => {
                        app.games.clear();
                        app.library_details = false;
                    }
                    GameInfoResponse(res) => {
                        app.games.insert(res.game.slug.clone(), res.game);
                    }
                    GameDetailsResponse(res) => {
                        let response = res.response;

                        for (slug, game) in &mut app.games {
                            if !slug.eq(&res.slug) {
                                continue;
                            }

                            game.details = GameDetailsWrapper::Available(GameDetails {
                                time: response.time,
                                achievements_unlocked: response.achievements_unlocked,
                                achievements_total: response.achievements_total,
                                path: response.path.clone(),
                                system_requirements_min: response.system_requirements_min.clone(),
                                system_requirements_rec: response.system_requirements_rec.clone(),
                            });
                        }
                    }
                    FriendInfoResponse(res) => app.friends.push(res.friend),
                    CriticalError(err) => app.critical_error = Some(*err),
                    NonFatalError(err) => app.nonfatal_errors.push(*err),
                    ActiveGameChanged(slug) => app.playing_game = slug,
                    LocateGameResponse(res) => {
                        app.installer_state.locate_response = Some(res);
                        app.installer_state.locating = false;
                    }
                    DownloadProgressChanged(offer_id, progress) => {
                        if let Some(dl_ing) = app.installing_now.as_mut() {
                            if dl_ing.offer == offer_id {
                                dl_ing.downloaded_bytes = progress.bytes;
                                dl_ing.total_bytes = progress.bytes_total;
                            }
                        }
                    }
                    DownloadFinished(_) => {
                        // Re-read the real manifest/registry state so the completed game can be played.
                        let _ = app
                            .backend
                            .backend_commander
                            .send(bridge_thread::AeAppLibRequest::GetGamesRequest);
                    }
                    DownloadStatusChanged(status) => {
                        if let Some(current) = app.installing_now.as_mut() {
                            if status.offer_id.as_deref() == Some(current.offer.as_str()) {
                                current.downloaded_bytes = status.bytes;
                                current.total_bytes = status.total;
                            }
                        }
                        crate::views::downloads_view::apply_status(ctx, status);
                    }
                    DownloadQueueUpdate(current, queue) => {
                        ctx.data_mut(|data| {
                            data.insert_temp(
                                crate::views::downloads_view::order_key(),
                                queue.clone(),
                            )
                        });
                        if let Some(current) = current {
                            if !app.installing_now.as_ref().is_some_and(|n| n.offer == current) {
                                app.installing_now = Some(QueuedDownload {
                                    slug: {
                                        // This sucks!
                                        let mut rtn: String = String::new();
                                        for (slug, game) in &app.games {
                                            if game.offer.eq(&current) {
                                                // "but it's less code in the nest"
                                                // "WHO CARES"
                                                // (it was the same amount overall)
                                                rtn = slug.to_string();
                                                break;
                                            }
                                        }
                                        rtn
                                    },
                                    offer: current,
                                    downloaded_bytes: 0,
                                    total_bytes: 0,
                                })
                            }
                        } else {
                            app.installing_now = None;
                        }

                        app.install_queue.clear();
                        for offer in queue {
                            let i_fucking_hate_this = QueuedDownload {
                                slug: {
                                    let mut rtn: String = String::new();
                                    for (slug, game) in &app.games {
                                        if game.offer.eq(&offer) {
                                            rtn = slug.to_string();
                                            break;
                                        }
                                    }
                                    rtn
                                },
                                offer: offer.clone(),
                                downloaded_bytes: 0,
                                total_bytes: 0,
                            };
                            app.install_queue.insert(offer, i_fucking_hate_this);
                        }
                    }
                }
                ctx.request_repaint();
            }
            Err(variant) => {
                match variant {
                    TryRecvError::Empty => {}
                    TryRecvError::Disconnected => {
                        app.critical_error = Some(BackendError::ChannelDisconnected);
                    }
                }
                break 'outer;
            }
        }
    }
}
