<p align="center">
  <img src="static/icon.png" alt="UrDis Logo" width="128" height="128">
</p>

# UrDis

A modern, high-performance Redis Explorer built with **Tauri** and **SvelteKit**.

*Inspired by [Medis](https://github.com/luin/medis), reimagined with modern web technologies.*

## Features

- **Tree View Navigation** - Browse Redis keys organized in a hierarchical tree structure
- **All Data Types Supported** - View and edit String, Hash, List, Set, Sorted Set (ZSet), and Stream
- **Live TTL Countdown** - Real-time countdown display for keys with expiration
- **In-place Value Editing** - Edit key values directly with line numbers
- **Multi-Database Support** - Quickly switch between all 16 Redis databases with key counts
- **Server Management** - Save and manage multiple Redis server connections
- **Smart Search** - Find keys with pattern matching support
- **Incremental Scan** - Progressive key loading for large databases without blocking
- **Smart Caching** - Intelligent value caching for near-instant key switching
- **Resizable Sidebar** - Adjustable panel width for comfortable browsing
- **Memory & Encoding Info** - View memory usage and internal encoding for each key

## Tech Stack

- **Frontend**: [SvelteKit](https://kit.svelte.dev/) + [Svelte 5](https://svelte.dev/)
- **Backend**: [Tauri 2](https://tauri.app/) + [Rust](https://www.rust-lang.org/)
- **Redis Client**: [redis-rs](https://github.com/redis-rs/redis-rs) with Tokio async runtime
- **UI Components**: [SimpleBar](https://github.com/Grsmto/simplebar) for custom scrollbars

## Prerequisites

- [Node.js](https://nodejs.org/) (v18 or later)
- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Tauri Prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites)

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/your-username/urdis.git
   cd urdis
   ```

2. Install dependencies:
   ```bash
   yarn install
   # or
   npm install
   ```

3. Run in development mode:
   ```bash
   yarn tauri dev
   # or
   npm run tauri dev
   ```

4. Build for production:
   ```bash
   yarn tauri build
   # or
   npm run tauri build
   ```

## Usage

1. Launch the application
2. Enter your Redis connection details (host, port, password) or select a saved server
3. Click **Connect** to establish connection
4. Browse keys in the sidebar tree view
5. Use search box to filter keys
6. Click on any key to view/edit its value
7. Use the database selector at the bottom to switch databases

## Security

UrDis uses **Tauri Plugin Store** to securely store Redis connection information:

- **Backend Storage**: Server credentials are stored in the Rust backend, not accessible via browser DevTools
- **OS-Level Protection**: Connection data is saved in the application data directory with OS file permissions
- **No Plaintext in Browser**: Passwords and credentials are never exposed in browser localStorage

## Project Structure

```
urdis/
├── src/                     # SvelteKit frontend
│   ├── routes/
│   │   ├── login/           # Connection page
│   │   ├── explorer/        # Main explorer interface
│   │   └── +layout.svelte   # Root layout
│   ├── lib/                 # Shared utilities & stores
│   │   ├── stores.js        # Svelte stores
│   │   ├── secureStore.js   # Tauri Store wrapper for secure storage
│   │   ├── utils.js         # Helper functions
│   │   └── actions.js       # Svelte actions
│   └── styles/              # Centralized styles
│       ├── global.scss      # Global styles
│       ├── layout.scss      # Layout styles
│       ├── login.scss       # Login page styles
│       └── explorer.scss    # Explorer page styles
├── src-tauri/               # Tauri/Rust backend
│   └── src/
│       ├── lib.rs           # Main library & commands
│       └── main.rs          # Application entry
└── static/                  # Static assets
```

## License

This project is licensed under the [MIT License](LICENSE).

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
