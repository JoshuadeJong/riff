mod details_header;
mod details_page;

pub use details_header::*;
pub use details_page::*;

/// Size (in pixels) used to fetch and display the header artwork.
pub(super) const HEADER_IMAGE_SIZE: i32 = 200;

/// Update the play button on a DetailsPage header based on playback state.
pub fn sync_play_button(page: &DetailsPage, source_is_playing: bool, is_playing: bool) {
    page.header().set_playing(source_is_playing && is_playing);
}

/// Generates the standard [`Component`] impl for a detail page struct
/// that has `page: DetailsPage` and `children: Vec<Box<dyn EventListener>>` fields.
#[macro_export]
macro_rules! impl_details_component {
    ($type:ty) => {
        impl $crate::app::components::Component for $type {
            fn get_root_widget(&self) -> &gtk::Widget {
                self.page.widget().upcast_ref()
            }
            fn get_children(
                &mut self,
            ) -> Option<&mut Vec<Box<dyn $crate::app::components::EventListener>>> {
                Some(&mut self.children)
            }
        }
    };
}

pub fn expose_widgets() {
    details_header::expose_widgets();
}
