# edex-ui-rs

Port di [eDEX-UI](https://github.com/GitSquared/edex-ui) da Electron a Rust (egui con backend OpenGL), pensato per PC vecchi con GPU datate.

Layout e tema "tron" ricalcano l'originale: colonne laterali (orologio, sistema, CPU, RAM, processi / rete, globo, traffico), shell a 5 tab, filesystem a icone e tastiera virtuale.

## Licenza
eDEX-UI è sotto GPL-3.0, quindi questo port è distribuito sotto GPL-3.0 (vedi `LICENSE`): chi riceve un binario ha diritto al sorgente.

## Font
- **Fira Mono** è incorporato (SIL OFL 1.1).
- **United Sans** (House Industries) ha una licenza che non ne consente la ridistribuzione, quindi non è inclusa.
  Per il look identico copia `united_sans_medium.ttf` e `united_sans_light.ttf` accanto all'exe (o in una cartella `fonts/`).
  Senza, l'app usa il font proporzionale di egui.

## Da fare
Audio, boot screen, globo con i continenti, icone file per estensione, altri temi, build Linux.
