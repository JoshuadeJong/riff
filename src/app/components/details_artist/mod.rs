#[allow(clippy::module_inception)]
mod details_artist;
pub use details_artist::*;

mod details_artist_model;
pub use details_artist_model::*;

pub fn expose_widgets() {
    details_artist::expose_widgets();
}
