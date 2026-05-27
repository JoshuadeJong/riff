use gettextrs::gettext;
use gtk::prelude::*;
use std::rc::Rc;

use super::NowPlayingModel;
use crate::app::components::{
    Component, DetailsPage, DeviceSelector, DeviceSelectorWidget, EventListener,
    HeaderImageShape, Playlist, sync_play_button,
};
use crate::app::dispatch::Worker;
use crate::app::state::PlaybackEvent;
use crate::app::{AppEvent, BrowserEvent};
use crate::feature_flags::{self, FeatureFlag};
use crate::impl_details_component;  

pub struct NowPlaying {
    model: Rc<NowPlayingModel>,
    worker: Worker,
    page: DetailsPage,
    children: Vec<Box<dyn EventListener>>,
}

impl NowPlaying {
    pub fn new(model: Rc<NowPlayingModel>, worker: Worker) -> Self {
        let tracks = gtk::ListView::new(None::<gtk::NoSelection>, None::<gtk::ListItemFactory>);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let queue_label = gtk::Label::builder()
            .label(&gettext("Queue"))
            .halign(gtk::Align::Start)
            .css_classes(["title-4"])
            .build();
        content.append(&queue_label);

        tracks.set_margin_top(12);
        content.append(&tracks);

        let page = DetailsPage::new(HeaderImageShape::Square, &content);
        page.set_loaded();
        page.header().set_caption("Now playing");
        page.header().set_caption_visible(true);
        page.header().set_subtitle_visible(true);

        page.header().connect_play(clone!(
            #[weak]
            model,
            move || model.toggle_play()
        ));

        page.header().connect_liked(clone!(
            #[weak]
            model,
            move || model.toggle_like()
        ));

        page.header().connect_info(clone!(
            #[weak]
            model,
            move || model.view_album()
        ));

        page.connect_bottom_edge(clone!(
            #[weak]
            model,
            move || { model.load_more(); }
        ));

        let playlist = Box::new(Playlist::new(tracks, model.clone(), worker.clone()));
        let headerbar = page.create_headerbar_listener(model.to_headerbar_model());

        let mut children: Vec<Box<dyn EventListener>> = vec![playlist, headerbar];

        if feature_flags::is_enabled(FeatureFlag::DeviceSelector) {
            let ds_widget: DeviceSelectorWidget = glib::Object::new();
            if let Some(hb) = page.headerbar() {
                hb.pack_end(&ds_widget);
            }
            let device_selector = Box::new(DeviceSelector::new(
                ds_widget,
                model.device_selector_model(),
            ));
            children.push(device_selector);
        }

        let np = Self {
            model,
            worker,
            page,
            children,
        };
        np.update_details();
        np
    }

    fn update_details(&self) {
        if let Some(song) = self.model.current_song() {
            self.page.set_details(&song.title, &song.artists_name());
            self.page.header().set_liked(self.model.is_current_song_liked());
            sync_play_button(&self.page, true, self.model.is_playing());
            self.page.load_artwork_or_finish(song.art.as_ref(), &self.worker);
        } else {
            self.page.set_details("", "");
            self.page.set_loaded();
        }
    }
}

impl_details_component!(NowPlaying);

impl EventListener for NowPlaying {
    fn on_event(&mut self, event: &AppEvent) {
        match event {
            AppEvent::PlaybackEvent(PlaybackEvent::TrackChanged(_)) => {
                self.model.load_more();
                self.update_details();
            }
            AppEvent::PlaybackEvent(PlaybackEvent::PlaybackPaused) => {
                sync_play_button(&self.page, true, false);
            }
            AppEvent::PlaybackEvent(PlaybackEvent::PlaybackResumed) => {
                sync_play_button(&self.page, true, true);
            }
            AppEvent::BrowserEvent(BrowserEvent::SavedTracksUpdated) => {
                self.page.header().set_liked(self.model.is_current_song_liked());
            }
            _ => {}
        }
        self.broadcast_event(event);
    }
}
