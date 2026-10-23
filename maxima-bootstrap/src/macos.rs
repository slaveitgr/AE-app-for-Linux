use cacao::appkit::{App, AppDelegate};
use url::Url;

use crate::{handle_launch_args, run};

pub struct MaximaBootstrapApp {
    rt: tokio::runtime::Handle,
}

impl MaximaBootstrapApp {
    pub fn new(rt: tokio::runtime::Handle) -> Self {
        Self { rt }
    }
}

impl AppDelegate for MaximaBootstrapApp {
    fn did_finish_launching(&self) {
        self.rt.spawn(async {
            if let Ok(true) = handle_launch_args().await {
                App::terminate();
            }
        });
    }

    fn open_urls(&self, urls: Vec<Url>) {
        self.rt.spawn(async move {
            let _ = run(&urls.iter().map(|u| u.to_string()).collect::<Vec<String>>()).await;
            App::terminate();
        });
    }
}
