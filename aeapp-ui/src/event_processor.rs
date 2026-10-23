use crate::{event_thread, AeAppEguiApp};

pub fn frontend_processor(app: &mut AeAppEguiApp, ctx: &egui::Context) {
    puffin::profile_function!();

    while let Ok(result) = app.backend.rtm_listener.try_recv() {
        match result {
            event_thread::AeAppEventResponse::FriendStatusResponse(res) => {
                for friend in &mut app.friends {
                    if friend.id != res.id {
                        continue;
                    }

                    friend.online = res.presence.basic().clone();
                    if res.presence.game().is_some() {
                        friend.game = Some(res.presence.status().clone());
                    }
                }
                ctx.request_repaint();
            }
        }
    }
}
