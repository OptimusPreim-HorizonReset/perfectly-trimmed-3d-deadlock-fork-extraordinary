# Konzept2 Roadmap: Stand vs. Anforderung

## Ziel

Dieses Dokument vergleicht den aktuellen Stand des Projekts mit den Anforderungen aus `konzept2.txt` und beschreibt eine Roadmap, um die Galaxien-Atom-Analogie inhaltlich und visuell kohärent umzusetzen.

Das Ziel ist, eine Simulation zu erreichen, die
- ein galaktisches Bild erzeugt, das astronomischer Realität nahekommt,
- gleichzeitig physikalisch sinnvolle Analogien zur elementaren Welt liefert,
- und visuell zwischen SMBH, Bulge, Scheibe, Gas und Satelliten differenziert.

## Anforderungen aus `konzept2.txt`

### Kernanforderungen

1. Eine Galaxie wird als ein einzelnes Atom interpretiert.
2. Der Kern besteht aus:
   - SMBH = Proton / zentrale Ladung / gravitative Identität
   - Bulge = Neutronen / die zentrale stellar-dichte Masse
3. Die Scheibe / der Halo bilden die Elektronenhülle:
   - Scheibe / Arme = Orbitalsterne
   - Gaswolken & Staub = freie Elektronen / reaktives Medium
   - Satelliten / Kugelhaufen = Valenzelektronen / äußere Bindungsebene

### Systematik

4. Das `Periodensystem` wird durch galaktische Eigenschaften abgebildet:
   - Ordnungszahl ↔ Gesamtmasse / Elementarität
   - Periode ↔ Entwicklungszustand / Morphologie
   - Gruppe ↔ Morphologietyp / Aktivität (Zwerg, Spiral, Elliptisch, AGN)

### Dynamik & Physik

5. Diskrete Orbitalschalen müssen tangentiale Bewegung in der Galaxienebene zeigen.
6. SMBH und Bulge müssen als unterschiedliche, aber gekoppelte Kernkomponenten sichtbar sein.
7. Physikalische Parameter sollen aus der Primzahl-/Massen-Analogie abgeleitet werden.
8. Verschmelzungs- / Merge-Logik muss typkonform und konzepttreu sein.

## Aktueller Stand

### Was bereits vorhanden ist

- `src/elements.rs` generiert derzeit:
  - ein Zentrum als `ParticleSegmentType::Core` für SMBH
  - mehrere `ParticleSegmentType::Bulge`-Körper für den Bulge
  - `ParticleSegmentType::Orbital`-Körper für die Scheibe
  - `ParticleSegmentType::Gas` für Gaswolken im Diskus
  - `ParticleSegmentType::Satellite` für Halo/äußere Körper

- `konzept2.txt` ist als Konzeptquelle vorhanden und dient als Leitlinie.
- `src/body.rs` enthält Segmenttypen, die die Trennung von Kern, Scheibe, Gas und Satelliten erlauben.
- `src/renderer.rs` weist kolorierte Visualisierung nach Segmenttyp auf.
- `src/simulation.rs` hat bereits eine center/core-contact Merge-Logik, die nach sustained contact fragt.
- Ein Test wurde ergänzt, der sicherstellt, dass ein generiertes galaktisches Atom-System einen einzigen SMBH-Kern und mehrere Bulge-Teilchen besitzt.

### Technische Umsetzung im Code

- `ElementDefinition::smbh_mass()` und `bulge_mass_fraction()` definieren bereits ein Verhältnis von Kern- zu Umgebungsmasse.
- `generate_galactic_atom_system()` produziert jetzt eine Bulge-Wolke statt eines einzelnen Bulge-Körpers.
- Tangentiale Orbitale werden in der Diskus- und Gasgenerierung über `tangent`-Vektoren umgesetzt.
- Halokörper erhalten reduzierte Orbitalgeschwindigkeiten und leichte vertikale Streuung.

## Differenzen und Lücken

### Visuelle / konzeptionelle Lücken

- Die aktuelle Bulge-Generierung ist zwar multipel, aber noch nicht klar genug als dichter Kernbereich:
  - Bulge-Teilchen werden rund um das Zentrum zufällig verteilt,
  - die Bulge-Masse ist zu ähnlich zu `Disk`-Körpern und benötigt eine bessere Gewichtung,
  - eine zu geringe Anzahl an Bulge-Teilchen würde zur Instabilität führen – es braucht eine morphologie- und periodenspezifisch skalierte Bulge-Initialisierung,
  - die konkrete Kopplung zwischen SMBH-Zentrum und Bulge-Wolke ist eher massenbasiert als physikalisch modelliert.
- Der Bulge muss als `Partikel-Wolke` lesbar sein, die eher an ausgebrannte Sternkerne, Pulsare und Magnetare erinnert als an reguläre Scheibenpartikel.
- Die Scheibe darf einzelne magnetar-/pulsar-ähnliche Ausreißer enthalten, muss aber insgesamt aus zahlreicheren, leichteren, tangentialen Orbitalkörpern bestehen.

- Die Scheiben-, Gas- und Satellitenkomponenten sind in der aktuellen Simulation getrennt, aber noch nicht als echte Schalen des gleichen Galaxienatoms erkennbar.
- Die Valenzelektronen-Analogie für Satelliten ist noch nicht spezifisch umgesetzt: Satelliten werden als Halokörper erzeugt, aber nicht als äußere, schwach gebundene Strukturen mit besonderem Bindungsverhalten.

### Physikalisch-analoge Lücken

- Der Primzahlanalogie fehlt eine explizite Verbindung zur Orbitaldynamik und zur Orts- / Frequenzeigenschaft der einzelnen Komponenten.
- Die aktuelle massenbasierte `smh_mass`- / `bulge_mass_fraction`-Aufteilung ist noch nicht direkt aus den Periodensystemfamilien ableitbar.
- Die Merge-Logik ist zwar zentralisiert, behandelt aber nicht explizit:
  - Bulge-SMBH Kopplung / Instabilität
  - Disk-/Gas-Bindung bei Kollisionen
  - Satelliten als Valenzelektronen mit bevorzugtem Austausch

### Realitätsorientierte Lücken

- Es fehlt eine explizitere galaktische Scheibenausrichtung, die den Diskus formal als flache Orbitalschicht „mit Tangentialdynamik“ darstellt.
- Gaswolken sollten nicht nur numerisch im Diskus erscheinen, sondern als diffuser, innerer Elektronenpool mit eigener Stabilität.
- SMBH sollte eine deutlich andere visuelle Signatur erhalten als normale Kerne, etwa durch Intensitäts-/Farbunterschied und zentrale Gravitationseigenschaften.

## Roadmap

### Phase 1: Konzept-Evaluation & klare Trennung

1. Dokumentation der aktuellen Implementierung an den Stellen:
   - `src/elements.rs` für Systemgenerierung
   - `src/body.rs` für Segmentdefinition
   - `src/renderer.rs` für Visualisierung
   - `src/simulation.rs` für Merge- und Kontaktlogik
2. Formales Mapping der `konzept2.txt`-Elemente auf Code-Segmente.
3. Ergänzung der Dokumentation durch diese Roadmap als Referenz.

### Phase 2: Kernstruktur validieren

1. SMBH als einzelner, zentraler `Core`-Körper beibehalten.
2. Bulge als echte `Partikel-Wolke` aus einer robusten Anzahl mittlerer bis schwerer `Bulge`-Partikel gestalten. Eine zu geringe Bulge-Teilchenzahl führt zur Instabilität. Die Initialisierung muss daher:
   - die Bulge-Menge stärker morphologie- und periodenspezifisch skalieren,
   - mehr Bulge-Partikel nutzen als vorher, aber dennoch weniger als die Scheibenpartikel,
   - die Masse pro Bulge-Partikel gegenüber Diskus-Körpern erhöht halten,
   - und einzelne magnetar-/pulsarähnliche Ausreißer in der Scheibe ebenfalls zulassen.
3. Den Bulge als gravitative, dynamische Umgebung des SMBH modellieren, nicht nur als separate Masse.

### Phase 2.1: Bulge-Initialisierung orchestrieren

1. Definiere eine Basis-Anzahl von Bulge-Partikeln pro Morphologie (z. B. Dwarf: 6, Spiral: 10, Elliptisch: 14, AGN: 18).
2. Skaliere diese Zahl zusätzlich mit der Periode oder Ordnungszahl, um größere Systeme stabiler zu machen.
3. Verwende `particle_mass_range` als Referenz für die Massenverteilung, aber interpretiere sie für den Bulge so, dass Bulge-Teilchen schwerer sind als Diskus-Teilchen.
4. Setze den Bulge-Radius eng genug, um einen dichten Kernbereich zu erzeugen, aber groß genug, damit die Wolke nicht mit dem SMBH kollidiert.
5. Überwache die Stabilität durch Tests, die sicherstellen, dass Bulge-Cluster nicht auseinanderfliegen oder zu sehr fragmentiert sind.

**Status:** Phase 2.1 ist in Bearbeitung. Die Bulge-Initialisierung wurde morphologiespezifisch skaliert und die Geschwindigkeiten wurden angepasst, um eine dichte, druckgestützte Kernwolke zu erzeugen.

### Phase 3: Orbitalschale & Gas

1. Diskus-Partikel als flache, tangential orbitierende Schicht erzeugen.
2. Gas-Partikel sollten als innere, diffusere Schicht mit geringerer Massendichte erscheinen.
3. Satelliten / Halo-Körper als äußere, lose gebundene Valenzelektronen strukturieren.
4. Eine mehrstufige Schalenstruktur prüfen: 
   - innere Scheibe,
   - äußere Scheibe,
   - Halo/Satellitenschicht.

**Status:** Phase 3 ist weitgehend umgesetzt. Das Diskus-Layout zeigt nun eine klarere flache, tangentiale Orbitalschicht, die Gaswolken sind als diffuses inneres Element abgesetzt, und Halo/Satelliten halten sich als äußere, lose gebundene Valenzelektronenstruktur.

### Phase 4: Physikalische Primzahlanalogie

1. Primzahl-/Massendimensionen stärker in:
   - SMBH-Masse
   - Bulge-Teilungsmuster
   - Disk-Radius und Tangentialgeschwindigkeit
   - Satellitenbindungsenergie
   integrieren.
2. Perioden als 7 Größenklassen nutzen: jede Periode beeinflusst Größe und Bewegungscharakteristik.
3. Die Perioden-Zuordnung im Code dokumentieren und in `informations.md` bzw. Config-Labels sichtbar machen.
4. `particle_mass_range` als globalen Baseline-Parameter verknüpfen, der für alle Elementsysteme wirkt, aber per Element in Bulge-, Disk- und Halo-Massenstrukturen individuell interpretiert wird.

**Status:** Phase 4 ist jetzt aktiv umgesetzt. Die Elementgenerierung nutzt die Primzahlanalogie als dimensionsabhängige Skalierung für Orbitalgeschwindigkeiten, Bulge-Dichte und Halo-Bindung. `particle_mass_range` wirkt als einheitlicher Basiswert und wird in Bulge-, Disk- und Halo-Massenstrukturen unterschiedlich interpretiert.

### Phase 5: Merge-/Verschmelzungslogik ausrichten

1. Sicherstellen, dass nur gleichartige Segmenttypen verschmelzen dürfen.
2. Für `Core`-Partikel eine separate Kontaktlogik mit >5 Frames beibehalten.
3. Für `Bulge`-Cluster und `Orbital`-Schalen spezifische Wechselwirkungsschwellen evaluieren.
4. Satelliten sollten bei Begegnung mit anderem System eher ausgelöst oder abgeschält werden, statt sofort zu verschmelzen.

**Status:** Phase 5 ist umgesetzt. Die Merge-Logik erlaubt jetzt nur noch gleichartigen Segmenttypen mit identischer Ordnungszahl zu verschmelzen. Es gelten segmenttypische Kontaktzeiten:
- Core/SMBH: 6 Frames der Überlappung
- Bulge: 3 Frames
- Orbital/Satellite: 2 Frames
- Gas: 3 Frames

Damit bleibt das Bulge-Cluster stabiler, die Orbitalkörper verschmelzen nicht bei einem einzigen Streifkontakt, und die Kernverschmelzung bleibt zunächst streng anhalteabhängig.

### Phase 6: Visuelle Realität & Astronomische Darstellung

1. Renderer so ausbauen, dass:
   - SMBH-Zentrum eine eigenständige Licht-/Farbsignatur erhält
   - Bulge klar als dichter, zentraler Bereich dargestellt wird
   - Diskus und Gas eine flache Scheibe mit Tangentialmotion zeigen
   - Satelliten als verstreute äußere Körper erkennbar sind
2. Farb- und Größenkodierung entlang der Periodenfamilien abstimmen.
3. Die Darstellung als „astronomisch realistische Galaxie“ prüfen, zugleich als Atom-Analogie lesbar bleiben.

**Status:** Phase 6 ist in Arbeit. Der Renderer nutzt jetzt segmenttypische Galaxienfarben und glühende Hervorhebungen für Core/SMBH, Bulge, Gas und Satelliten, damit die anfängliche visuelle Struktur des Galaxienatoms stärker sichtbar wird.

### Phase 7: Validierung und Review

1. Neue Tests definieren für:
   - `Core` + Bulge-Cluster-Relation
   - tangentiale Orbitalbewegung
   - korrektes Segment-Merge-Verhalten
2. Visuelle Regression prüfen in einem kurzen Demo-Lauf oder Screenshot-Review.
3. Konzeptkorrekturen aus dem Code in das Dokument `konzept2.txt` rückkopplen, falls notwendig.

## Priorisierte ToDos

1. Zentral: Bulge-Cluster muss physikalisch als echte Kernmasse erscheinen, nicht nur als zweiter großer Körper.
2. Tangentiale Scheiben-Orbitale müssen stärker als Diskusprinzip sichtbar werden.
3. Satelliten / Halo-Körper müssen als äußere Valenzelektronen differenziert werden.
4. Primzahlanalogie muss explizit in den Bewegungsparametern und visuellen Klassen landen.
5. Merge-Logik muss das konzeptuelle Verhalten von Kern-, Scheiben- und Satellitenpartikeln respektieren.

## Ergebnis dieser Roadmap

Wenn diese Roadmap abgearbeitet ist, sollte die Simulation:
- visuell die Astronomie respektieren,
- gleichzeitig die Galaxien-Atom-Analogie klar lesbar machen,
- und physikalisch konsistente Eigenschaften aus der Primzahl-/Periodenordnung integrieren.

Die nächste konkrete Implementierungsphase ist die Überarbeitung von `src/elements.rs` und `src/renderer.rs`, begleitet von Tests in `src/elements.rs` und `src/simulation.rs`.
