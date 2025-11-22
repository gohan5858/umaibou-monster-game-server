# Project Overview

## Purpose

This project is a server for a 3D real-time battle game. It provides player matching via a REST API and real-time game state updates (at 60Hz) via WebSockets.

## Tech Stack

- **Framework:** Actix Web
- **Actors:** Actix
- **Async Runtime:** Tokio
- **WebSocket:** Actix Web Actors
- **Serialization:** Serde, Serde JSON
- **Database:** SQLx (with SQLite)
- **UUID:** uuid
- **Time:** chrono
- **Testing:** `actix-test`, `awc`, `tokio-tungstenite`

## Codebase Structure

```
src/
├── lib.rs                  # Library entry point
├── main.rs                 # Server startup
├── models.rs               # Data models
├── utils.rs                # Utility functions
├── db/
│   ├── mod.rs
│   └── models.rs           # Database models
├── game/
│   ├── mod.rs
│   ├── state.rs            # Game state management
│   └── manager.rs          # 60Hz game loop
└── handlers/
    ├── mod.rs
    ├── matching.rs         # Matching API
    ├── model_upload.rs     # 3D model upload handler
    └── websocket.rs        # WebSocket handler
```
