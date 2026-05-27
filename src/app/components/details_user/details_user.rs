//! User details page: shows a user's public playlists.

use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use std::rc::Rc;

use super::UserDetailsModel;

use crate::app::components::{
    CardList, Component, DetailsPage, EventListener, HeaderImageShape,
};
use crate::impl_details_component;
use crate::app::{AppEvent, BrowserEvent, Worker};

// --- Content Widget (Blueprint template) ---
// Defined in details_user.blp: a vertical Box with a FlowBox of playlist cards.

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/dev/diegovsky/Riff/components/details_user.ui")]
    pub struct UserDetailsContentWidget {
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UserDetailsContentWidget {
        const NAME: &'static str = "UserDetailsContentWidget";
        type Type = super::UserDetailsContentWidget;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for UserDetailsContentWidget {}
    impl WidgetImpl for UserDetailsContentWidget {}
    impl BoxImpl for UserDetailsContentWidget {}
}

glib::wrapper! {
    pub struct UserDetailsContentWidget(ObjectSubclass<imp::UserDetailsContentWidget>) @extends gtk::Widget, gtk::Box;
}

impl UserDetailsContentWidget {
    fn new() -> Self {
        glib::Object::new()
    }
}

// --- User Details Page ---

pub struct UserDetails {
    model: Rc<UserDetailsModel>,
    worker: Worker,
    page: DetailsPage,
    /// Kept alive to maintain the FlowBox ↔ model binding for playlist cards.
    _card_list: CardList,
    children: Vec<Box<dyn EventListener>>,
}

impl UserDetails {
    pub fn new(model: UserDetailsModel, worker: Worker) -> Self {
        model.load_user_details(model.id.clone());
        let model = Rc::new(model);

        let content = UserDetailsContentWidget::new();
        let page = DetailsPage::new(HeaderImageShape::Circle, &content);
        page.header().set_caption_visible(true);

        // Pagination: load more playlists when scrolled to bottom.
        page.connect_bottom_edge(clone!(
            #[weak]
            model,
            move || { model.load_more(); }
        ));

        // Bind the FlowBox to the playlist card list model.
        let card_list = CardList::new();
        content.append(card_list.widget());
        card_list.bind(&model, worker.clone());

        let headerbar = page.create_headerbar_listener(model.to_headerbar_model());

        Self {
            model,
            worker,
            page,
            _card_list: card_list,
            children: vec![headerbar],
        }
    }

    /// Called when user data arrives from the API.
    fn update_details(&self) {
        if let Some(name) = self.model.get_user_name() {
            self.page.header().set_caption("Profile");
            self.page.header().set_title(&name);
        }
        let photo = self.model.get_user_photo();
        if photo.is_none() {
            self.page.header().set_default_icon("avatar-default-symbolic");
        }
        self.page.load_artwork_or_finish(photo.as_deref(), &self.worker);
    }
}

// --- Component / EventListener wiring ---

impl_details_component!(UserDetails);

impl EventListener for UserDetails {
    fn on_event(&mut self, event: &AppEvent) {
        if let AppEvent::BrowserEvent(BrowserEvent::UserDetailsUpdated(id)) = event {
            if id == &self.model.id {
                self.update_details();
            }
        }
        self.broadcast_event(event);
    }
}

pub fn expose_widgets() {
    UserDetailsContentWidget::static_type();
}
