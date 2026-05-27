use gio::prelude::*;
use gio::SimpleActionGroup;
use std::ops::Deref;
use std::rc::Rc;

use crate::api::SpotifyApiError;
use crate::app::components::SimpleHeaderBarModel;
use crate::app::components::{labels, CardListModel, ImageShape, PlaylistModel, SimpleHeaderBarModelWrapper, HeaderBarModel};
use crate::app::models::*;
use crate::app::state::SelectionContext;
use crate::app::state::{
    BrowserAction, BrowserEvent, PlaybackAction, PaginationTarget, SelectionAction, SelectionState,
};
use crate::app::{ActionDispatcher, AppAction, AppEvent, AppModel, ListStore};

pub struct ArtistDetailsModel {
    pub id: String,
    app_model: Rc<AppModel>,
    dispatcher: Box<dyn ActionDispatcher>,
}

impl ArtistDetailsModel {
    pub fn new(id: String, app_model: Rc<AppModel>, dispatcher: Box<dyn ActionDispatcher>) -> Self {
        Self {
            id,
            app_model,
            dispatcher,
        }
    }

    pub fn get_artist_name(&self) -> Option<impl Deref<Target = String> + '_> {
        self.app_model
            .map_state_opt(|s| s.browser.artist_state(&self.id)?.artist.as_ref())
    }

    pub fn get_artist_photo(&self) -> Option<impl Deref<Target = ImageSet> + '_> {
        self.app_model
            .map_state_opt(|s| s.browser.artist_state(&self.id)?.photo.as_ref())
    }

    pub fn get_list_store(&self) -> Option<impl Deref<Target = ListStore<CardModel>> + '_> {
        self.app_model
            .map_state_opt(|s| Some(&s.browser.artist_state(&self.id)?.albums))
    }

    pub fn load_artist_details(&self, id: String) {
        let api = self.app_model.get_spotify();
        self.dispatcher
            .call_spotify_and_dispatch(move || async move {
                let artist = api.get_artist(&id).await;
                match artist {
                    Ok(artist) => Ok(BrowserAction::SetArtistDetails(Box::new(artist)).into()),
                    Err(SpotifyApiError::BadStatus(400, _))
                    | Err(SpotifyApiError::BadStatus(404, _)) => {
                        Ok(BrowserAction::NavigationPop.into())
                    }
                    Err(e) => Err(e),
                }
            });
    }

    pub fn open_album(&self, id: String) {
        self.dispatcher.dispatch(AppAction::ViewAlbum(id));
    }

    pub fn is_followed(&self) -> bool {
        self.app_model
            .get_state()
            .browser
            .artist_state(&self.id)
            .map(|s| s.is_followed)
            .unwrap_or(false)
    }

    pub fn toggle_follow_artist(&self) {
        let id = self.id.clone();
        let is_followed = self.is_followed();
        let api = self.app_model.get_spotify();

        self.dispatcher
            .call_spotify_and_dispatch(move || async move {
                if is_followed {
                    api.unfollow_artist(&id).await?;
                    Ok(BrowserAction::UnfollowArtist(id).into())
                } else {
                    api.follow_artist(&id).await?;
                    Ok(BrowserAction::FollowArtist(id).into())
                }
            });
    }

    pub fn load_more(&self) -> Option<()> {
        let api = self.app_model.get_spotify();
        let state = self.app_model.get_state();
        let next_page = state.browser.artist_state(&self.id)?.next_page.clone();
        drop(state);

        let id = next_page.data;
        let batch_size = next_page.batch_size;
        let offset = next_page.next_offset?;

        self.app_model
            .update_state(BrowserAction::ConsumeNextPage(PaginationTarget::ArtistReleases(id.clone())).into());

        self.dispatcher
            .call_spotify_and_dispatch(move || async move {
                api.get_artist_albums(&id, offset, batch_size)
                    .await
                    .map(|albums| BrowserAction::AppendArtistReleases(id, albums).into())
            });

        Some(())
    }

    pub fn to_headerbar_model(self: &Rc<Self>) -> Rc<impl HeaderBarModel> {
        Rc::new(SimpleHeaderBarModelWrapper::new(
            self.clone(),
            self.app_model.clone(),
            self.dispatcher.box_clone(),
        ))
    }
}

impl CardListModel for ArtistDetailsModel {
    fn get_store(&self) -> Option<impl Deref<Target = ListStore<CardModel>> + '_> {
        self.get_list_store()
    }

    fn load_more(&self) {
        let _ = ArtistDetailsModel::load_more(self);
    }

    fn refresh(&self) {}

    fn has_items(&self) -> bool {
        self.get_list_store().map(|s| s.len() > 0).unwrap_or(false)
    }

    fn open_item(&self, id: String) {
        self.open_album(id);
    }

    fn image_shape(&self) -> ImageShape {
        ImageShape::Square
    }
}

impl PlaylistModel for ArtistDetailsModel {
    fn song_list_model(&self) -> SongListModel {
        self.app_model
            .get_state()
            .browser
            .artist_state(&self.id)
            .expect("illegal attempt to read artist_state")
            .top_tracks
            .clone()
    }

    fn is_paused(&self) -> bool {
        !self.app_model.get_state().playback.is_playing()
    }

    fn current_song_id(&self) -> Option<String> {
        self.app_model.get_state().playback.current_song_id()
    }

    fn play_song_at(&self, _pos: usize, id: &str) {
        let tracks: Vec<SongDescription> = self.song_list_model().collect();
        self.dispatcher
            .dispatch(PlaybackAction::LoadSongs(tracks).into());
        self.dispatcher
            .dispatch(PlaybackAction::Load(id.to_string()).into());
    }

    fn actions_for(&self, id: &str) -> Option<gio::ActionGroup> {
        let song = self.song_list_model().get(id)?;
        let song = song.description();

        let group = SimpleActionGroup::new();

        for view_artist in song.make_artist_actions(self.dispatcher.box_clone(), None) {
            group.add_action(&view_artist);
        }
        group.add_action(&song.make_album_action(self.dispatcher.box_clone(), None));
        group.add_action(&song.make_link_action(None));
        group.add_action(&song.make_queue_action(self.dispatcher.box_clone(), None));

        Some(group.upcast())
    }

    fn menu_for(&self, id: &str) -> Option<gio::MenuModel> {
        let song = self.song_list_model().get(id)?;
        let song = song.description();

        let menu = gio::Menu::new();
        menu.append(Some(&*labels::VIEW_ALBUM), Some("song.view_album"));
        for artist in song.artists.iter().filter(|a| self.id != a.id) {
            menu.append(
                Some(&labels::more_from_label(&artist.name)),
                Some(&format!("song.view_artist_{}", artist.id)),
            );
        }

        menu.append(Some(&*labels::COPY_LINK), Some("song.copy_link"));
        menu.append(Some(&*labels::ADD_TO_QUEUE), Some("song.queue"));
        Some(menu.upcast())
    }

    fn select_song(&self, id: &str) {
        let song = self.song_list_model().get(id);
        if let Some(song) = song {
            self.dispatcher
                .dispatch(SelectionAction::Select(vec![song.into_description()]).into());
        }
    }

    fn deselect_song(&self, id: &str) {
        self.dispatcher
            .dispatch(SelectionAction::Deselect(vec![id.to_string()]).into());
    }

    fn enable_selection(&self) -> bool {
        self.dispatcher
            .dispatch(AppAction::EnableSelection(SelectionContext::Default));
        true
    }

    fn selection(&self) -> Option<Box<dyn Deref<Target = SelectionState> + '_>> {
        Some(Box::new(self.app_model.map_state(|s| &s.selection)))
    }
}

impl SimpleHeaderBarModel for ArtistDetailsModel {
    fn title(&self) -> Option<String> {
        Some(self.get_artist_name()?.clone())
    }

    fn title_updated(&self, event: &AppEvent) -> bool {
        matches!(
            event,
            AppEvent::BrowserEvent(BrowserEvent::ArtistDetailsUpdated(_))
        )
    }

    fn selection_context(&self) -> Option<SelectionContext> {
        None
    }

    fn select_all(&self) {}
}
