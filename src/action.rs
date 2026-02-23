// Every user interaction, async result, and internal event is represented as an
// Action variant. The App event loop dispatches these to component handlers.

use crate::api::models::DiscoveryItem;
use crate::player::StreamMetadata;

/// All events flowing through the app — user actions, async results, and
/// internal signals. The [`App`](crate::app::App) event loop dispatches
/// each variant to the appropriate handler.
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
    PlaybackStarted {
        title: String,
    },
    PlaybackLoading,
    PlaybackFinished,
    PlaybackPosition(f64),
    AudioLevels {
        rms: f64,
        peak: f64,
    },
    StreamMetadataChanged(StreamMetadata),

    AddToQueue(DiscoveryItem),
    AddToQueueNext(DiscoveryItem),
    RemoveFromQueue,
    ClearQueue,

    LoadNtsLive,
    NtsLiveLoaded(Vec<DiscoveryItem>),
    LoadNtsPicks,
    NtsPicksLoaded(Vec<DiscoveryItem>),

    LoadGenres,
    GenresLoaded(Vec<DiscoveryItem>),
    SearchByGenre {
        genre_id: String,
    },
    SearchByQuery {
        query: String,
    },
    SearchResultsPartial {
        search_id: u64,
        items: Vec<DiscoveryItem>,
        done: bool,
    },

    VolumeUp,
    VolumeDown,
    VolumeChanged(u8),

    OpenDirectPlay,
    CloseDirectPlay,

    PlaybackDuration(Option<f64>),
    SeekRelative(f64),
    OpenSeekModal,
    CloseSeekModal,

    CycleVisualizer,
    ToggleSkipIntro,
    OnboardingComplete {
        theme: String,
        completed_screens: Vec<String>,
    },
    ShowOnboarding,

    ShowError(String),
    ClearError,
    ShowHelp,
    HideHelp,
    Tick,
}
