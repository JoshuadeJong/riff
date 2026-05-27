//! Album details page: shows album art, track list, and release info.

use gtk::prelude::*;
use libadwaita::prelude::AdwDialogExt;
use std::rc::Rc;

use super::release_details::ReleaseDetailsDialog;
use super::DetailsModel;

use crate::app::components::{
    Component, DetailsPage, EventListener, HeaderImageShape, Playlist, sync_play_button,
};
use crate::impl_details_component;
use crate::app::dispatch::Worker;
use crate::app::state::PlaybackEvent;
use crate::app::{AppEvent, BrowserEvent};

pub struct Details {
    model: Rc<DetailsModel>,
    worker: Worker,
    page: DetailsPage,
    /// Dialog showing label, release date, copyright, etc.
    modal: ReleaseDetailsDialog,
    children: Vec<Box<dyn EventListener>>,
}

impl Details {
    pub fn new(model: Rc<DetailsModel>, worker: Worker) -> Self {
        if model.get_album_info().is_none() {
            model.load_album_info();
        }

        let tracks = gtk::ListView::new(None::<gtk::NoSelection>, None::<gtk::ListItemFactory>);
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&tracks);

        let page = DetailsPage::new(HeaderImageShape::Square, &content);
        page.header().set_subtitle_visible(true);

        // Pagination: load more tracks when scrolled to bottom.
        page.connect_bottom_edge(clone!(
            #[weak]
            model,
            move || { model.load_more(); }
        ));

        let playlist = Box::new(Playlist::new(tracks, model.clone(), worker.clone()));

        let headerbar = page.create_headerbar_listener(model.to_headerbar_model());

        let modal = ReleaseDetailsDialog::new();

        // Header button: toggle "liked" (saved to library).
        page.header().connect_liked(clone!(
            #[weak]
            model,
            move || model.toggle_save_album()
        ));

        // Header button: play/pause the album.
        page.header().connect_play(clone!(
            #[weak]
            model,
            move || model.toggle_play_album()
        ));

        // Header button: open release details dialog.
        page.header().connect_info(clone!(
            #[weak]
            modal,
            #[weak(rename_to = widget)]
            page.widget(),
            move || {
                let modal = modal.upcast_ref::<libadwaita::Dialog>();
                let parent = widget.root().and_then(|r| r.downcast::<gtk::Window>().ok());
                modal.present(parent.as_ref());
            }
        ));

        Self {
            model,
            worker,
            page,
            modal,
            children: vec![playlist, headerbar],
        }
    }

    fn update_liked(&self) {
        if let Some(info) = self.model.get_album_info() {
            self.page.header().set_liked(info.description.is_liked);
        }
    }

    /// Sync the play button icon with current playback state.
    fn update_playing(&self, is_playing: bool) {
        sync_play_button(&self.page, self.model.album_is_playing(), is_playing);
    }

    /// Called when album data arrives from the API.
    fn update_details(&self) {
        if let Some(album) = self.model.get_album_info() {
            let details = &album.release_details;
            let album = &album.description;

            self.page.header().set_liked(album.is_liked);
            self.page.set_details(&album.title, &album.artists_name());

            // Clicking the subtitle navigates to the artist page.
            self.page.header().connect_subtitle_clicked(clone!(
                #[weak(rename_to = model)]
                self.model,
                move || model.view_artist()
            ));

            self.modal.set_details(
                &album.title,
                &album.artists_name(),
                &details.label,
                album.release_date.as_ref().unwrap(),
                details.total_tracks,
                &details.copyright_text,
            );

            self.page.load_artwork_or_finish(album.art.as_ref(), &self.worker);
        }
    }
}

// --- Component / EventListener wiring ---

impl_details_component!(Details);

impl EventListener for Details {
    fn on_event(&mut self, event: &AppEvent) {
        match event {
            AppEvent::BrowserEvent(BrowserEvent::AlbumDetailsLoaded(id))
                if id == &self.model.id =>
            {
                self.update_details();
                self.update_playing(true);
            }
            AppEvent::BrowserEvent(BrowserEvent::AlbumSaved(id))
            | AppEvent::BrowserEvent(BrowserEvent::AlbumUnsaved(id))
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
            _ => {}
        }
        self.broadcast_event(event);
    }
}
