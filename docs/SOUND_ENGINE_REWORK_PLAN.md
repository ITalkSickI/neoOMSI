# Sound-Engine-Rework mit anschließender Steam-Audio-Integration

**Status:** Planungsgrundlage  
**Reihenfolge:** zuerst Sound-Engine-Rework, danach Steam Audio  
**Leitentscheidung:** Die OMSI-Runtime und der Rust-Mixer bleiben die Sound Engine. CPAL übernimmt weiterhin die Geräteausgabe. Steam Audio ergänzt später die räumliche Verarbeitung und ersetzt weder SoundCfg-Auswertung noch Wiedergabeverwaltung oder Mixer.

## Ziel

Die Audioverarbeitung soll leichter erweiterbar, stabiler und klanglich hochwertiger werden, ohne die von OMSI erwarteten SoundCfg- und Script-Regeln zu verlieren. Der erste Abschnitt bringt die bestehende Engine in eine klare, verlässliche Form und funktioniert vollständig ohne Steam Audio. Erst nach dessen Abnahme wird Steam Audio für HRTF-Spatialization, Occlusion, Transmission und Reflexionen ergänzt.

Ein Entwickler soll Steam Audio nicht separat installieren oder manuell herunterladen müssen. Benötigte Header, Bindings, Bibliotheken und Lizenzhinweise müssen im Repository oder in den normalen, gepinnten Build- und Packaging-Schritten verfügbar sein. Ein frischer Checkout soll mit den üblichen Projektbefehlen bauen können.

```text
SoundCfg / Script
        ↓
OMSI-Runtime: Bedingungen, Trigger, Soundzustände, Auswahl und Budgets
        ↓ geordnete Wiedergabeereignisse und Parameterzustände
Audio-Thread: Stimmen, Resampling, Effekte und Mixing
        ↓
CPAL: Geräteformat, Ausgabe und Gerätewechsel
```

Nach Phase 1 kann Steam Audio als räumliche Verarbeitung zwischen Wiedergabe und Mischung ergänzt werden. Die OMSI-Runtime behält dabei die Entscheidung, welche Sounds starten, laufen und enden.

## Was die Untersuchung von `omsi.exe` für den Plan bedeutet

Die Referenz unter `H:\marcel_omsi` wurde statisch untersucht. Die enthaltene EXE stimmt laut Manifest und Hash mit OMSI 2.2.032 überein. Die Untersuchung umfasste SoundPack-Parsing, Sound-Updates, Trigger, Buffer-Lebensdauer, Budgetierung und Hallanbindung. Es fand kein Hörvergleich mit einer laufenden OMSI-Installation statt. DirectSounds interne DSP-Implementierung ist in der Referenz nicht enthalten; aus ihr lässt sich daher das Steuerungsverhalten, aber keine bitgenaue Klanggleichheit ableiten.

| Befund in OMSI | Konsequenz für neoOMSI |
|---|---|
| `TSoundPack` und `TSound` verwalten Konfiguration, Bedingungen und Wiedergabezustände. | Diese Regeln gehören in eine eigenständige OMSI-Runtime und nicht in den DSP-Renderer. |
| Der untersuchte Ladepfad setzt DirectSound-Flags für Frequenz, Pan, Lautstärke und FX, aber kein `CTRL3D`. | Ein eigener Mixer mit CPAL als Geräteausgabe passt grundsätzlich zum untersuchten Pfad. CPAL selbst ist dabei kein Mixer. |
| Entfernung, Pan, Doppler, Frequenz und logarithmische Lautstärke werden im untersuchten Sound-Update berechnet. | Diese Regeln müssen im Legacy-Pfad unabhängig von Steam Audio erhalten und geprüft werden. Die inverse Entfernungskurve kommt in diesem Pfad nicht automatisch von DirectSound3D. |
| Ein Trigger setzt den Triggerzustand, löscht den gespeicherten Lautstärkepeak und stößt unmittelbar ein Update an. | Ereignisreihenfolge und Variablenwerte zum Auslösezeitpunkt sind wichtig. Mehrere Ereignisse eines Frames dürfen nicht zusammenfallen. |
| Loopstarts können mit einem zufälligen Buffer-Offset beginnen; One-Shots starten bei Position null. | Ein pauschaler Start aller Stimmen bei null bildet den Referenzpfad nicht ab. |
| `[checkloading]` beeinflusst das Laden und Freigeben von Buffern. | Das Flag wird in neoOMSI geparst, aber derzeit nicht vollständig zur Laufzeit ausgewertet. |
| `[onlyone]` verwendet eine zentrale, dateibezogene Verwaltung. | Eine Prüfung nur innerhalb desselben Sound-Eintrags reicht nicht. Der genaue Geltungsbereich braucht einen kontrollierten Laufzeitvergleich. |
| SoundPacks werden nach normierter Entfernung sortiert und anschließend budgetiert. | Das unterscheidet sich von einer reinen Sortierung einzelner Stimmen nach Lautstärke. |
| Der FX-Pfad verwendet `IDirectSoundFXWavesReverb` mit Hallmix und Hallzeit. | Bestehende Hallvorgaben brauchen eine ausdrückliche Zuordnung zu neuer Verarbeitung. |

Unsichere Details werden vor einer Kompatibilitätsentscheidung mit kleinen OMSI-Laufzeittests geprüft. Die Ghidra-Ausgaben und alten Kommentare sind Hinweise; für Entscheidungen zählen die untersuchten Assembly-Stellen und reproduzierbare Vergleiche.

## Phase 1 – Sound Engine refactoren

**Abnahmeziel:** neoOMSI besitzt eine eigenständige, gut strukturierte Sound Engine mit überprüften OMSI-Regeln. Alle Anwendungen laufen ohne Steam-Audio-Abhängigkeit. Steam Audio wird erst begonnen, wenn diese Phase abgeschlossen und akzeptiert ist.

### 1. Verbindliches Verhalten und Testgrundlage festhalten

Vor dem Umbau werden SoundCfg- und Script-Fälle als Eingaben, erwartete Zustandsübergänge und hörbare Parameter beschrieben. Wo sich das Verhalten aus dem untersuchten Programm nicht sicher ableiten lässt, wird ein kompakter Vergleich in laufendem OMSI geplant, bevor die Regel festgeschrieben wird.

Die Fälle umfassen mindestens:

- `[sound]`, `[loopsound]`, `[noloop]`, Trigger-Sounds und Datei-Trigger;
- Bedingungen, Pitch- und Volume-Kurven, negative Volume-Variablen und Zeitvariablen;
- Viewpoint, Innen-/Außensicht und `Snd_OutsideVol`;
- Wiederholung eines Triggers im selben Frame sowie Reihenfolge normaler und dateibasierter Trigger;
- zufällige Loopstartphase, One-Shot-Neustart und Ende einer Stimme;
- `[checkloading]`, `[onlyone]`, fehlende Audiodateien und erneutes Laden;
- SoundPack-Reihenfolge, Pack-Budget und Verhalten einer später wieder hörbaren Stimme;
- Pausieren, Zeitfortschritt, Spielerfahrzeug, KI, Trailer und gekoppelte SoundSets.

Ein Offline-Renderer und synthetische SoundCfg-/Script-Fixtures sollen Wiedergabeaktionen sowie Signalverarbeitung ohne angeschlossenes Audiogerät reproduzierbar machen. Hörtests bleiben zusätzlich nötig, weil numerische Tests keine wahrgenommene Klangqualität belegen.

**Abnahme:** Für die genannten Fälle liegt eine nachvollziehbare Verhaltensbeschreibung vor. Unsichere Annahmen sind als offen markiert oder durch OMSI-Vergleiche geklärt. Es gibt wiederholbare Offline-Fixtures als Grundlage für die Umstrukturierung.

### 2. Verantwortlichkeiten trennen, öffentliche Fassade zunächst erhalten

Die bestehende `AudioEngine` bleibt während der Umstrukturierung zunächst die öffentliche Schnittstelle. Ihre Aufgaben werden intern in Bereiche mit klaren Zuständigkeiten zerlegt:

| Bereich | Verantwortung |
|---|---|
| `soundset` / `runtime` | OMSI-Regeln, Bedingungen, Trigger, Soundzustände und Pack-Auswahl |
| `engine` / `commands` | Wiedergabebefehle, geordnete Ereignisse und Rückmeldungen |
| `voice` | Playback-Position, Looping, Wiedergabestatus und kurze Start-/Stop-Übergänge |
| `renderer` | Resampling, Stimmenverarbeitung, Busse und Summierung |
| `device` | CPAL-Ausgabeformat, Kanalbelegung, Geräteverlust und Wiederverbindung |
| `assets` / `stream` | Decode, Clip-Cache, WAV-Dateien und Radio-Streams |
| `spatial` / `acoustics` | Schnittstelle für räumliche Verarbeitung; zunächst ohne Steam-Audio-Zwang |

Die Grenzen sollen so gestaltet sein, dass Wiedergabeentscheidungen unabhängig vom gewählten Renderer bleiben. Größere Änderungen am öffentlichen Rust-API erfolgen erst, wenn interne Zuständigkeiten stehen und Aufrufer einzeln migriert werden können.

**Abnahme:** Die Zuständigkeiten sind in Modulen und Typen erkennbar. Die vorhandenen Aufrufer können schrittweise migriert werden; Renderlogik entscheidet nicht selbst über SoundCfg-Bedingungen.

### 3. Audio-Thread und Echtzeitpfad stabilisieren

Der Audio-Thread erhält exklusiven Besitz an aktiven Stimmen und den DSP-Zuständen. Gameplay sendet begrenzte Befehle und zusammenhängende Parameterzustände. Der Rückkanal meldet Wiedergabestatus, damit Aufrufer den Stimmenbestand nicht pro Frame unter einem gemeinsamen Mutex abfragen müssen.

Der CPAL-Callback soll eine vorhersehbare, begrenzte Menge Arbeit ausführen. Laden und Decodieren, Logging, temporäre Allokationen, Sortierungen großer Listen und kostspielige Freigaben gehören nicht in den Callback. Parameteränderungen dürfen zusammengefasst werden; Start-, Stop- und Triggerereignisse müssen in ihrer Reihenfolge erhalten bleiben. Die Queue braucht definierte Kapazitäts- und Überlaufregeln, damit bei Last weder unbegrenzt Speicher wächst noch wichtige Ereignisse still verloren gehen.

**Abnahme:** Der Callback blockiert nicht auf Gameplay- oder Decoder-Locks, erzeugt keine unerwarteten Heap-Allokationen und führt keine Dateizugriffe aus. Überlast- und Geräteverlustfälle enden in einem definierten Zustand.

### 4. OMSI-Runtime und Ereignisverarbeitung korrigieren

Getrennte Listen für normale Trigger und Datei-Trigger werden durch einen gemeinsamen, geordneten Ereignisstrom ersetzt. Ein Ereignis enthält Quelle, Sequenz beziehungsweise Reihenfolge, Zeitpunkt, optionalen Dateinamen und die Variablen, die für seine Auswertung gebraucht werden. Dadurch bleiben wiederholte Auslösungen in einem Frame erhalten und die Reihenfolge über Player, KI, Szenerie und gekoppelte Fahrzeuge hinweg nachvollziehbar.

Soundzustände werden ausdrücklich modelliert, beispielsweise: nicht geladen, lädt, startbereit, laufend, logisch gestoppt, durch OMSI-Regeln unterdrückt, vom Renderer virtualisiert und beendet. Asset-Cache und logischer Wiedergabezustand werden getrennt. Ein asynchroner Ladevorgang darf ein bereits angenommenes Ereignis nicht unbemerkt verschlucken.

Hier werden die geprüften OMSI-Regeln umgesetzt: Loop- und One-Shot-Startpositionen, Trigger-Neustart, `[checkloading]`, dateibezogene `[onlyone]`-Verwaltung und SoundPack-Budgetierung. Die genaue Zeitbasis und Behandlung bei Pause wird anhand der Vergleichsfälle entschieden; die aktuelle `Instant`-Nutzung wird nicht pauschal durch Simulationszeit ersetzt.

**Abnahme:** Wiederholte Ereignisse und ihre Snapshots bleiben erhalten. Player, KI, Szenerie, Multiplayer-Sounds und Trailer verwenden dieselben Runtime-Regeln. Asset-Ladefehler und erneutes Laden hinterlassen keinen widersprüchlichen logischen Wiedergabestatus.

### 5. Legacy-Wiedergabeparameter und Budgetierung sauber nachbilden

Die Runtime liefert getrennte Werte für Aufnahmepegel, Script-Lautstärke, Entfernung, Innen-/Außenübertragung, Wiedergabefrequenz und räumliche Richtung. Der Renderer wendet sie in einer dokumentierten Reihenfolge an. Damit wird verhindert, dass Pegel früh abgeschnitten werden oder Entfernung und Innenraumwirkung versehentlich zweimal in die Lautstärke eingehen.

Der Legacy-Pfad bildet das beobachtete DirectSound-Pan-Verhalten nach. Dabei wird ein Kanal relativ zum anderen gedämpft; die aktuelle Panningformel verhält sich anders. Volume-Konvertierung, Grenzwerte und Frequenzverhalten werden mit den OMSI-Fixtures verglichen, bevor sie als kompatibel gelten.

OMSI-SoundPack-Zulassung und technische Rendererlimits bleiben zwei getrennte Entscheidungen. Ein von OMSI gestoppter Sound hat andere Lebensdauerregeln als eine Stimme, die nur aus Leistungsgründen virtualisiert wird. Bei erneuter Hörbarkeit muss der definierte Soundtyp entscheiden, ob seine Phase weiterläuft, neu beginnt oder erneut zugelassen werden muss.

**Abnahme:** Ein Wechsel des Renderers verändert keine Bedingungen, Sound-Auswahl oder Trigger-Lebensdauer. Budgetgrenzen haben dokumentierte Folgen und lassen sich mit vielen Fahrzeugen reproduzieren.

### 6. Allgemeine Klangqualität, Geräteausgabe und alle Aufrufer abschließen

Erst wenn Runtime und Legacy-Parameter stehen, werden die allgemeinen Qualitäts- und Robustheitsverbesserungen eingebaut:

- bandbegrenztes Resampling für variable Motor- und Soundfrequenzen;
- zeitabhängige Glättung für Lautstärke, Frequenz und Filter, kurze Start-/Stop-Fades und geprüfte Loopübergänge;
- explizite Unterstützung geeigneter CPAL-Geräteformate und Kanalbelegungen statt der Annahme eines einzelnen F32-Ausgabeformats;
- Zustandsmaschine für Start ohne Gerät, Geräteverlust und Wiederverbindung;
- Radio-Decoding in begrenzte Streamingbuffer ohne gemeinsamen Decoder-/Mixer-Lock;
- validierte WAV-Verarbeitung und höhere Samplepräzision, wo sie sinnvoll ist;
- getrennte Mixerbusse für Fahrzeuge, Umgebung, Fahrgäste, Ansagen, Radio und Interface;
- abgestimmter Headroom und Limiter für dichte Verkehrssituationen.

Alle Sound-Erzeuger werden durchgeprüft: Spielerfahrzeug, KI, Multiplayer, Trailer, Szenerie, Wetter, Schritte, Radio, Ansagen und HTML-Sounds. Vor dem Steam-Audio-Teil wird auf repräsentativen Situationen gehört und die CPU- sowie Speicherlast erfasst.

**Abnahme und Phasentor:** Die Engine läuft vollständig ohne Steam Audio, ihre OMSI-Verhaltensfixtures bestehen, Gerätewechsel und viele gleichzeitige Quellen sind kontrolliert, und die Klangqualität ist mindestens so gut wie zuvor. Erst nach dieser Abnahme beginnt Phase 2.

## Phase 2 – Steam Audio als räumliche Erweiterung integrieren

**Abnahmeziel:** Steam Audio verbessert die räumliche Darstellung und akustische Umgebung. OMSI-Runtime, SoundCfg-Auswertung, Wiedergabeauswahl und CPAL-Ausgabe bleiben Bestandteile der neoOMSI-Engine.

### 7. SDK-Version, Bindings und Verteilung festlegen

Zum Planungsstand ist Steam Audio 4.8.1 die gepinnte Referenz. Vor Umsetzung wird die Release-Asset-Liste gegen alle tatsächlich unterstützten Ziele geprüft. Falls ein benötigtes Artefakt fehlt, wird es aus der gepinnten Quelle zentral gebaut und im normalen CI-/Packaging-Ablauf bereitgestellt. Entwickler sollen keine separate Installation und keinen manuellen Download durchführen müssen.

Ein kleines Binding-Crate kapselt die C-API und native Handles hinter sicheren Rust-Typen. Es verwaltet Erzeugung, Thread-Zuständigkeit, Fehler und Freigabe von Kontext, Szene, Quellen, Effekten und Simulationsergebnissen. ABI-Version, Checksummen, Lizenztexte und Hinweise für Drittkomponenten werden festgehalten. Build- und Releasepakete müssen die passenden nativen Bibliotheken für die tatsächlich unterstützten Architekturen automatisch enthalten.

**Abnahme:** Ein frischer Checkout kann die Audio-Unterstützung mit dem normalen Projekt-Setup bauen. Releasepakete starten ohne separat installiertes Steam-Audio-SDK. Lizenz- und Drittkomponentenhinweise sind enthalten.

### 8. Koordinaten, Listener und grundlegende Spatialization integrieren

Zunächst werden Koordinaten, Orientierung und Geräteausgabe festgelegt. neoOMSI verwendet Weltachsen X = Ost, Y = Nord und Z = oben. Steam Audio verwendet X = rechts, Y = oben und negative Z = vorwärts. Die Umrechnung, Listener-Basis und Drehrichtung werden mit bekannten Testpositionen geprüft, bevor echte Spielquellen migriert werden.

Danach werden Kopfhörer-HRTF und passende Lautsprecherlayouts eingebunden. Quelle und Listener erhalten geglättete Positions- und Richtungsupdates. Die bereits in Phase 1 definierte Entfernung wird genau einmal angewandt; Steam Audio darf nicht dieselbe Entfernung ein zweites Mal dämpfen. Ein umschaltbarer Legacy-Pfad bleibt während Vergleich und Einführung verfügbar.

**Abnahme:** Vorne, hinten, links, rechts und Bewegung um den Listener herum werden auf Kopfhörern nachvollziehbar wiedergegeben. Unterstützte Lautsprecherlayouts erhalten eine passende Ausgabe. OMSI-Fixtures für Auswahl und Lebensdauer verhalten sich wie zuvor.

### 9. Akustische Geometrie aus Welt- und Fahrzeugdaten aufbauen

Die World-/Tile-Pipeline liefert vereinfachte akustische Geometrie aus geeigneten CPU-Meshes. Render- und Kollisionsgeometrie werden nicht ungeprüft übernommen: Transparente Flächen, dekorative Details und ungeeignete Kollisionskörper können falsche Schallhindernisse erzeugen. Geometrie wird nach akustischer Eignung bewertet und kann vereinfachte Standardmaterialien sowie optionale Material-Overrides erhalten.

Fahrzeugkörper, gekoppelte Teile, Türen und Fenster brauchen eigene Aktualisierungsregeln, weil sie ihre Position oder Offenheit ändern können. Tile-Laden und -Entladen müssen die zugehörigen Akustikdaten sicher registrieren und entfernen. Dreiecksorientierung und Koordinatenwandel werden anhand einfacher Testflächen validiert.

**Abnahme:** Einfache Wände und Fahrzeugflächen werden als Hindernisse erkannt. Tile-Wechsel und bewegliche Fahrzeugteile erzeugen keine veralteten Referenzen in der Simulation.

### 10. Occlusion und Transmission ergänzen

Steam Audios direkte Simulation liefert die Grundlage für Abschattung und Übertragung durch Hindernisse. Die Aktualisierung läuft außerhalb des Audio-Callbacks und wird für aktive oder hörbare Quellen priorisiert. Innen- und Außenwerte, Materialdurchlässigkeit sowie Tür- und Fensterzustände werden in einen klaren Satz direkter Effektparameter übersetzt.

`Snd_OutsideVol` bleibt für bestehende Inhalte ein kompatibler Eingang, besonders wenn keine brauchbare Fahrzeuggeometrie vorhanden ist. Es darf jedoch nicht gleichzeitig mit einer gleichartigen Steam-Audio-Dämpfung denselben Weg doppelt absenken. Für fehlende Geometrie braucht es definierte Fallbacks.

**Abnahme:** Eine Wand, ein Fahrzeugkörper und geöffnete beziehungsweise geschlossene Tür- und Fensterzustände verändern den Direktschall erwartungsgemäß. Quellen mit unvollständigen Geometriedaten bleiben hörbar und verhalten sich definiert.

### 11. Reflexionen, Hall, Tunnel und Innenräume ergänzen

Nach der direkten Ausbreitung folgen Reflexionen und räumlicher Hall für Tunnel, Unterführungen, Innenräume und andere relevante Orte. Vorhandene SoundCfg-Reverbvorgaben erhalten eine definierte Rolle: Sie werden entweder nachvollziehbar auf neue Hallparameter abgebildet oder als Fallback verwendet, wenn für eine Umgebung keine räumliche Simulation vorliegt.

Reflexionssimulation und Datenaufbereitung laufen auf Worker-Threads. Ergebnisse werden mit sicherer Lebensdauer an den Renderer übergeben. Aktualisierungsraten, Anzahl aktiver Quellen, Rechenzeit und Speicher erhalten eigene Budgets. Mehrere Motorlayer am selben Ort sollen nach Möglichkeit gemeinsame Ausbreitungsdaten nutzen. Längere Nachhallfahnen müssen auch nach dem Ende einer Quellstimme kontrolliert ausklingen.

**Abnahme:** Tunnel und Innenräume unterscheiden sich hörbar und nachvollziehbar von offenen Bereichen. Simulation blockiert weder den CPAL-Callback noch den Spielframe; bei hoher Last wird die Qualität kontrolliert reduziert.

### 12. Baking, Plattformen, Migration und endgültige Abnahme

Wenn dynamische Simulation stabil ist, werden Baking und Pathing als spätere Optimierung ergänzt. Bestehende Maps müssen ohne vorberechnete Akustikdaten funktionieren. Steam Audio wird schrittweise für Spielerfahrzeuge, KI, Trailer, Szenerie, Radio beziehungsweise ausgewählte Streams und Umgebungstöne aktiviert; ungeeignete Quellen können beim Legacy-Pfad bleiben.

CI und Releasepakete werden für jede unterstützte Plattform und Architektur geprüft. Tests umfassen wiederholte Trigger, viele Fahrzeuge mit derselben Aufnahme, dateibezogenes `[onlyone]`, Pack-Budgets, Tür-/Fensterwechsel, Kameramodi, Map-Unloading, Stream-Ende und Gerätewechsel. Hörvergleiche, CPU-Zeit, Speicher, Aussetzer und Paketgröße entscheiden über die endgültigen Standardbudgets.

**Abnahme:** Ein Release startet ohne installiertes SDK, behält die Kompatibilität der Runtime-Fixtures und liefert eine messbar sowie hörbar verbesserte räumliche Wiedergabe. Für Geräte oder Situationen ohne Steam-Audio-Ausgabe greift ein kontrollierter Fallback.

## Risiken und Entscheidungen, die vor oder während der Umsetzung geklärt werden

- **Verhalten gegenüber Klanggleichheit:** Die Referenz erklärt OMSIs Steuerlogik, aber nicht DirectSounds internen DSP. Ziel ist überprüfbares Verhalten und gute Klangqualität, keine unbelegte bitgenaue Gleichheit.
- **Unklare Legacy-Regeln:** Geltungsbereich von `[onlyone]`, Timing bei Pause, einzelne Triggerdetails und Pack-Budgetgrenzen müssen an realen OMSI-Szenarien geprüft werden.
- **SDK und Plattformen:** Verfügbare vorgebaute Bibliotheken können je nach Release und Zielarchitektur abweichen. Fehlende Ziele müssen zentral gebaut und im CI gepflegt werden.
- **Audio-Thread-Sicherheit:** Steam-Audio-Simulationsdaten dürfen nicht während einer Aktualisierung freigegeben oder überschrieben werden, wenn der Renderer sie noch verwendet.
- **Geometriequalität:** Vorhandene Render- und Kollisionsdaten sind nicht automatisch gute Akustikdaten. Materialzuordnung und bewegliche Fahrzeugteile brauchen eine klare Datenquelle.
- **Leistung:** Direkte Simulation und Reflexionen benötigen getrennte Budgets. Grenzwerte sollen aus repräsentativen Strecken mit vielen Fahrzeugen abgeleitet werden.
- **Doppelte Dämpfung:** OMSI-Entfernung, Innenraumfilter, `Snd_OutsideVol` und Steam-Audio-Effekte müssen einen eindeutigen Verantwortlichen pro akustischem Effekt haben.

## Vorgeschlagene Implementierungsaufteilung

1. **Umsetzungsblock A:** Phase 1, Schritte 1–3: Verhalten beschreiben, Offline-Grundlage schaffen, interne Grenzen ziehen und Audio-Thread stabilisieren.
2. **Umsetzungsblock B:** Phase 1, Schritte 4–6: OMSI-Runtime-Regeln korrigieren, Legacy-Mixing und Ausgabequalität abschließen, alle Aufrufer migrieren und das Phasentor abnehmen.
3. **Umsetzungsblock C:** Phase 2, Schritte 7–8: SDK automatisch bereitstellen, Bindings kapseln und grundlegende Spatialization integrieren.
4. **Umsetzungsblock D:** Phase 2, Schritte 9–12: akustische Welt- und Fahrzeugdaten, Occlusion, Transmission, Reflexionen, Plattformpakete und Gesamtabnahme ergänzen.

Jeder Block soll als eigener Review-Schritt abnehmbar sein. Die Übergabe von Block B an C ist das feste Tor zwischen allgemeinem Sound-Engine-Rework und Steam-Audio-Implementierung.

## Quellen und Referenzen

**Lokale Referenzanalyse:** `H:\marcel_omsi` — insbesondere `assembly/0074ff2c.asm`, `assembly/00750444.asm`, `assembly/0074f3ec.asm`, `assembly/0074fbb4.asm`, `assembly/0074fc98.asm`, `assembly/007513c0.asm`, `decompiled/0074f600.c` und `assembly/00404b58.asm`. Die Analyse ist statisch und bezieht sich auf OMSI 2.2.032.

**Steam Audio:**

- [Steam Audio Releases](https://github.com/ValveSoftware/steam-audio/releases) — SDK-Versionen und Distributionsartefakte.
- [Steam Audio C API: Getting Started](https://valvesoftware.github.io/steam-audio/doc/capi/getting-started.html) — Initialisierung und Audioformat.
- [Steam Audio C API Guide](https://valvesoftware.github.io/steam-audio/doc/capi/guide.html) — Direktschall, Spatialization und Reflexionen.
- [Simulation API](https://valvesoftware.github.io/steam-audio/doc/capi/simulation.html) — Simulationszustand und getrennte Aktualisierung.
- [Geometry API](https://valvesoftware.github.io/steam-audio/doc/capi/geometry.html) und [Scene API](https://valvesoftware.github.io/steam-audio/doc/capi/scene.html) — Akustikgeometrie, Dreiecke und Materialien.
- [Reflections Effect](https://valvesoftware.github.io/steam-audio/doc/capi/reflections-effect.html) und [Baking](https://valvesoftware.github.io/steam-audio/doc/capi/baking.html) — Hallfahnen und vorberechnete Daten.
- [Steam Audio Lizenz](https://github.com/ValveSoftware/steam-audio/blob/v4.8.1/LICENSE.md) und [Drittkomponenten](https://github.com/ValveSoftware/steam-audio/blob/v4.8.1/core/THIRDPARTY.md) — Hinweise für Verteilung und Notices.

**DirectSound-Referenz:** [Microsoft DirectSound Header](https://raw.githubusercontent.com/microsoft/win32metadata/main/generation/WinSDK/RecompiledIdlHeaders/um/dsound.h) und [IDirectSoundFXWavesReverb::SetAllParameters](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/mt708938(v=vs.85)).
