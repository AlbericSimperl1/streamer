# Streamer (PC-kant) — Moonlight-compatibele server

## Doel
Een native Moonlight-compatibele streamingserver voor Linux (Hyprland), geschreven in Rust. Geen Sunshine als onderlaag: de server spreekt zelf de protocollen die Moonlight-clients verwachten, zodat elke standaard Moonlight-client kan verbinden.

## Huidige stack (PC)
- Rust met egui-GUI (bewust gekozen boven Sunshine's browsergebaseerde, tekstuele configuratie)
- Virtuele headless monitor op Hyprland
- Capture via PipeWire
- Encoding met ffmpeg (x264), momenteel over UDP

## Beslissing: alleen Moonlight, geen eigen iPad-app meer
- De eigen iPad-client (Swift + Rust core) wordt **volledig laten vallen**.
- Alles wat alleen diende voor die client (eigen handshake, eigen transportprotocol, iPad-specifieke code en documentatie) mag verwijderd of herschreven worden.
- De server moet volledig compatibel zijn met de Moonlight-clientprotocollen (pairing, app/stream-sessie, video, audio, input). Waar het huidige eigen UDP-protocol afwijkt, wordt het vervangen door het Moonlight-protocol.

## Eis 1: resolutie van de virtuele monitor makkelijk togglen
- De gebruiker moet de oriëntatie van de virtuele monitor met één actie kunnen wisselen: `A×B` ⇄ `B×A` (bijv. 1920×1080 ⇄ 1080×1920).
- Beschikbaar in de egui-GUI (knop/toggle). Handig is ook een aanroepbare actie (CLI-flag of hotkey).
- Bij het togglen moet de Hyprland-monitor opnieuw geconfigureerd worden (resolutie, positie) en de capture/encoder-pipeline netjes herstart of aangepast worden, zodat een lopende stream niet crasht.
- Moonlight vraagt zelf ook een resolutie aan bij het starten van een sessie; houd rekening met hoe die aanvraag zich verhoudt tot de ingestelde toggle.

## Eis 2: echte resolutie van het hoofdscherm uitlezen
- De hardcoded 1080p voor het hoofdscherm moet weg.
- Lees de werkelijke resolutie (en liefst refresh rate en schaal) van het hoofdscherm dynamisch uit via Hyprland (bijv. `hyprctl monitors -j`).
- Gebruik deze waarden overal waar nu 1080p wordt aangenomen: standaardresolutie van de virtuele monitor, positionering ten opzichte van het hoofdscherm, en de resoluties die aan Moonlight worden aangeboden.
- Als uitlezen mislukt, val terug op een duidelijke foutmelding of een expliciet gekozen standaardwaarde, niet stilzwijgend op 1080p.

## Werkafspraken
- Taal van communicatie: Nederlands. Code, identifiers en commentaar mogen Engels blijven.
- Houd de GUI de primaire manier om te configureren.
- Maak kleine, testbare stappen: eerst monitor-uitlezing en toggle (Eis 1 en 2), daarna de Moonlight-protocolcompatibiliteit.

## Voorgestelde volgorde
1. Hoofdscherm dynamisch uitlezen (Eis 2).
2. Toggle `A×B` ⇄ `B×A` voor de virtuele monitor, inclusief pipeline-herstart (Eis 1).
3. iPad-specifieke code en docs opruimen.
4. Moonlight-protocol implementeren: discovery/pairing, sessiestart, video, audio, input.
