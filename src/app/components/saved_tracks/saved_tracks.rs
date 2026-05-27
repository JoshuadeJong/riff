use gettextrs::ngettext;
use gtk::prelude::*;
use std::rc::Rc;

use super::SavedTracksModel;
use crate::app::components::{
    Component, DetailsPage, EventListener, HeaderImageShape, Playlist, PlaylistModel, sync_play_button,
};
use crate::app::state::{LoginEvent, PlaybackEvent};
use crate::app::{AppEvent, BrowserEvent, Worker};
use crate::impl_details_component;

pub struct SavedTracks {
    model: Rc<SavedTracksModel>,
    page: DetailsPage,
    children: Vec<Box<dyn EventListener>>,
}

impl SavedTracks {
    pub fn new(model: Rc<SavedTracksModel>, worker: Worker) -> Self {
        let tracks = gtk::ListView::new(None::<gtk::NoSelection>, None::<gtk::ListItemFactory>);
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&tracks);

        let page = DetailsPage::new(HeaderImageShape::Square, &content);
        page.set_loaded();
        page.set_details("All Tracks", "");
        page.header().set_subtitle_visible(true);
        page.header().set_default_icon("emote-love-symbolic");

        page.header().connect_play(clone!(
            #[weak]
            model,
            move || model.toggle_play_saved_tracks()
        ));

        page.connect_bottom_edge(clone!(
            #[weak]
            model,
            move || { model.load_more(); }
        ));

        let playlist = Box::new(Playlist::new(tracks, model.clone(), worker));
        let headerbar = page.create_headerbar_listener(model.to_headerbar_model());

        Self {
            model,
            page,
            children: vec![playlist, headerbar],
        }
    }

    fn update_track_count(&self) {
        let count = self.model.song_list_model().len();
        let subtitle = ngettext!("{} Track", "{} Tracks", count as u32, count);
        self.page.header().set_subtitle(&subtitle);
    }

    fn update_playing(&self, is_playing: bool) {
        sync_play_button(&self.page, self.model.saved_tracks_is_playing(), is_playing);
    }
}

impl_details_component!(SavedTracks);

impl EventListener for SavedTracks {
    fn on_event(&mut self, event: &AppEvent) {
        match event {
            AppEvent::LoginEvent(LoginEvent::LoginCompleted) => {
                self.model.load_initial();
            }
            AppEvent::BrowserEvent(BrowserEvent::SavedTracksUpdated) => {
                self.update_track_count();
            }
            AppEvent::PlaybackEvent(PlaybackEvent::PlaybackPaused) => {
                self.update_playing(false);
            }
            AppEvent::PlaybackEvent(PlaybackEvent::PlaybackResumed) => {
                self.update_playing(true);
            }
            _ => {}
        }
        self.broadcast_event(event);
    }
}
