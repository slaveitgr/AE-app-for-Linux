use crate::{
    translation_manager::TranslationManager,
    views::{
        friends_view::{FriendsViewBarPage, FriendsViewBarStatusFilter},
        game_view::{GameViewBarGenre, GameViewBarPlatform},
    },
    FrontendLanguage,
};

pub trait EnumToString<T> {
    fn get_string(&self, variant: &mut T) -> &str;
    fn get_string_nonmut(&self, variant: &T) -> &str;
}

impl EnumToString<FriendsViewBarPage> for TranslationManager {
    fn get_string_nonmut(&self, variant: &FriendsViewBarPage) -> &str {
        match variant {
            FriendsViewBarPage::Online => &self.localization.friends_view.toolbar.online,
            FriendsViewBarPage::All => &self.localization.friends_view.toolbar.all,
            FriendsViewBarPage::Pending => &self.localization.friends_view.toolbar.pending,
            FriendsViewBarPage::Blocked => &self.localization.friends_view.toolbar.blocked,
        }
    }
    fn get_string(&self, variant: &mut FriendsViewBarPage) -> &str {
        self.get_string_nonmut(variant)
    }
}

impl EnumToString<FriendsViewBarStatusFilter> for TranslationManager {
    fn get_string_nonmut(&self, variant: &FriendsViewBarStatusFilter) -> &str {
        let locale = &self.localization.friends_view.toolbar.filter_options;
        match variant {
            FriendsViewBarStatusFilter::Name => &locale.name,
            FriendsViewBarStatusFilter::Game => &locale.game,
        }
    }
    fn get_string(&self, variant: &mut FriendsViewBarStatusFilter) -> &str {
        self.get_string_nonmut(variant)
    }
}

impl EnumToString<GameViewBarGenre> for TranslationManager {
    fn get_string_nonmut(&self, variant: &GameViewBarGenre) -> &str {
        let locale = &self.localization.games_view.toolbar.genre_options;
        match variant {
            GameViewBarGenre::AllGames => &locale.all,
            GameViewBarGenre::Shooters => &locale.shooter,
            GameViewBarGenre::Simulation => &locale.simulation,
        }
    }
    fn get_string(&self, variant: &mut GameViewBarGenre) -> &str {
        self.get_string_nonmut(variant)
    }
}

impl EnumToString<GameViewBarPlatform> for TranslationManager {
    fn get_string_nonmut(&self, variant: &GameViewBarPlatform) -> &str {
        let locale = &self.localization.games_view.toolbar.platform_options;
        match variant {
            GameViewBarPlatform::AllPlatforms => &locale.all,
            GameViewBarPlatform::Windows => &locale.windows,
            GameViewBarPlatform::Mac => &locale.mac,
        }
    }
    fn get_string(&self, variant: &mut GameViewBarPlatform) -> &str {
        self.get_string_nonmut(variant)
    }
}

impl EnumToString<FrontendLanguage> for TranslationManager {
    fn get_string_nonmut(&self, variant: &FrontendLanguage) -> &str {
        match variant {
            FrontendLanguage::SystemDefault => &self.localization.locale.default,
            FrontendLanguage::EnUS => &self.localization.locale.en_us,
        }
    }
    fn get_string(&self, variant: &mut FrontendLanguage) -> &str {
        self.get_string_nonmut(variant)
    }
}
