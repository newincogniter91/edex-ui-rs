# edex-ui-rs

Port di [eDEX-UI](https://github.com/GitSquared/edex-ui) da Electron a Rust (egui con backend OpenGL), pensato per PC vecchi con GPU datate.

L'app parte a schermo intero. Layout e tema "tron" ricalcano l'originale: colonne laterali (orologio, sistema, CPU, RAM, processi / rete, globo, traffico), shell a 5 tab, filesystem a icone e tastiera virtuale.

## Installazione
Scarica `edex-ui-rs-setup.exe` (artifact `edex-ui-rs-installer` della build) ed eseguilo: installa in Programmi, crea i collegamenti nel menu Start (e sul desktop, se vuoi) e un disinstallatore.

### Linux (x86_64 e aarch64)
Scarica `edex-ui-rs-2.0.0-linux-x86_64.tar.gz` oppure `edex-ui-rs-2.0.0-linux-aarch64.tar.gz` (artifact `edex-ui-rs-linux-x86_64` / `edex-ui-rs-linux-aarch64` della build), estrai e lancia `./install.sh`: installa l'app per il tuo utente in `~/.local/bin` e aggiunge la voce nel menu applicazioni. In alternativa avvia direttamente `./edex-ui-rs`.

Requisiti: sessione X11 o Wayland, OpenGL 3.x (anche software/Mesa), la libreria `libasound2` per l'audio (l'app parte anche senza scheda audio) e una shell (usa `$SHELL`). Compilato su Ubuntu 22.04, quindi gira su distribuzioni con glibc 2.35 o più recente.

## Avvio e audio
All'avvio parte la sequenza originale: boot log, schermata del titolo con glitch, comparsa dei pannelli. Suoni e volumi sono quelli di eDEX-UI (tasti, output del terminale, cartelle, pannelli).

## Impostazioni e scorciatoie
Impostazioni (tema, tastiera, audio, orologio, ping, shell, ecc.) con **Ctrl+Shift+S**, salvate in `%APPDATA%\edex-ui-rs\settings.json`. I 21 temi e i 19 layout tastiera sono quelli di eDEX-UI.

| Scorciatoia | Azione |
|---|---|
| Ctrl+Shift+C / Ctrl+Shift+V / click destro | copia selezione / incolla / incolla |
| Ctrl+Tab, Ctrl+Shift+Tab, Ctrl+1…5 | cambio tab |
| Ctrl+Shift+S / K | impostazioni / elenco scorciatoie |
| Ctrl+Shift+F | fuzzy finder sulla cartella corrente |
| Ctrl+Shift+L / H | vista a lista / file nascosti |
| Ctrl+Shift+P | modalità password (niente suoni dei tasti) |
| Ctrl+Shift+F5 | rivede l'animazione iniziale |

Il clic sui processi apre la tabella completa. La posizione sul globo usa una richiesta HTTP a ip-api.com (disattivabile nelle impostazioni).

## Licenza
eDEX-UI è sotto GPL-3.0, quindi questo port è distribuito sotto GPL-3.0 (vedi `LICENSE`): chi riceve un binario ha diritto al sorgente.

## Font
- **Fira Mono** è incorporato (SIL OFL 1.1).
- **United Sans** (House Industries) ha una licenza che non ne consente la ridistribuzione, quindi non è inclusa.
  Per il look identico copia `united_sans_medium.ttf` e `united_sans_light.ttf` accanto all'exe (o in una cartella `fonts/`).
  Senza, l'app usa il font proporzionale di egui.

## Icone e dati
- Icone dei file: Font Awesome Free 5.15 (CC BY 4.0), le stesse dell'originale, pre-rasterizzate in `assets/icons.bin`.
- Suoni e log di avvio: dagli asset di eDEX-UI (GPL-3.0).
- Punti dei continenti del globo e regole di abbinamento icone/estensioni: derivati dai dati di eDEX-UI (GPL-3.0).

## Da fare
Altri temi.
