use gio::prelude::SettingsExt;

const SETTINGS: &str = "dev.diegovsky.Riff";

#[derive(Clone, Copy, Debug)]
pub enum FeatureFlag {
    /*
     Selection mode allows users to select multiple songs to queue, save, or remove.
     It has visually bugged buttons in the page's header across all pages that use it.
     */
    SelectMode,
    /*
     Creating new playlists workflow needs to be flushed out further before launching. Currently,
     users can create a new play list with no songs but interacting with the playlist is awkward.
     Furthermore, it is possible to crash the application by viewing a newly created playlist and
     then viewing another playlist in the same session.
     */
    CreateNewPlaylist,
    /*
     Device selector allows switching playback between Spotify Connect devices.
     */
    DeviceSelector,
}

impl FeatureFlag {
    pub const ALL: &[FeatureFlag] = &[
        FeatureFlag::SelectMode,
        FeatureFlag::CreateNewPlaylist,
        FeatureFlag::DeviceSelector,
    ];

    pub fn key(&self) -> &'static str {
        match self {
            FeatureFlag::SelectMode => "feature-select-mode",
            FeatureFlag::CreateNewPlaylist => "feature-create-new-playlist",
            FeatureFlag::DeviceSelector => "feature-device-selector",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            FeatureFlag::SelectMode => "Select Mode",
            FeatureFlag::CreateNewPlaylist => "Create New Playlist",
            FeatureFlag::DeviceSelector => "Device Selector",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            FeatureFlag::SelectMode => {
                "Enable selection mode to select multiple songs for queuing, saving, or removing."
            }
            FeatureFlag::CreateNewPlaylist => {
                "Enable the New Playlist button in the sidebar."
            }
            FeatureFlag::DeviceSelector => {
                "Enable the device selector in the Now Playing headerbar."
            }
        }
    }
}

pub fn is_enabled(flag: FeatureFlag) -> bool {
    let settings = gio::Settings::new(SETTINGS);
    settings.boolean(flag.key())
}
