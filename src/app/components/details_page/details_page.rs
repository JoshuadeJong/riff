use std::rc::Rc;

use gtk::prelude::*;
use gtk::subclass::prelude::*;
use libadwaita::prelude::*;

use crate::app::components::{
    display_add_css_provider, EventListener, HeaderBarComponent, HeaderBarModel, HeaderBarWidget,
    ScrollingHeaderWidget,
};
use crate::app::dispatch::Worker;
use crate::app::loader::ImageLoader;
use crate::app::models::ImageSet;

use super::{DetailsHeader, HeaderImageShape, HEADER_IMAGE_SIZE};

/// Maximum width (in pixels) for both the header and content clamps, keeping them aligned.
const CLAMP_MAX_SIZE: i32 = 1600;

// =============================================================================
// DetailsPage
// =============================================================================

/// A reusable details page layout used by album, artist, and playlist views.
///
/// Structure (top to bottom):
///   ┌─────────────────────────────┐
///   │ HeaderBarWidget (collapses) │  ← shows title when header scrolls away
///   ├─────────────────────────────┤
///   │ ScrollingHeaderWidget       │
///   │  ├─ Revealer (header area)  │  ← artwork + title + buttons
///   │  └─ ScrolledWindow (body)   │  ← caller-provided content (track list, etc.)
///   └─────────────────────────────┘
pub struct DetailsPage {
    widget: libadwaita::Bin,
    scrolling_header: ScrollingHeaderWidget,
    headerbar: Option<HeaderBarWidget>,
    header: DetailsHeader,
}

impl DetailsPage {
    fn load_css() {
        display_add_css_provider(resource!("/components/details_page.css"));
    }

    /// Build a new details page.
    ///
    /// - `shape`: controls whether the header artwork is square (albums) or circular (artists).
    /// - `content`: the main body widget (e.g. a track list) placed below the header.
    pub fn new(shape: HeaderImageShape, content: &impl IsA<gtk::Widget>) -> Self {
        Self::load_css();

        // --- Headerbar (top bar that shows title when header is scrolled out of view) ---
        let headerbar = HeaderBarWidget::new();
        headerbar.add_classes(&["details__headerbar"]);

        // --- Header (artwork + title + action buttons) ---
        let header = DetailsHeader::new(shape);
        header.widget().add_css_class("details-header");
        header.widget().set_hexpand(true);

        let header_clamp = libadwaita::Clamp::new();
        header_clamp.set_maximum_size(CLAMP_MAX_SIZE);
        header_clamp.set_tightening_threshold(CLAMP_MAX_SIZE);
        header_clamp.set_child(Some(header.widget()));
        header_clamp.add_css_class("details-header-clamp");

        // WindowHandle allows dragging the window from the header area.
        let window_handle = gtk::WindowHandle::new();
        window_handle.set_child(Some(&header_clamp));

        // --- Content (caller-provided body, e.g. track list) ---
        content.upcast_ref::<gtk::Widget>().set_hexpand(true);

        let content_clamp = libadwaita::Clamp::new();
        content_clamp.set_maximum_size(CLAMP_MAX_SIZE);
        content_clamp.set_tightening_threshold(CLAMP_MAX_SIZE);
        content_clamp.set_child(Some(content));
        content_clamp.add_css_class("details-content-clamp");

        // --- Scrolling header (manages reveal/hide of the header on scroll) ---
        let scrolling_header = ScrollingHeaderWidget::new();
        scrolling_header.revealer().set_child(Some(&window_handle));
        scrolling_header
            .scrolled_window()
            .set_child(Some(&content_clamp));
        scrolling_header.add_css_class("container");
        scrolling_header.add_css_class("details-page");

        // --- Assemble the page ---
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        vbox.set_vexpand(true);
        vbox.set_hexpand(true);
        vbox.append(headerbar.upcast_ref::<gtk::Widget>());
        vbox.append(scrolling_header.upcast_ref::<gtk::Widget>());

        let bin = libadwaita::Bin::new();
        bin.set_child(Some(&vbox));

        let page = Self {
            widget: bin,
            scrolling_header,
            headerbar: Some(headerbar),
            header,
        };
        page.connect_header_collapse();
        page
    }

    // =========================================================================
    // Accessors
    // =========================================================================

    pub fn widget(&self) -> &libadwaita::Bin {
        &self.widget
    }

    pub fn header(&self) -> &DetailsHeader {
        &self.header
    }

    pub fn headerbar(&self) -> Option<&HeaderBarWidget> {
        self.headerbar.as_ref()
    }

    /// Create a [`HeaderBarComponent`] bound to this page's headerbar widget.
    /// The returned listener should be added to the page's children.
    pub fn create_headerbar_listener(&self, model: Rc<impl HeaderBarModel + 'static>) -> Box<dyn EventListener> {
        Box::new(HeaderBarComponent::new(
            self.headerbar().unwrap().clone(),
            model,
        ))
    }

    // =========================================================================
    // Content updates
    // =========================================================================

    /// Set title and subtitle on both the header widget and the collapsed headerbar.
    pub fn set_details(&self, title: &str, subtitle: &str) {
        self.header.set_title(title);
        self.header.set_subtitle(subtitle);
        if let Some(ref headerbar) = self.headerbar {
            headerbar.set_title_and_subtitle(title, subtitle);
        }
    }

    /// Asynchronously load artwork from an ImageSet, or mark the page as loaded if none.
    pub fn load_artwork_or_finish(&self, art: Option<&ImageSet>, worker: &Worker) {
        if let Some(url) = art.and_then(|s| s.best_for_width(HEADER_IMAGE_SIZE as u32)) {
            let url = url.to_string();
            let weak_header = self.header.widget_weak();
            let weak = self.scrolling_header.downgrade();
            worker.send_local_task(async move {
                let pixbuf = ImageLoader::new()
                    .load_remote(&url, "jpg", HEADER_IMAGE_SIZE, HEADER_IMAGE_SIZE)
                    .await;
                if let (Some(sh), Some(ref pixbuf)) = (weak.upgrade(), pixbuf) {
                    if let Some(header) = weak_header.upgrade() {
                        let texture = gdk::Texture::for_pixbuf(pixbuf);
                        header.imp().image.set_paintable(Some(&texture));
                        header.imp().image_box.remove_css_class("details-header__image-placeholder");
                    }
                    sh.add_css_class("container--loaded");
                }
            });
        } else {
            self.set_loaded();
        }
    }

    /// Mark the page as loaded (triggers CSS transition out of skeleton/loading state).
    pub fn set_loaded(&self) {
        self.scrolling_header.add_css_class("container--loaded");
    }

    // =========================================================================
    // Scroll callbacks
    // =========================================================================

    /// Connect a callback for when the user scrolls to the bottom (used for pagination).
    pub fn connect_bottom_edge<F: Fn() + 'static>(&self, f: F) {
        self.scrolling_header.connect_bottom_edge(f);
    }

    // =========================================================================
    // Internal wiring
    // =========================================================================

    /// When the header scrolls out of view, reveal the title in the headerbar
    /// and remove the "flat" (transparent) style so it gets a solid background.
    fn connect_header_collapse(&self) {
        if let Some(ref headerbar) = self.headerbar {
            headerbar.set_title_visible(false);
            headerbar.add_classes(&["flat"]);
            self.scrolling_header.connect_header_visibility(clone!(
                #[weak]
                headerbar,
                move |header_visible| {
                    headerbar.set_title_visible(!header_visible);
                    if header_visible {
                        headerbar.add_classes(&["flat"]);
                    } else {
                        headerbar.remove_classes(&["flat"]);
                    }
                }
            ));
        }
    }
}
