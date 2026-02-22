// Data fetching: spawns async tasks that load NTS live/picks/genre data.

use std::future::Future;

use crate::action::Action;
use crate::api::genres::TOP_GENRES;
use crate::api::models::DiscoveryItem;
use crate::app::App;

// NTS search API caps results at 12 per page (server limit).
const SEARCH_PAGE_SIZE: u64 = 12;
// Maximum offset the NTS API will return results for.
const SEARCH_MAX_OFFSET: u64 = 240;
// Send partial results to the UI after accumulating this many items.
const SEARCH_BATCH_SIZE: usize = 48;
impl App {
    /// Spawn a background fetch task that sends the result (or an error) back as an action.
    fn spawn_fetch<Fut>(&self, fut: Fut, on_ok: fn(Vec<DiscoveryItem>) -> Action)
    where
        Fut: Future<Output = anyhow::Result<Vec<DiscoveryItem>>> + Send + 'static,
    {
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match fut.await {
                Ok(items) => tx.send(on_ok(items)).ok(),
                Err(e) => tx.send(Action::ShowError(e.to_string())).ok(),
            };
        });
    }

    pub(super) fn spawn_fetch_live(&self) {
        let client = self.nts_client.clone();
        self.spawn_fetch(
            async move { client.fetch_live().await },
            Action::NtsLiveLoaded,
        );
    }

    pub(super) fn spawn_fetch_picks(&self) {
        let client = self.nts_client.clone();
        self.spawn_fetch(
            async move { client.fetch_picks().await },
            Action::NtsPicksLoaded,
        );
    }

    pub(super) fn load_genres(&mut self) -> anyhow::Result<()> {
        let mut items: Vec<DiscoveryItem> = Vec::with_capacity(TOP_GENRES.len());
        for &(id, name) in TOP_GENRES {
            items.push(DiscoveryItem::NtsGenre {
                name: name.to_string(),
                genre_id: id.to_string(),
            });
        }

        self.action_tx.send(Action::GenresLoaded(items))?;
        self.viewing_genre_results = false;
        Ok(())
    }

    pub(super) fn search_by_genre(&mut self, genre_id: String) -> anyhow::Result<()> {
        self.search_id += 1;
        let sid = self.search_id;
        self.discovery_list.set_items(vec![]);
        self.discovery_list.set_loading(true);
        self.viewing_genre_results = true;

        // Remote paginated search
        let tx = self.action_tx.clone();
        let client = self.nts_client.clone();
        tokio::spawn(async move {
            let mut buf = Vec::new();
            let mut offset = 0u64;

            while offset <= SEARCH_MAX_OFFSET {
                match client
                    .search_episodes(&genre_id, offset, SEARCH_PAGE_SIZE)
                    .await
                {
                    Ok(items) => {
                        let got = items.len();
                        buf.extend(items);
                        if (got as u64) < SEARCH_PAGE_SIZE {
                            break;
                        }
                    }
                    Err(_) => break,
                }
                offset += SEARCH_PAGE_SIZE;

                if buf.len() >= SEARCH_BATCH_SIZE || offset > SEARCH_MAX_OFFSET {
                    let batch = std::mem::take(&mut buf);
                    let done = offset > SEARCH_MAX_OFFSET;
                    tx.send(Action::SearchResultsPartial {
                        search_id: sid,
                        items: batch,
                        done,
                    })
                    .ok();
                }
            }

            // Flush remaining
            tx.send(Action::SearchResultsPartial {
                search_id: sid,
                items: buf,
                done: true,
            })
            .ok();
        });

        Ok(())
    }

    pub(super) fn search_by_query(&mut self, query: String) -> anyhow::Result<()> {
        self.search_id += 1;
        let sid = self.search_id;
        self.discovery_list.set_items(vec![]);
        self.discovery_list.set_loading(true);
        self.viewing_query_results = true;

        let tx = self.action_tx.clone();
        let client = self.nts_client.clone();
        tokio::spawn(async move {
            let mut buf = Vec::new();
            let mut offset = 0u64;

            while offset <= SEARCH_MAX_OFFSET {
                match client
                    .search_episodes_by_query(&query, offset, SEARCH_PAGE_SIZE)
                    .await
                {
                    Ok(items) => {
                        let got = items.len();
                        buf.extend(items);
                        if (got as u64) < SEARCH_PAGE_SIZE {
                            break;
                        }
                    }
                    Err(_) => break,
                }
                offset += SEARCH_PAGE_SIZE;

                if buf.len() >= SEARCH_BATCH_SIZE || offset > SEARCH_MAX_OFFSET {
                    let batch = std::mem::take(&mut buf);
                    let done = offset > SEARCH_MAX_OFFSET;
                    tx.send(Action::SearchResultsPartial {
                        search_id: sid,
                        items: batch,
                        done,
                    })
                    .ok();
                }
            }

            // Flush remaining
            tx.send(Action::SearchResultsPartial {
                search_id: sid,
                items: buf,
                done: true,
            })
            .ok();
        });

        Ok(())
    }
}
