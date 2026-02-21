use crate::api::models::DiscoveryItem;

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    SwitchSubTab(usize),
    Back,

    FocusSearch,
    SearchSubmit,

    PlayItem(DiscoveryItem),
    TogglePlayPause,
    Stop,
    NextTrack,
    PrevTrack,
    PlaybackStarted { title: String, url: String },
    PlaybackLoading,
    PlaybackFinished,
    PlaybackPosition(f64),
    StreamMetadataChanged(String),

    AddToQueue(DiscoveryItem),
    AddToQueueNext(DiscoveryItem),
    ClearQueue,

    ToggleFavorite,
    AddToHistory(DiscoveryItem),

    LoadNtsLive,
    NtsLiveLoaded(Vec<DiscoveryItem>),
    LoadNtsPicks,
    NtsPicksLoaded(Vec<DiscoveryItem>),

    LoadGenres,
    GenresLoaded(Vec<DiscoveryItem>),
    SearchByGenre { genre_id: String, genre_name: String },
    SearchResultsPartial { search_id: u64, items: Vec<DiscoveryItem>, done: bool },

    FilterList(String),
    ClearFilter,

    VolumeUp,
    VolumeDown,
    VolumeChanged(u8),

    OpenDirectPlay,
    CloseDirectPlay,

    ShowError(String),
    ClearError,
    ShowHelp,
    HideHelp,
    Resize(u16, u16),
    Tick,
}
