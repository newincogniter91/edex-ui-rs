# edex-ui-rs

Port di [eDEX-UI](https://github.com/GitSquared/edex-ui) da Electron a Rust (egui con backend OpenGL), pensato per PC vecchi con GPU datate.

L'app gira su **Windows** e su **Linux** (x86_64 e aarch64) e parte a schermo intero. Layout e tema "tron" ricalcano l'originale: colonne laterali (orologio, sistema, CPU, RAM, processi / rete, globo, traffico), shell a 5 tab, filesystem a icone e tastiera virtuale.

## Funzioni
- **Pannelli di sistema:** orologio (12/24 ore), uptime, tipo di sistema, alimentazione (ON, CHARGE, WIRED o percentuale di batteria), produttore/modello/telaio, uso e **frequenza della CPU**, RAM e swap.
- **Processi:** i primi cinque nel pannello; un clic apre la tabella completa, ordinabile.
- **Rete:** stato, IP, ping (indirizzo configurabile) e grafico del traffico.
- **Globo:** mostra la tua posizione con le coordinate reali (richiesta a ip-api.com, disattivabile nelle impostazioni). Non ci sono i punti delle singole connessioni.
- **Terminale:** 5 tab, selezione col mouse e copia/incolla. I colori ANSI, a 256 colori e RGB (per esempio quelli di PowerShell/PSReadLine) vengono ridotti alle sole tinte del tema scelto: restano le differenze di luminosità, non quelle di tinta.
- **File manager:** vista a icone o a lista (dimensione e data, ordinabile cliccando le colonne), file nascosti attivabili, fuzzy finder. Un clic su un file lo apre col programma predefinito; con Ctrl attivo sulla tastiera virtuale scrive invece il percorso nel terminale.
- **Tastiera virtuale:** 19 layout, gli stessi di eDEX-UI.
- **Cursore dell'app:** un puntatore disegnato nei colori del tema (freccia, cerchio sui pulsanti, barra sul testo). Si può tornare al puntatore di sistema e cambiarne la dimensione dalle impostazioni.
- **Windows sempre sveglio:** finché l'app è aperta, Windows non va in sospensione, non avvia lo screensaver e non spegne lo schermo. Su Linux non fa nulla.

## Installazione
Scarica `edex-ui-rs-setup.exe` (artifact `edex-ui-rs-installer` della build) ed eseguilo: installa in Programmi, crea i collegamenti nel menu Start (e sul desktop, se vuoi) e un disinstallatore.

### Linux (x86_64 e aarch64)
Scarica `edex-ui-rs-2.0.0-linux-x86_64.tar.gz` oppure `edex-ui-rs-2.0.0-linux-aarch64.tar.gz` (artifact `edex-ui-rs-linux-x86_64` / `edex-ui-rs-linux-aarch64` della build), estrai e lancia `./install.sh`: installa l'app per il tuo utente in `~/.local/bin` e aggiunge la voce nel menu applicazioni. In alternativa avvia direttamente `./edex-ui-rs`.

Requisiti: sessione X11 o Wayland, OpenGL 3.x (anche software/Mesa), la libreria `libasound2` per l'audio (l'app parte anche senza scheda audio) e una shell (usa `$SHELL`). Compilato su Ubuntu 22.04, quindi gira su distribuzioni con glibc 2.35 o più recente.

## Avvio e audio
All'avvio parte la sequenza originale: boot log, schermata del titolo con glitch, comparsa dei pannelli. Suoni e volumi sono quelli di eDEX-UI (tasti, output del terminale, cartelle, pannelli).

## Impostazioni e scorciatoie
Le impostazioni si aprono con **Ctrl+Shift+S** e si salvano in `%APPDATA%\edex-ui-rs\settings.json` su Windows, in `~/.config/edex-ui-rs/settings.json` su Linux. I 21 temi e i 19 layout tastiera sono quelli di eDEX-UI.

Opzioni disponibili: tema, layout tastiera, dimensione del font del terminale, audio e volume, suoni dei tasti, orologio 12/24 ore, indirizzo del ping, geolocalizzazione IP, file nascosti, vista a lista, cursore del terminale nascosto ("No cursor"), **nascondere il cursore di sistema** (usa quello dell'app), **dimensione del cursore dell'app**, shell, argomenti della shell, cartella iniziale, salto dell'intro e avvio in finestra. Molte si applicano subito; intro e finestra solo al prossimo avvio.

| Scorciatoia | Azione |
|---|---|
| Ctrl+Shift+C / Ctrl+Shift+V / click destro | copia selezione / incolla / incolla |
| Ctrl+Tab, Ctrl+Shift+Tab, Ctrl+1…5 | cambio tab |
| Ctrl+Shift+S / K | impostazioni / elenco scorciatoie |
| Ctrl+Shift+F | fuzzy finder sulla cartella corrente |
| Ctrl+Shift+L / H | vista a lista / file nascosti |
| Ctrl+Shift+P | modalità password manuale (niente suoni dei tasti) |
| Ctrl+Shift+F5 | rivede l'animazione iniziale |

Il clic sui processi apre la tabella completa. La posizione sul globo usa una richiesta HTTP a ip-api.com (disattivabile nelle impostazioni).

## Build e release
La build gira su GitHub Actions a ogni push su `main` (o a mano con `workflow_dispatch`) e compila in parallelo Windows, Linux x86_64 e Linux aarch64. Ogni lavoro produce un artifact; il workflow non crea release né parte sui tag: le release si fanno a mano.

Per compilare da sorgente serve un Rust recente (le dipendenze richiedono almeno 1.88). Su Linux servono anche i pacchetti di sviluppo `pkg-config`, `libasound2-dev`, `libxkbcommon-dev`, `libwayland-dev`, `libx11-dev`, `libxcb1-dev` e `libgl1-mesa-dev`; poi `cargo build --release`.

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
