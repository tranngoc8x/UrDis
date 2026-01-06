# UrDis

A modern, high-performance Redis Explorer built with **Tauri** and **SvelteKit**.

## Features

- **Tree View Navigation** - Browse Redis keys organized in a hierarchical tree structure with folder-like navigation
- **All Data Types Supported** - View and edit String, Hash, List, Set, Sorted Set (ZSet), and Stream
- **Live TTL Countdown** - Real-time countdown display for keys with expiration
- **In-place Value Editing** - Edit key values directly with syntax highlighting and save changes
- **Multi-Database Support** - Quickly switch between all 16 Redis databases with key counts
- **Smart Caching** - Intelligent value caching for near-instant key switching
- **Incremental Scan** - Progressive key loading with SCAN for large databases without blocking
- **Resizable Sidebar** - Adjustable panel width for comfortable browsing
- **Memory & Encoding Info** - View memory usage and internal encoding for each key

## Tech Stack

- **Frontend**: [SvelteKit](https://kit.svelte.dev/) + [Svelte 5](https://svelte.dev/)
- **Backend**: [Tauri 2](https://tauri.app/) + [Rust](https://www.rust-lang.org/)
- **Redis Client**: [redis-rs](https://github.com/redis-rs/redis-rs) with Tokio async runtime

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
2. Enter your Redis connection details (host, port, password)
3. Click **Connect** to establish connection
4. Browse keys in the sidebar tree view
5. Click on any key to view/edit its value
6. Use the database selector at the bottom to switch databases

## Project Structure

```
urdis/
├── src/                    # SvelteKit frontend
│   ├── routes/
│   │   ├── login/          # Connection page
│   │   └── explorer/       # Main explorer interface
│   └── lib/                # Shared utilities & stores
├── src-tauri/              # Tauri/Rust backend
│   └── src/
│       ├── lib.rs          # Main library & commands
│       └── main.rs         # Application entry
└── static/                 # Static assets
```

## License

This project is licensed under the [MIT License](LICENSE).

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
