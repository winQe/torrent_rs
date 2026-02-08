# Torrent RS

A BitTorrent client written in Rust. Downloads torrents via the BitTorrent protocol with concurrent peer connections, piece verification, and a progress bar.

## Usage

```
cargo run -- <TORRENT_FILE> [OPTIONS]
```

**Options:**

| Flag | Description | Default |
|------|-------------|---------|
| `-o, --output <DIR>` | Download directory | `./downloads` |
| `-p, --peers <N>` | Max peer connections | `50` |
| `-v, --verbose` | Enable debug logging | off |

**Example:**

```
cargo run -- example/debian-12.7.0-amd64-netinst.iso.torrent -o ./downloads
```

## Features

- Bencode parsing and `.torrent` file reading
- HTTP tracker announce
- Peer wire protocol (handshake, bitfield, request/piece messages)
- Rarest-first piece selection
- Pipelined block requests
- SHA-1 piece verification
- Multi-file torrent support
- Async concurrency via Tokio (spawned peer workers, semaphore-bounded connections)
- Lock-free atomic stats, `RwLock`/`Mutex` shared state, `mpsc` channels for piece completion

## Sources

- https://wiki.theory.org/BitTorrentSpecification
- https://blog.jse.li/posts/torrent/
