# Sound-Engine: kurzer Spieltest

Die aktuelle Test-EXE liegt unter
`H:\neoOMSI\dist\windows-audio-stage6-1\neoomsi.exe` und meldet
`0.2.0-audio-stage6.1`. Sie wurde frisch als optimierter `dev-release`-Build gebaut.
Alle 90 Audio-Tests bestehen; Versions-/Hilfestart und Offline-WAV wurden geprüft.

Starte diese EXE direkt. Ihr separater Ordner sorgt dafür, dass auch der integrierte
Launcher die neue Spielversion auswählt. Vorhandene Karten/Fahrzeuge sind verknüpft;
die bisherige `dist/windows-dev/neoomsi.exe` bleibt als Vergleichsversion erhalten.
Die Microsoft C++ Build Tools und das Windows SDK wurden mit deiner Zustimmung installiert.

## Speziell in Stage 6.1 prüfen

1. C2 EN 6/BVG: im Stand nach links/rechts drehen. Der Motor soll auf beiden Ohren
   hörbar bleiben; nicht-3D-Innenaufnahmen sind zentriert, 3D-Quellen haben noch Richtung.
2. Alle Türen schließen, dann eine öffnen: Außenaufnahmen des eigenen Busses sollen
   geschlossen leiser/dumpfer sein und geöffnet deutlicher durchkommen. Innenaufnahmen
   bleiben erhalten; Schalter für beide Ansichten werden nicht pauschal gedämpft.
3. Bus/Verkehr neu laden oder in Hörweite kommen lassen: kein kurz lauter Außensound
   vor der Innenraumdämpfung. Auch die erste Ausgabe respektiert die eingestellte Lautstärke.
4. Tasten/Schalter mehrmals auslösen und Motor bei konstanter Drehzahl hören. Der
   zusätzliche Kabinenhall ist weg, Resampling-Zwischenpositionen sind jetzt stufenlos.
   Melde bitte, ob Rauschen/Fiepsen verschwunden ist oder welcher Sound es weiterhin hat.

## Was sich hörbar ändern soll

- **Motor / Lüfter / Rollgeräusche:** bei steigender Drehzahl weniger künstliches
  Pfeifen oder metallische Nebentöne durch Resampling. Höre besonders beim langsamen
  Hochdrehen, abrupten Gasgeben und Ausrollen hin.
- **Übergänge:** weniger Knacken beim Starten/Stoppen, Lautstärkewechseln, Öffnen von
  Türen/Fenstern und Wechsel zwischen Innen- und Außenansicht. Loops sollen ohne
  regelmäßiges Knacken oder kurze Löcher weiterlaufen. Kurze Sounds haben 3-ms-Fades;
  prüfe, ob Türen, Schalter und andere Transienten trotzdem direkt genug wirken.
- **Lautstärke / dichter Verkehr:** Stage 6 reserviert 6 dB vor dem Limiter. Die Ausgabe
  kann leiser wirken; Umgebung und Radio sind zusätzlich etwas niedriger gewichtet.
  Bei vielen Fahrzeugen sollen Verzerrungen abnehmen. Achte auf hörbares Pumpen, zu
  starkes Wegdrücken des eigenen Motors oder unverständliche Ansagen/Fahrgaststimmen.
- **Position / Eigenfahrzeug:** links und rechts müssen zur Soundposition passen.
  Der Legacy-Pan dämpft vor allem den gegenüberliegenden Kanal. Im eigenen Fahrzeug
  darf die Tonhöhe bei gleichbleibender Drehzahl nicht durch Kamerabewegung zittern.
- **Gelenk-/Anhängerbereiche:** Motor, Türen und Datei-Ansagen im hinteren Teil müssen
  zuverlässig ankommen. Datei-Ereignisse und Variablenwerte vom Auslösezeitpunkt werden
  jetzt auch an gekoppelte Teile weitergegeben.
- **Radio:** beim Senderwechsel kein harter Klick; nach einer Netzpause kurz still und
  anschließend gepuffert weiter. Speicher darf bei längerem Radiohören nicht wegen
  eines unbegrenzt wachsenden Audiopuffers stetig steigen.
- **Geräte:** Ausgabegerät während des Spiels wechseln, abziehen und wieder anschließen.
  Motor und Radio sollen wiederkommen. Loopphase darf bei Wiederverbindung neu starten;
  während eines Ausfalls ausgelöste Hupen oder Schritte sollen nicht verspätet erklingen.

## Praktischer Ablauf (10–15 Minuten)

1. Bekanntes Fahrzeug und bekannte Karte, möglichst dieselben Lautstärkeeinstellungen
   wie zuvor. Erst allein am Stand: Innen-/Außensicht, Türen/Fenster, Hupe, Schalter und Fahrkartenentwerter.
2. Einmal von Leerlauf bis hoher Drehzahl beschleunigen und ausrollen. Danach einen
   konstanten Motor-/Lüfterloop etwa eine Minute hören.
3. An eine stark befahrene Haltestelle: mehrere KI-Fahrzeuge, Regen, Fahrgäste, Ansage
   und Radio zusammen. Klang und Bildraten mit der vorherigen Version vergleichen.
4. Bei einem Gelenkbus besonders den hinteren Teil prüfen; Fahrzeug-/Tile-Wechsel
   durchführen und auf hängenbleibende oder doppelte Sounds achten.
5. Kopfhörer/Ausgabegerät wechseln, während Motor und Radio laufen. Optional das Spiel
   einmal ganz ohne verfügbares Ausgabegerät starten und später eines anschließen.

Bei einem Fehler bitte Fahrzeug, Karte, Situation, Innen-/Außensicht, Audiogerät und
betroffenen Sound nennen. Ein kurzes Video mit Ton hilft beim Vergleich. Falls ein Sound
komplett fehlt, auch die Logmeldung sichern: WAV-Dateien werden jetzt strenger validiert.

Die Tests bestätigen erst nach Durchführung die wahrgenommene Verbesserung. Offene
OMSI-Kompatibilitätsfragen (z.B. Viewpoint, Checkloading, Pack-Budget) wurden nicht geraten.
Vollständige technische Prüfungen und Lastmessung: `STAGE6_TESTING.md`.

## C2-Blinker-Klick: Testpaket 6.2

Der kurze hohe Ton aus dem Video wurde vom Nutzer auch in der Original-Sounddatei
bestaetigt. `dist/windows-audio-stage6-2/neoomsi.exe` verwendet den Engine-Stand 6.1
mit zwei bereinigten lokalen C2-Blinkeraufnahmen. EXE und `Vehicles`-Unterordner
zusammen lassen. Die Klicks sind kuerzer und dunkler; die Originalinstallation
bleibt erhalten. Details und Messungen: [C2_BLINKER_FIEPEN.md](C2_BLINKER_FIEPEN.md).
90 Audio-Tests sind bestanden; beide Override-Pfade wurden im Offline-Mixer geprueft.

## Innenansage bei Aussenkamera: Testpaket 6.3

`dist/windows-audio-stage6-3/neoomsi.exe` behaelt die C2-Klickkorrektur aus 6.2.
Eine normale Innenansage starten und waehrenddessen nach draussen wechseln:
Sie bleibt hoerbar, klingt durch geschlossene Karosserie leiser und dumpfer.
Tueren/Fenster oeffnen: mehr Pegel und Hoehen. Abstand zum Bus vergroessern:
Die Ansage wird leiser. Zurueck innen: normaler Klang ohne zusaetzlichen Hall.
95 Audio-Tests bestanden; die Abstimmung im echten Spiel ist noch zu hoeren.

## Ansagenhall und staerkere Aussendaempfung: Testpaket 6.4

`dist/windows-audio-stage6-4/neoomsi.exe` enthaelt kurzen Hall nur fuer Innenansagen
(25 Prozent Wet, RT60 0,45 s, gedaempfte Hoehen im Hall). Der direkte Anteil wird
abgesenkt. Auf raeumlichen Nachlauf nach Wort-/Satzenden und Sprachverstaendlichkeit
achten; Motor/Schalter sollen weiterhin ohne diesen Ansagenhall bleiben.
Draussen betraegt die Karosserieuebertragung jetzt 0,08/900 Hz bei geschlossenem
Bus und 0,30/4000 Hz bei maximaler Skript-Oeffnung, vor der Entfernungsdaempfung.
Die Aussenwiedergabe soll deutlich leiser sein als in 6.3, auch bei offenen Tueren.
`Ansagen-ohne-Zusatzhall.cmd` erlaubt einen trockenen Vergleich und Aufnahmen mit
bereits enthaltenem Hall; eine automatische Erkennung gibt es nicht.
99 Audio-Tests bestanden. OMSI-Klanggleichheit und reale Lautstaerke sind weiterhin
per Hoertest zu pruefen. Die C2-Klickkorrekturen sind im Paket erhalten.
