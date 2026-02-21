# Genre Search Handoff

## What was done

Simplified sub-tabs from 8 to 3 (Live, Picks, Search). Tab/BackTab cycles through them. Search tab shows a genre list fetched from `/api/v2/genres`, with "My Favorites" and "History" at the top. Selecting a genre triggers a client-side search of `recently-added` episodes, filtering by genre tag ID.

All 95 tests pass. Build is clean.

## The problem: genre search returns empty for most genres

### Root cause

The NTS `/api/v2/collections/recently-added` endpoint **caps offset at ~240**. Despite claiming 84,979 total episodes, you can only access the most recent ~252. The current code tries to sample 20 points across 85K episodes, but offsets >240 return `422 Unprocessable Entity`.

So we're only ever searching ~252 episodes. Common tags like `genres-ambientnewage-ambient` (28 hits) work, but rarer ones like `genres-ambientnewage-newage` (0 hits in 252) don't.

### What we tried
- Matching by genre display name → wrong, should match tag IDs
- Matching by tag ID on raw `NtsEpisodeDetail` → correct but limited by offset cap
- Sampling across catalog with large offsets → broken, API rejects offset >240

### What needs investigation

The NTS explore page at `nts.live/explore` filters by genre client-side using React state (`window._REACT_STATE_`). It likely uses either:

1. **Algolia or similar search service** — the page source contains `APP_ID` which hints at Algolia. Need to extract the Algolia app ID and API key from the page JS bundles, then query Algolia directly with genre facet filters.

2. **A different API endpoint** we haven't found — tried `/api/v2/explore`, `/api/v2/episodes`, various query params (`genre=`, `tag=`, `genres=`). All either 400 or ignore the param.

3. **The shows endpoint as a workaround** — `/api/v2/shows` returns 1,679 shows with no offset cap (needs verification). Could fetch shows, then fetch episodes per show. But the `genre=`/`tag=` params are ignored there too, so you'd need to filter client-side on the show's genre tags.

### Recommended next step

Open browser DevTools on `nts.live/explore`, select a genre, and watch the Network tab. This will reveal exactly which endpoint/service the frontend queries. Most likely it's Algolia — if so, the app ID and search-only API key are public (embedded in the JS bundle) and can be used directly.

## Files changed

### Modified
- `src/action.rs` — removed 10 old actions (LoadNtsRecent, NtsShowsLoaded, etc.), added 4 new (LoadGenres, GenresLoaded, SearchByGenre, SearchResultsPartial)
- `src/api/models.rs` — added NtsGenresResponse, NtsGenreCategory, NtsSubgenre, DiscoveryItem::NtsGenre variant
- `src/api/nts.rs` — added fetch_genres(), fetch_recent_raw()
- `src/components/nts/mod.rs` — NtsSubTab reduced to Live/Picks/Search, added active_index()
- `src/components/nts/collections.rs` — removed recent_items_from_action
- `src/components/discovery_list.rs` — added append_items(), NtsGenre Enter handler
- `src/app.rs` — Tab/BackTab cycling, genre/search handlers, search_id + viewing_genre_results state, removed old tab handlers, updated help overlay
- `src/db.rs` — NtsGenre match arms (early return, not storable)
- `tests/phase2_nts_core.rs` — added Search tab + active_index tests
- `tests/phase3_nts_discovery.rs` — rewrote: removed shows/mixtapes/schedule tests, added NtsGenre + append_items + Tab cycling tests
- `tests/phase6_refinements.rs` — updated sub-tab tests for 3 tabs

### Deleted
- `src/components/nts/shows.rs`
- `src/components/nts/mixtapes.rs`
- `src/components/nts/schedule.rs`

## Architecture notes

- Genre tag IDs: API returns `ambientnewage-newage`, episodes tag as `genres-ambientnewage-newage` (prefix `genres-`)
- `search_id: u64` in App — incremented on each new search, used to discard stale SearchResultsPartial messages
- `viewing_genre_results: bool` — controls whether Escape goes back to genre list or does normal back behavior
- SwitchSubTab always clears discovery list and re-fetches (no stale data between tabs)
- `append_items()` on DiscoveryList preserves scroll position and re-applies active filter
