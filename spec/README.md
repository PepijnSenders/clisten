# clisten — Implementation Specification

Split from `spec.md` into layered chunks, ordered bottom-up from foundation to integration.

| File | Layer | Contents |
|------|-------|----------|
| [00-overview.md](00-overview.md) | Foundation | Project overview, layout wireframe, project structure, Cargo.toml, module stubs |
| [01-architecture.md](01-architecture.md) | Architecture | Component pattern, action/message passing, async model, data flow |
| [02-data-models.md](02-data-models.md) | Data models | All serde types (NTS + SC), DiscoveryItem enum, DB record types |
| [03-api-nts.md](03-api-nts.md) | NTS API | NTS API reference (endpoints, JSON examples), NtsClient code |
| [04-api-soundcloud.md](04-api-soundcloud.md) | SoundCloud API | SC API reference, client_id extraction, auth flow, SoundCloudClient code |
| [05-player.md](05-player.md) | Player | MpvPlayer with IPC socket, Queue struct |
| [06-database.md](06-database.md) | Database | SQL migration, rusqlite Database wrapper |
| [07-config.md](07-config.md) | Config | TOML schema, Config struct, loading/saving code |
| [08-components.md](08-components.md) | Components | Component trait, all UI components (tabs, list, search, now playing, controls, NTS views, SC views) |
| [09-keybindings.md](09-keybindings.md) | Keybindings + Errors | Keybinding table, error handling strategy |
| [10-implementation.md](10-implementation.md) | Implementation | Phased build order (5 phases with testable milestones) |
