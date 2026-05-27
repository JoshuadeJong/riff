//! Artist details page: shows artist photo, top tracks, and album releases.

use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use std::rc::Rc;

use super::ArtistDetailsModel;

use crate::app::components::{
    CardList, Component, DetailsPage, EventListener, HeaderImageShape, Playlist,
};
use crate::impl_details_component;
use crate::app::{AppEvent, BrowserEvent, Worker};

// --- Content Widget (Blueprint template) ---
// This is the scrollable content area below the header.
// Defined in details_artist.blp: a vertical Box with top_tracks ListView and releases FlowBox.

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/dev/diegovsky/Riff/components/details_artist.ui")]
    pub struct ArtistDetailsContentWidget {
        #[template_child]
        pub top_tracks: TemplateChild<gtk::ListView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ArtistDetailsContentWidget {
        const NAME: &'static str = "ArtistDetailsContentWidget";
        type Type = super::ArtistDetailsContentWidget;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ArtistDetailsContentWidget {}
    impl WidgetImpl for ArtistDetailsContentWidget {}
    impl BoxImpl for ArtistDetailsContentWidget {}
}

glib::wrapper! {
    pub struct ArtistDetailsContentWidget(ObjectSubclass<imp::ArtistDetailsContentWidget>) @extends gtk::Widget, gtk::Box;
}

impl ArtistDetailsContentWidget {
    fn new() -> Self {
        glib::Object::new()
    }
}

// --- Artist Details Page ---

pub struct ArtistDetails {
    model: Rc<ArtistDetailsModel>,
    worker: Worker,
    page: DetailsPage,
    /// Kept alive to maintain the FlowBox ↔ model binding for album cards.
    _card_list: CardList,
    children: Vec<Box<dyn EventListener>>,
}

impl ArtistDetails {
    pub fn new(model: Rc<ArtistDetailsModel>, worker: Worker) -> Self {
        // Kick off the API request immediately.
        model.load_artist_details(model.id.clone());

        let content = ArtistDetailsContentWidget::new();
        let page = DetailsPage::new(HeaderImageShape::Circle, &content);

        // Header button: toggle follow/unfollow artist.
        page.header().connect_liked(clone!(
            #[weak]
            model,
            move || model.toggle_follow_artist()
        ));

        // Pagination: load more album releases when scrolled to bottom.
        page.connect_bottom_edge(clone!(
            #[weak]
            model,
            move || { model.load_more(); }
        ));

        // Bind the releases FlowBox to the album card list model.
        let card_list = CardList::new();
        content.append(card_list.widget());
        card_list.bind(&model, worker.clone());

        // Bind the top tracks ListView to the playlist/song model.
        let playlist = Box::new(Playlist::new(
            content.imp().top_tracks.get(),
            Rc::clone(&model),
            worker.clone(),
        ));

        let headerbar = page.create_headerbar_listener(model.to_headerbar_model());

        Self {
            model,
            worker,
            page,
            _card_list: card_list,
            children: vec![playlist, headerbar],
        }
    }

    /// Called when artist data arrives from the API.
    fn update_details(&self) {
        if let Some(name) = self.model.get_artist_name() {
            self.page.set_details(&name, "");
        }
        self.page.header().set_liked(self.model.is_followed());
        let photo = self.model.get_artist_photo();
        self.page.load_artwork_or_finish(photo.as_deref(), &self.worker);
    }
}

// --- Component / EventListener wiring ---

impl_details_component!(ArtistDetails);

impl EventListener for ArtistDetails {
    fn on_event(&mut self, event: &AppEvent) {
        if let AppEvent::BrowserEvent(BrowserEvent::ArtistDetailsUpdated(id)) = event {
            if id == &self.model.id {
                self.update_details();
            }
        }
        self.broadcast_event(event);
    }
}

pub fn expose_widgets() {
    ArtistDetailsContentWidget::static_type();
}
