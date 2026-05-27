//! Playlist details page: shows playlist art, track list, and supports editing.

use gtk::prelude::*;
use std::rc::Rc;

use super::PlaylistDetailsModel;

use crate::app::components::{
    Component, DetailsPage, EventListener, HeaderImageShape, Playlist, sync_play_button,
};
use crate::impl_details_component;
use crate::app::dispatch::Worker;
use crate::app::state::{PlaybackEvent, SelectionEvent};
use crate::app::{AppEvent, BrowserEvent};

pub struct PlaylistDetails {
    model: Rc<PlaylistDetailsModel>,
    worker: Worker,
    page: DetailsPage,
    children: Vec<Box<dyn EventListener>>,
}

impl PlaylistDetails {
    pub fn new(model: Rc<PlaylistDetailsModel>, worker: Worker) -> Self {
        if model.get_playlist_info().is_none() {
            model.load_playlist_info();
        }

        let tracks = gtk::ListView::new(None::<gtk::NoSelection>, None::<gtk::ListItemFactory>);
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&tracks);

        let page = DetailsPage::new(HeaderImageShape::Square, &content);
        page.header().set_subtitle_visible(true);

        let playlist = Box::new(Playlist::new(tracks, model.clone(), worker.clone()));

        let headerbar = page.create_headerbar_listener(model.to_headerbar_model());

        // Pagination: load more tracks when scrolled to bottom.
        page.connect_bottom_edge(clone!(
            #[weak]
            model,
            move || { model.load_more_tracks(); }
        ));

        // Clicking the subtitle navigates to the playlist owner's page.
        page.header().connect_subtitle_clicked(clone!(
            #[weak]
            model,
            move || model.view_owner()
        ));

        // Header button: play/pause the playlist.
        page.header().connect_play(clone!(
            #[weak]
            model,
            move || model.toggle_play_playlist()
        ));

        // Header button: like/save the playlist (visibility managed in update_details).
        page.header().connect_liked(clone!(
            #[weak]
            model,
            move || model.toggle_save_playlist()
        ));

        Self {
            model,
            worker,
            page,
            children: vec![playlist, headerbar],
        }
    }

    /// Called when playlist data arrives from the API.
    fn update_details(&self) {
        if let Some(info) = self.model.get_playlist_info() {
            self.page.set_details(&info.title, &info.owner.display_name);
            if self.model.is_playlist_editable() {
                self.page.header().set_like_visible(false);
            } else {
                self.page.header().set_liked(self.model.is_playlist_saved());
            }
            self.page.load_artwork_or_finish(info.art.as_ref(), &self.worker);
        }
    }

    fn update_liked(&self) {
        if !self.model.is_playlist_editable() {
            self.page.header().set_liked(self.model.is_playlist_saved());
        } else {
            self.page.header().set_like_visible(false);
        }
    }

    /// Sync the play button icon with current playback state.
    fn update_playing(&self, is_playing: bool) {
        sync_play_button(&self.page, self.model.playlist_is_playing(), is_playing);
    }

    /// Enter or exit playlist edit mode (rename via header title).
    fn set_editing(&self, editing: bool) {
        if !self.model.is_playlist_editable() {
            return;
        }
        if !editing {
            // Commit or revert the title change.
            let new_name = self.page.header().get_title_text();
            let info = self.model.get_playlist_info();
            if let Some(info) = info {
                if new_name != info.title && !new_name.is_empty() {
                    self.model.update_playlist_details(new_name);
                } else {
                    self.page.header().set_title(&info.title);
                }
            }
        }
    }
}

// --- Component / EventListener wiring ---

impl_details_component!(PlaylistDetails);

impl EventListener for PlaylistDetails {
    fn on_event(&mut self, event: &AppEvent) {
        match event {
            AppEvent::BrowserEvent(BrowserEvent::PlaylistDetailsLoaded(id))
                if id == &self.model.id =>
            {
                self.update_details();
                self.update_playing(true);
            }
            AppEvent::BrowserEvent(BrowserEvent::PlaylistSaved(id))
            | AppEvent::BrowserEvent(BrowserEvent::PlaylistUnsaved(id))
                if id == &self.model.id =>
            {
                self.update_liked();
            }
            AppEvent::PlaybackEvent(PlaybackEvent::PlaybackPaused) => {
                self.update_playing(false);
            }
            AppEvent::PlaybackEvent(PlaybackEvent::PlaybackResumed) => {
                self.update_playing(true);
            }
            AppEvent::SelectionEvent(SelectionEvent::SelectionModeChanged(active)) => {
                self.set_editing(*active);
            }
            _ => {}
        }
        self.broadcast_event(event);
    }
}
