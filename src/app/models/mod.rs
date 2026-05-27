// Domain models
mod main;
pub use main::*;

// UI models (GObject)
mod songs;
pub use songs::*;

mod card_model;
pub use card_model::*;

impl From<&AlbumDescription> for CardModel {
    fn from(album: &AlbumDescription) -> Self {
        CardModel::new(&album.id, album.art.as_ref().and_then(|s| s.best_for_width(200)).map(str::to_owned).as_ref(), &album.title, &album.artists_name())
    }
}

impl From<AlbumDescription> for CardModel {
    fn from(album: AlbumDescription) -> Self {
        Self::from(&album)
    }
}

impl From<&PlaylistDescription> for CardModel {
    fn from(playlist: &PlaylistDescription) -> Self {
        CardModel::new(
            &playlist.id,
            playlist.art.as_ref().and_then(|s| s.best_for_width(200)).map(str::to_owned).as_ref(),
            &playlist.title,
            &playlist.owner.display_name,
        )
    }
}

impl From<PlaylistDescription> for PlaylistSummary {
    fn from(PlaylistDescription { id, title, .. }: PlaylistDescription) -> Self {
        Self { id, title }
    }
}

impl From<PlaylistDescription> for CardModel {
    fn from(playlist: PlaylistDescription) -> Self {
        Self::from(&playlist)
    }
}

impl From<SongDescription> for SongModel {
    fn from(song: SongDescription) -> Self {
        SongModel::new(song)
    }
}

impl From<&SongDescription> for SongModel {
    fn from(song: &SongDescription) -> Self {
        SongModel::new(song.clone())
    }
}

impl From<&ArtistSummary> for CardModel {
    fn from(artist: &ArtistSummary) -> Self {
        CardModel::new(&artist.id, artist.photo.as_ref().and_then(|s| s.best_for_width(200)).map(str::to_owned).as_ref(), &artist.name, "")
    }
}
