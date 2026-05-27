use gio::prelude::*;
use gio::SimpleActionGroup;
use std::ops::Deref;
use std::rc::Rc;

use crate::app::components::{labels, HeaderBarModel, PlaylistModel, SimpleHeaderBarModel, SimpleHeaderBarModelWrapper};
use crate::app::models::*;
use crate::app::state::SelectionContext;
use crate::app::state::{PlaybackAction, SelectionAction, SelectionState};
use crate::app::{ActionDispatcher, AppAction, AppEvent, AppModel, BatchQuery, BrowserAction, PaginationTarget, SongsSource};
use crate::feature_flags::{self, FeatureFlag};

pub struct SavedTracksModel {
    app_model: Rc<AppModel>,
    dispatcher: Box<dyn ActionDispatcher>,
}

impl SavedTracksModel {
    pub fn new(app_model: Rc<AppModel>, dispatcher: Box<dyn ActionDispatcher>) -> Self {
        Self {
            app_model,
            dispatcher,
        }
    }

    pub fn load_initial(&self) {
        let loader = self.app_model.get_batch_loader();
        let query = BatchQuery {
            source: SongsSource::SavedTracks,
            batch: Batch::first_of_size(50),
        };
        self.dispatcher.dispatch_async(Box::pin(async move {
            loader
                .query(query, |_s, song_batch| {
                    BrowserAction::SetSavedTracks(Box::new(song_batch)).into()
                })
                .await
        }));
    }

    pub fn load_more(&self) -> Option<()> {
        let api = self.app_model.get_spotify();

        let state = self.app_model.get_state();
        let next_page = state.browser.home_state()?.next_saved_tracks_page.clone();
        drop(state);

        let batch_size = next_page.batch_size;
        let offset = next_page.next_offset?;

        self.app_model
            .update_state(BrowserAction::ConsumeNextPage(PaginationTarget::SavedTracks).into());

        self.dispatcher
            .call_spotify_and_dispatch(move || async move {
                api.get_saved_tracks(offset, batch_size)
                    .await
                    .map(|song_batch| BrowserAction::AppendSavedTracks(Box::new(song_batch)).into())
            });

        Some(())
    }

    pub fn saved_tracks_is_playing(&self) -> bool {
        matches!(
            self.app_model.get_state().playback.current_source(),
            Some(SongsSource::SavedTracks)
        )
    }

    pub fn toggle_play_saved_tracks(&self) {
        if !self.saved_tracks_is_playing() {
            if self.app_model.get_state().playback.is_shuffled() {
                self.dispatcher
                    .dispatch(PlaybackAction::ToggleShuffle.into());
            }
            let first_song = self.song_list_model().index(0);
            if let Some(first_song) = first_song {
                self.play_song_at(0, &first_song.get_id());
            }
        } else if self.app_model.get_state().playback.is_playing() {
            self.dispatcher.dispatch(PlaybackAction::Pause.into());
        } else {
            self.dispatcher.dispatch(PlaybackAction::Play.into());
        }
    }

    pub fn to_headerbar_model(self: &Rc<Self>) -> Rc<impl HeaderBarModel> {
        Rc::new(SimpleHeaderBarModelWrapper::new(
            self.clone(),
            self.app_model.clone(),
            self.dispatcher.box_clone(),
        ))
    }
}

impl SimpleHeaderBarModel for SavedTracksModel {
    fn title(&self) -> Option<String> {
        Some("All Tracks".to_string())
    }

    fn title_updated(&self, _: &AppEvent) -> bool {
        false
    }

    fn selection_context(&self) -> Option<SelectionContext> {
        if !feature_flags::is_enabled(FeatureFlag::SelectMode) {
            return None;
        }
        Some(SelectionContext::SavedTracks)
    }

    fn select_all(&self) {
        let songs: Vec<SongDescription> = self.song_list_model().collect();
        self.dispatcher
            .dispatch(SelectionAction::Select(songs).into());
    }
}

impl PlaylistModel for SavedTracksModel {
    fn song_list_model(&self) -> SongListModel {
        self.app_model
            .get_state()
            .browser
            .home_state()
            .expect("illegal attempt to read home_state")
            .saved_tracks
            .clone()
    }

    fn is_paused(&self) -> bool {
        !self.app_model.get_state().playback.is_playing()
    }

    fn current_song_id(&self) -> Option<String> {
        self.app_model.get_state().playback.current_song_id()
    }

    fn play_song_at(&self, pos: usize, id: &str) {
        let source = SongsSource::SavedTracks;
        let batch = self.song_list_model().song_batch_for(pos);
        if let Some(batch) = batch {
            self.dispatcher
                .dispatch(PlaybackAction::LoadPagedSongs(source, batch).into());
            self.dispatcher
                .dispatch(PlaybackAction::Load(id.to_string()).into());
        }
    }
    fn autoscroll_to_playing(&self) -> bool {
        true
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

        Some(group.upcast())
    }

    fn menu_for(&self, id: &str) -> Option<gio::MenuModel> {
        let song = self.song_list_model().get(id)?;
        let song = song.description();

        let menu = gio::Menu::new();
        menu.append(Some(&*labels::VIEW_ALBUM), Some("song.view_album"));
        for artist in song.artists.iter() {
            menu.append(
                Some(&labels::more_from_label(&artist.name)),
                Some(&format!("song.view_artist_{}", artist.id)),
            );
        }

        menu.append(Some(&*labels::COPY_LINK), Some("song.copy_link"));

        Some(menu.upcast())
    }

    fn select_song(&self, id: &str) {
        let song = self.song_list_model().get(id);
        if let Some(song) = song {
            self.dispatcher
                .dispatch(SelectionAction::Select(vec![song.description().clone()]).into());
        }
    }

    fn deselect_song(&self, id: &str) {
        self.dispatcher
            .dispatch(SelectionAction::Deselect(vec![id.to_string()]).into());
    }

    fn enable_selection(&self) -> bool {
        if !feature_flags::is_enabled(FeatureFlag::SelectMode) {
            return false;
        }
        self.dispatcher
            .dispatch(AppAction::EnableSelection(SelectionContext::SavedTracks));
        true
    }

    fn selection(&self) -> Option<Box<dyn Deref<Target = SelectionState> + '_>> {
        let selection = self.app_model.map_state(|s| &s.selection);
        Some(Box::new(selection))
    }
}
