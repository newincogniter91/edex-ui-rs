# AGENTS.md

Guida per un'altra IA (o una persona) che prende in mano questo progetto. Leggila tutta prima di toccare il codice: contiene le regole del proprietario, come compilare e provare l'app, com'è fatto il codice e cosa è già stato provato o scartato.

## Il progetto in breve
- **Cosa è:** port di [eDEX-UI](https://github.com/GitSquared/edex-ui) da Electron a Rust (egui/eframe 0.27 con backend OpenGL/glow), pensato per PC vecchi con GPU datate.
- **Dove:** repo GitHub `newincogniter91/edex-ui-rs` (proprietario: Lorenzo, `newincogniter91`). Il codice e i commenti sono in italiano, le etichette dell'interfaccia in inglese come nell'originale.
- **Versione:** 2.0.0 (in `Cargo.toml`). Piattaforme: Windows, Linux x86_64, Linux aarch64 (64 bit). Niente 32 bit, niente macOS (c'è solo un ramo `open` per macOS in `fsdisp.rs`, mai provato).
- **Licenza:** GPL-3.0 (derivato dagli asset di eDEX-UI). Font United Sans NON incluso (licenza non ridistribuibile); Fira Mono incluso.

## Regole del proprietario (vanno rispettate)
1. **Fai solo le modifiche richieste.** Niente refactoring, sostituzioni o "miglioramenti" non chiesti.
2. **Modifiche mirate.** Per bug piccoli usa sostituzioni puntuali (`str_replace`), non riscrivere interi file: risparmia token.
3. **Rispondi in modo diretto e fattuale** alle domande specifiche, senza dirottare su strumenti o approcci alternativi non richiesti. Parla in italiano.
4. **Push:** non fare push finché il proprietario non lo chiede o non dà l'ok. Ogni push su `main` fa partire la build di tutti e tre i lavori. Per un push di sola documentazione usa `[skip ci]` nel messaggio del commit. Dopo un push che avvia la build **non aspettarne il completamento**: dai il link del run e basta.
5. **Release: solo lettura.** Puoi leggere le release e i loro file, mai creare, modificare, cancellare né fare richieste di scrittura "di prova" sull'endpoint delle release. Le release le fa il proprietario a mano. Il workflow non deve creare release né partire sui tag.
6. **Credenziali:** se ti viene dato un token GitHub, usalo solo per quello che serve, non scriverlo in file, commit, log o memoria. Ricorda al proprietario di revocarlo a fine lavoro.
7. **Non dichiarare come verificato ciò che non hai provato.** Dì sempre cosa hai compilato/avviato davvero e cosa no.
8. **Repo privata:** il proprietario vuole la repo privata, ma al momento è pubblica. Se la rende privata, i runner ARM gratuiti (`ubuntu-22.04-arm`) non sono più disponibili: il lavoro `linux` per aarch64 va spostato su una compilazione incrociata da runner x86.

## Come compilare e provare
Su Windows e su Linux: `cargo build --release` (serve Rust >= 1.88: alcune dipendenze lo richiedono). Su Linux servono i pacchetti di sviluppo `pkg-config libasound2-dev libxkbcommon-dev libwayland-dev libx11-dev libxcb1-dev libgl1-mesa-dev`.

**Ambiente di sviluppo senza internet completo (sandbox Ubuntu 24.04):** `rustup` non è raggiungibile, ma `apt` sì. Funziona così:

```sh
apt-get update
apt-get install -y rustc-1.91 cargo-1.91 pkg-config libasound2-dev libxkbcommon-dev \
    libwayland-dev libx11-dev libxcb1-dev libgl1-mesa-dev
export PATH=/usr/lib/rust-1.91/bin:$PATH
cargo check          # la prima volta scarica le crate (~2-3 minuti)
cargo build          # build di debug; i comandi lunghi vanno tenuti in primo piano con timeout:
                     # un processo lasciato in background viene ucciso quando finisce la chiamata
cargo test           # 4 test: audio, icone, impostazioni (2)
```
Con `rustc-1.85` (la `cargo` predefinita di apt è la 1.75) la compilazione fallisce: servono almeno 1.88.

**Provare l'app a video, anche senza schermo** (è stato fatto e funziona):

```sh
apt-get install -y xvfb libgl1-mesa-dri libglx-mesa0 x11-utils imagemagick xdotool
export XDG_CONFIG_HOME=/tmp/cfg LIBGL_ALWAYS_SOFTWARE=1
mkdir -p /tmp/cfg/edex-ui-rs
echo '{"nointro":true,"allow_windowed":true,"audio":false}' > /tmp/cfg/edex-ui-rs/settings.json
Xvfb :99 -screen 0 1600x900x24 & sleep 2
DISPLAY=:99 ./target/debug/edex-ui-rs &      # gli errori ALSA "cannot find card" sono attesi senza scheda audio
sleep 15
DISPLAY=:99 xdotool search --name eDEX       # per dare focus: xdotool windowfocus <id>, poi xdotool type/key
DISPLAY=:99 import -window root /tmp/shot.png   # poi guarda l'immagine
```
`nointro` salta l'animazione iniziale, `allow_windowed` evita lo schermo intero. Questo prova solo Linux x86_64 con X11 e OpenGL software.

**Non verificabile in sandbox:** build Windows, build aarch64, Wayland, il comportamento su hardware vero. Il primo controllo reale per questi casi è la build su GitHub Actions.

## CI e rilascio (`.github/workflows/build.yml`)
- Parte a ogni push su `main` e con `workflow_dispatch`. Tre lavori indipendenti, quindi in parallelo: `windows`, `linux` (matrice `x86_64` su `ubuntu-22.04`, `aarch64` su `ubuntu-22.04-arm`).
- **Windows:** `cargo build --release`, poi Inno Setup (`installer/edex-ui-rs.iss`, versione presa da `Cargo.toml`) → artifact `edex-ui-rs-installer` (`dist/edex-ui-rs-setup.exe`).
- **Linux:** `cargo build --release`, poi `edex-ui-rs-<versione>-linux-<arch>.tar.gz` con eseguibile, `install.sh`, `.desktop`, icona, `LICENSE`, `README.md` (file in `linux/`) → artifact `edex-ui-rs-linux-<arch>`. Compilato su Ubuntu 22.04 per avere glibc 2.35 come minimo. Non è né un installer vero né un AppImage; `install.sh` installa nella home dell'utente senza root.
- Nessun lavoro crea release. Per cambiare versione modifica `version` in `Cargo.toml` (e di conseguenza `Cargo.lock`).

## Architettura (`src/`)
L'interfaccia è disegnata a mano con il `Painter` di egui; tutte le misure sono multipli di `vh` (un centesimo dell'altezza della finestra), così il layout scala come nell'originale.

| File | Ruolo |
|---|---|
| `main.rs` | Entry point e `App`: loop `update`, layout a colonne, scorciatoie, finestre, applicazione a caldo delle impostazioni, `draw_pointer` (cursore dell'app), `keep_awake` (solo Windows) |
| `settings.rs` | Struct `Settings` (serde, `#[serde(default)]`), `settings.json`, `config_dir()`, `apply_theme`, test |
| `theme.rs` | Palette corrente, helper colori (`fg`, `bg`, `fg_a`…), `tint` (riduce un colore del terminale alle tinte del tema), font, cornici dei moduli |
| `terminal.rs` | Terminale: PTY (`portable-pty`), parser `vt100`, disegno celle, selezione, copia/incolla, tab, `color_of` |
| `modules.rs` | Pannelli laterali: orologio, sistema, CPU (uso e frequenza), RAM, processi, rete, globo, traffico; `SysPanels::refresh` |
| `fsdisp.rs` | File manager (icone o lista), fuzzy finder, apertura file (`open_path`), dischi |
| `keyboard.rs` | Tastiera virtuale, layout da `assets/kb/*.json` |
| `dialogs.rs` | Finestre: impostazioni, scorciatoie, fuzzy finder, tabella processi |
| `shell.rs` | Cornice e tab del terminale |
| `hw.rs` | Produttore/modello/telaio (PowerShell su Windows, DMI su Linux) e batteria |
| `net.rs` | Thread di rete: ping, IP locale, geolocalizzazione con ip-api.com |
| `audio.rs` | Suoni (`rodio`, WAV incorporati); parte anche senza scheda audio |
| `boot.rs` | Sequenza iniziale: boot log, titolo con glitch, comparsa dei pannelli |
| `icons.rs` / `data.rs` | Icone dei file (`assets/icons.bin`, regole in JSON) e dati incorporati (temi, layout, ecc.) |

Gli asset (`assets/`) sono incorporati nell'eseguibile con `include_bytes!`: temi (21 JSON), layout tastiera (19 JSON), suoni, icone, punti dei continenti, log di avvio, icona dell'app.

## Convenzioni da seguire
- **Colori:** mai colori fissi nell'interfaccia o nel terminale. Usa le funzioni di `theme.rs`. Il terminale passa da `theme::tint` (solo luminosità, tinta del tema).
- **Nuova impostazione:** aggiungi il campo in `Settings`, in `Default`, la riga in `dialogs::settings_window`, l'uso in `main.rs`, e aggiorna il README. I campi nuovi devono avere un default sensato (il file esistente dell'utente non li contiene).
- **Scorciatoie:** tre posti da tenere allineati: la gestione degli eventi in `main.rs`, la lista `LIST` in `dialogs::shortcuts_window` e la tabella del README.
- **Codice per piattaforma:** `#[cfg(windows)]` deve avere sempre il ramo `#[cfg(not(windows))]` corrispondente (come in `hw.rs`, `fsdisp.rs`, `settings.rs`). Per le API di Windows non c'è una dipendenza dedicata: `keep_awake` dichiara a mano la funzione di `kernel32`.
- **Dipendenze:** aggiungile solo se servono davvero. Il programma deve restare leggero e girare su PC vecchi; le versioni sono bloccate in `Cargo.lock`.
- **Thread e blocchi lunghi:** rete e hardware sono letti in thread separati (`net.rs`, `hw.rs`); il thread della UI non deve bloccarsi.
- **Messaggi di commit:** in italiano, brevi, che dicano cosa cambia.

## Stato, limiti e decisioni già prese
- **Colori del terminale:** sono ridotti alla sola luminosità, quindi errori e comandi si distinguono per chiarezza e non per tinta. È voluto (richiesta del proprietario: tutto nei colori del tema, PowerShell/PSReadLine incluso).
- **Modalità password:** Ctrl+Shift+P la attiva a mano e silenzia solo suoni e evidenziazione dei tasti. Un rilevamento automatico dei prompt di password è stato scritto e poi **tolto** su richiesta del proprietario, che dopo quel commit riferì che non si vedeva nulla. La causa non è mai stata confermata (il rilevamento non toccava il disegno). Non reintrodurlo senza provarlo a video.
- **Cursore dell'app:** il puntatore di sistema viene nascosto e ne viene disegnato uno nel tema (freccia, cerchio, barra). Con il puntatore di sistema nascosto non si vedono i cursori di ridimensionamento sui bordi. Si può disattivare dalle impostazioni; la dimensione è regolabile (0.5–3.0).
- **Globo:** mostra solo la posizione dell'utente, senza i punti delle connessioni (servirebbe un database di geolocalizzazione). La geolocalizzazione è una richiesta HTTP a ip-api.com, disattivabile.
- **Scorciatoie:** non sono configurabili da file.
- **Windows:** il blocco di sospensione e screensaver (`SetThreadExecutionState`), la batteria e il produttore/modello via PowerShell sono codice solo Windows. La build su Actions compila, ma chi ha scritto questo file non l'ha mai avviato su Windows: i test a video sono stati fatti solo su Linux.
- **Etichette tagliate:** nella colonna di sinistra "MANUFACTURER / MODEL / CHASSIS" escono dal bordo in finestre piccole (visto su Linux). Non corretto.
- **Linux:** richiede glibc >= 2.35, X11 o Wayland, OpenGL 3.x (anche Mesa software) e `libasound2`. Provato solo su x86_64/X11.
- **Release esistenti:** v1.0, V1.1, V1.2, v1.3, v1.4 (tutte pre-release), gestite a mano dal proprietario. Titoli e note hanno qualche incoerenza di numerazione (es. V1.3 con note 1.3.1): segnalala se serve, non correggerla tu.

## Prima di dichiarare un lavoro finito
1. `cargo check` (e `cargo test` se tocchi impostazioni, icone o audio) senza errori.
2. Se tocchi l'interfaccia, avvia l'app con la procedura Xvfb sopra e guarda lo screenshot.
3. Aggiorna `README.md` se cambia qualcosa di visibile all'utente.
4. Di' chiaramente cosa hai verificato e cosa no (Windows, ARM, Wayland sono sempre "non verificati" da una sandbox Linux x86).
5. Non fare push senza ok; se lo fai, non aspettare la build.
