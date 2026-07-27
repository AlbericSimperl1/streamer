# hyprStream

Stream je Hyprland-desktop (of een virtuele monitor) draadloos naar een iPad — of een ander apparaat — met lage latency, in full H.264.

## Overzicht

```
┌─────────────┐         H.264 video          ┌────────────┐
│  PC (Linux) │  ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ▶  │   iPad     │
│  Hyprland   │        UDP (wireless)        │ hyprClient │
│  hyprStream │                              │            │
└─────────────┘                              └────────────┘
```

- **PC-kant** (`PC/`): een Rust-applicatie met `egui` GUI die een headless virtual monitor aanmaakt via `hyprctl`, en deze captured via PipeWire & XDG Desktop Portals. Streamt de output als H.264-encoded video over TCP naar een client-apparaat.
- **iPad-kant** (`IPAD/`): een Swift app die de H.264-stream ontvangt, decodeert met VideoToolbox en weergeeft in real-time.

## Structuur

```
streamer/
├── PC/                  # Rust applicatie (hyprStream)
│   ├── src/
│   │   ├── main.rs      # Entrypoint — egui/eframe window
│   │   ├── app.rs       # App-logica, state, capture lifecycle
│   │   ├── gui.rs       # egui custom GUI (donker thema)
│   │   ├── hypr.rs      # hyprctl wrapper: monitors aanmaken/verwijderen
│   │   ├── types.rs     # MonitorConfig, MonitorJson, LogEntry
│   │   └── capture/     # PipeWire + Portal capture pipeline
│   │       ├── mod.rs       # CaptureSession orchestrator
│   │       ├── pipewire.rs  # PipeWire connectie & buffer afhandeling
│   │       ├── portal.rs    # XDG Desktop Portal screencast
│   │       ├── encoder.rs   # H.264 encoding
│   │       └── packetizer.rs# NAL unit packetizing voor netwerk
│   └── Cargo.toml
├── IPAD/                # Swift applicatie — hyprClient
│   ├── Sources/hyprClient/
│   │   ├── hyprPadClient.swift   # App entrypoint
│   │   ├── AppState.swift         # Observable app state
│   │   ├── ContentView.swift      # Hoofd UI
│   │   ├── StreamEngine.swift     # TCP ontvangst + H.264 feed
│   │   ├── H264Decoder.swift      # VideoToolbox H.264 → frames
│   │   ├── VideoView.swift        # Metal-accelerated rendering
│   │   └── GlobalBridge.swift     # Interop met Rust core
│   ├── Sources/CRustCore/  # C bridge + voorgecompileerde .a library
│   ├── rust_core/          # Rust library die naar iOS compileert
│   ├── Package.swift
│   └── xtool.yml
└── README.md
```

## Hoe het werkt

1. De PC app maakt een **headless virtual monitor** aan via `hyprctl output create headless <name>`.  
   Alle gewenste resolutie/refresh rate/position worden als hyprctl keywords toegepast.

2. De capture pipeline start via een **XDG Desktop Portal** (PipeWire) screen cast. De user kiest de virtuele monitor in de portal popup.

3. Raw video frames worden **H.264 encoded** en over TCP naar het ingestelde IP-adres gestuurd (pakketjes in de eigen stream code.

4. De **iPad app** (of andere client) maakt TCP-verbinding, leest H.264 NAL units en voert deze via **VideoToolbox hardware decoding** naar het scherm.

## Bouwen

### PC (hyprStream)

```bash
cd PC
cargo build --release
```

Dependencies:

- Hyprland (natuurlijk)
- PipeWire, `libpipewire`
- XDG Desktop Portal (`org. HDR.portal.Desktop`)
- ffmpeg / een H.264-hardware encoder beschikbaar

### iPad (hyprClient)

```bash
cd IPAD/rust_core
cargo build --target aarch64-apple-ios --release
cbindgen --config cbindgen.toml --output ../Sources/CRustCore/ind/rust_core.h
cp target/aarch64-apple-ios/release/librust_core.a ../Sources/CRustCore/lib/librust_core.a
```

Daarna open je `IPAD/Package.swift` in Xcode en build voor device.

## Configuratie

In de GUI:

- **Naam**: virtual monitor name (standaard `virtual`)
- **Resolutie**: breedte × hoogte
- **FPS**: refreshrate voor de virtual monitor
- **Positie**: X,Y offset vanaf primary scherm
- **Scale**: de HDR scaling factor
- **IP**: het IP-adres van de stream client (iPad)

Hit "Configureren", klik "Start Capture", en je iPad kan verbinden.

## Features

- Virtual monitor aansturing via `hyprctl`
- PipeWire + XDG Desktop Portal screencast pipeline
- H.264 hardware-encoding
- TCP-stream met NAL packetizing
- Realtime video stream naar mobiele client
- iPad app met H.264 VideoToolbox decoding (Metal-backed SwiftUI)
- Dark theme UI met egui custom styling
- FPS-counter en logging via built-in console
- Virtual monitor autom. opruimen bij afsluiten
