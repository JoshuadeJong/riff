#[allow(clippy::module_inception)]
mod details_user;
pub use details_user::*;

mod details_user_model;
pub use details_user_model::*;

pub fn expose_widgets() {
    details_user::expose_widgets();
}
