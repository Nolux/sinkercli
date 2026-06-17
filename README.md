# sinkercli

A terminal UI for managing Linux audio routing — create loopbacks, virtual sinks, and virtual microphone inputs without touching config files.

Works on both **PipeWire** and **PulseAudio** (auto-detected).

```
╭─ sinkercli ──────────────────────────────────────── [PipeWire] ─╮
│ Sources                    │ Sinks                               │
│────────────────────────────│──────────────────────────────────── │
│ ▶ Arctis Nova 7P Mic       │   Built-in Speakers                 │
│   [mon] Monitor of Speakers│ ▶ Headphones                        │
│   [mon] Monitor of game-out│   [null] game-out                   │
├────────────────────────────┴──────────────────────────────────── ┤
│ Applications               │ Active Loopbacks                    │
│────────────────────────────│──────────────────────────────────── │
│ ▶ Firefox  → Headphones    │   1  game-out.monitor → Headphones  │
│   Spotify  → Speakers      │                                     │
├─────────────────────────────────────────────────────────────────┤
│ [u] virtual input  [l] listen  [m] move app  [n] loopback  ...  │
╰─────────────────────────────────────────────────────────────────╯
```

## Features

- **Sources & sinks** — list all audio inputs and outputs, including monitors
- **Loopbacks** — route any source to any sink (`n`)
- **Listen to a sink** — tap a sink's output to another sink, e.g. hear a virtual sink through headphones (`l`)
- **Virtual sinks** — create null/virtual sinks to route apps into (`v`)
- **Virtual mic inputs** — wrap a sink monitor as a proper microphone so recording software sees it (`u`)
- **App routing** — move a running application's audio stream to any sink (`m`)
- **Presets** — save and restore named routing configurations (`p`)

## Installation

### AUR (Arch Linux)

Pre-built binary:
```bash
yay -S sinkercli-bin
```

Build from source:
```bash
yay -S sinkercli
```

### From a GitHub release

Download the binary from the [releases page](https://github.com/Nolux/sinkercli/releases), extract, and place it on your `PATH`:

```bash
tar xzf sinkercli-x86_64-unknown-linux-gnu.tar.gz
sudo install -Dm755 sinkercli /usr/local/bin/sinkercli
```

### Build from source

Requires Rust and `libpulse` headers:

```bash
# Arch
sudo pacman -S rust libpulse

# Ubuntu/Debian
sudo apt install cargo libpulse-dev

git clone https://github.com/Nolux/sinkercli
cd sinkercli
cargo build --release
./target/release/sinkercli
```

## Usage

```bash
sinkercli
```

### Keybindings

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Cycle panel focus |
| `↑` / `↓` or `k` / `j` | Navigate |
| `n` | New loopback (pick source → pick sink) |
| `v` | New virtual sink |
| `l` | Listen to selected sink (tap it to an output) |
| `u` | Create virtual mic input from selected sink's monitor |
| `m` | Move selected application to a different sink |
| `d` | Delete selected loopback or virtual sink |
| `p` | Presets — load or save current routing |
| `r` | Refresh |
| `q` / `Ctrl+C` | Quit |

### Common workflows

**Route a game's audio separately from other apps**
1. `v` → create a virtual sink called `game-out`
2. `Tab` to Applications, select the game, `m` → move it to `game-out`
3. `Tab` to Sinks, select `game-out`, `l` → listen via headphones

**Capture audio in OBS or recording software**
1. Create a virtual sink for the audio you want to capture
2. Route apps into it with `m`
3. In OBS, select `game-out.monitor` as an audio capture source
   — or use `u` to wrap it as a named virtual microphone if OBS doesn't show monitors

**Save a streaming preset**
1. Set up your routing
2. `p` → `s` → name it `streaming`
3. Next session: `p` → select `streaming` → `Enter` to restore everything

## Requirements

- PipeWire (recommended) or PulseAudio
- `pactl` on `PATH` (provided by `pipewire-pulse` or `pulseaudio`)

## License

MIT
