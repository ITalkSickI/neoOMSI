# C2: kurzer hoher Ton beim Blinker-Klick

Die am 6. Oktober gemeldete 1,301-s-Aufnahme stammt aus dem Test mit
`Vehicles/MB_C2_EN_BVG/MB_C2_E6_Gn_BVG_main.bus`, Soundkonfiguration
`Sound-10er/47er_main.cfg`. Der Nutzer hat den störenden Ton als kurzen Ton beim
Klicken eingegrenzt und anschließend denselben Ton in der direkt abgespielten
Originalaufnahme `Cockpit/D_blinker_on.wav` bestätigt.

Der bandbegrenzte Wellenformvergleich (1,2–6 kHz) mit der auf 48 kHz umgerechneten
Originaldatei ergibt eine normalisierte Korrelation von 0,927, Start bei 0,4633 s.
Das ist ein Hinweis auf diese Aufnahme als Quelle, keine vollständige Zerlegung
aller Geräusche des Videos. Die Originaldatei enthält die auffälligen Bereiche um
1,5 und 3 kHz. Ein durchgehendes Motor-Fiepen ist mit diesem Befund nicht erklärt.

## Separater Hörtest 6.2

`dist/windows-audio-stage6-2/neoomsi.exe` trägt die Version
`0.2.0-audio-stage6.2`. Die Engine entspricht 6.1. Das Paket enthält zwei lokale
Sound-Overrides unter `Vehicles/MB_C2_EN_BVG/Sound-10er/Cockpit`:
`D_blinker_on.wav` und `D_blinker_off.wav`. Die OMSI-Originalinstallation wird
nicht geändert. Der normale Content-Resolver lädt diese Kopien vor den Originalen.
Andere Fahrzeuge und andere Sounddateien werden nicht gefiltert.

Die Bearbeitung dämpft beide Resonanzbereiche mit zwei parametrischen EQs
(1500/3020 Hz, jeweils 400 Hz Bandbreite und -18 dB). Ein weicher Fade von 60 bis
120 ms entfernt den verrauschten Nachlauf. Rate (44100 Hz), Stereokanäle und
Dateilänge bleiben erhalten. Gemessen über die ersten 120 ms sinkt die Energie
in 1300–1700 und 2800–3250 Hz um etwa 14–16 dB; die Energie in 100–1000 Hz sinkt
um etwa 1,4–1,7 dB. Die Peaks steigen nicht. Der Klick wird kürzer und dunkler;
das ist eine bewusst angepasste Aufnahme und keine OMSI-Konformitätskorrektur.
Ob der subjektiv störende Ton ausreichend beseitigt ist, entscheidet der Hörtest.

Der Patch ist mit `scripts/prepare-c2-click-patch.ps1 -OmsiRoot <Original>
-OutputRoot <separates Paket>` reproduzierbar (lokales FFmpeg benötigt).
`clip_render input.wav output.wav 48000 1 0.54 <Paket> <Originalroot>` rendert
eine reale WAV durch den Offline-Mixer und zeigt den aufgelösten Quellpfad.
Ohne die optionalen Content-Roots kann es Original und Ausgabe direkt vergleichen.
Es öffnet kein Audiogerät.

Zum Vergleich 6.1 und 6.2 am selben Fahrerplatz mit demselben C2 testen: Blinker
an/aus, kurze Klicks und ihr Nachlauf. Zunächst geringe Lautstärke verwenden.
Für die Rückkehr zum unbearbeiteten Sound genügt das Paket 6.1.
