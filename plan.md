# Plan: Chemiekompass-gestützte Element-Template-Verfeinerung

## Ziel
Die bestehenden 118 Element-Body-Templates sollen systematisch so erweitert werden, dass sie:

- eine explizite Farblogik nach einem chemischen Elektronegativitäts-Farbkreis tragen,
- chemische Reaktivität und Bindungstendenz als regelbasiertes Template-Merkmal bereitstellen,
- Inkonsistenzen in der bisher verwendeten Elementdefinition beheben,
- die Center/Core-Body-Erzeugung im Galaxy-Pair-Elementmodus separat und konsistent behandeln.

## Problemstellung
Der aktuelle Elementmodus ist bislang eher eine analoge Regelstruktur als ein vollständig kohärentes chemisch motiviertes System. Insbesondere fehlen:

- eine dedizierte Elektronegativitäts-Farbklassifikation,
- einheitliche Reaktivitätswerte und Bindungstendenzen,
- ein Core-Template-Mechanismus, der nicht einfach das zuerst ausgewählte Element verwendet.

## Vorgehensweise

1. Audit der aktuellen Implementierung
   - Review `src/elements.rs` zur aktuellen Element-Template-Struktur.
   - Review `src/simulation.rs` zur Galaxy-Pair-Initialisierung und Elementzuordnung.
   - Review `chemiekompass.md` als fachliche Vorgabe für Farben und Reaktivität.

2. Erweiterung der Element-Template-Struktur
   - `ElementDefinition` um neue Felder ergänzen: Elektronegativität, Chemiegruppe, Farbwert, Reaktivitätsrating, Bindungstendenz.
   - Farbgruppen nach Chemiekompass einführen: Alkalimetalle, Erdalkalimetalle, Übergangsmetalle, Halbmetalle, Nichtmetalle, Halogene, Edelgase.
   - Standardisierte Klassifikationen für jede Elementgruppe und jede Periode bereitstellen.

3. Helper-Funktionen in `src/elements.rs`
   - `color()` / `element_color()`
   - `electronegativity_category()`
   - `reactivity_score()` / `element_reactivity_score()`
   - `binding_tendency(other: &ElementDefinition)`
   - optional: `is_electronegative_pair(other)` oder `complementary_color(other)`

4. Konsistenzkorrekturen der 118 Templates
   - Existierende Inkonsistenzen analysieren und korrigieren.
   - Grenzwerte für Extreme wie H, He, F, O, Cs, Fr festlegen.
   - Sicherstellen, dass jedes Element eine eindeutige Farbe / Reaktivität besitzt.
   - Linearität und Periodenabhängigkeit entlang der Gruppe berücksichtigen.

5. Dedizierte Center-Core-Erzeugung
   - Separate Auswahl der Core-Elemente im Galaxy-Pair-Modus statt Default-Element.
   - Core-Initialisierung in `src/simulation.rs` mit eigenständigen `Body::new_element(...)`-Parametern.
   - Core-Körper als eigene Kategorie behandeln, mit gesonderter Reaktivitäts-/Farbregel.

6. Integration in Simulation
   - Anwendung der Farb- und Reaktivitätswerte während der Elementgenerierung.
   - Falls möglich: Nutzung der neuen Reaktivitätswerte zur Steuerung von Bindungs-/Merge-Regeln.
   - Sicherstellen, dass sowohl initiale Galaxien als auch Spawn-/Akzretionskörper die neuen Regeln nutzen.

7. Dokumentation
   - `chemiekompass.md` um eine technische Spezifikation erweitern.
   - `informations.md` um neue Konfigurationshinweise und Erklärungen ergänzen.
   - Aspekte: Farbkreis, Elektronegativität, Reaktivitätsgruppen, Core-Verhalten.

8. Tests
   - Tests für Farbzuordnung je Element und Elektronegativitätsgruppe.
   - Tests für Reaktivitätsrating und Bindungstendenz.
   - Tests für dedizierte Center-Core-Auswahl im Elementmodus.
   - Konsistenztests für alle 118 Element-Definitionen.

## Betroffene Dateien

- `src/elements.rs`
- `src/simulation.rs`
- `src/body.rs`
- `informations.md`
- `chemiekompass.md`
- `src/config.rs` (nur bei zusätzlicher Steuerung oder Dokumentation erforderlich)
- `src/main.rs` (nur wenn neue Config-Hashfelder zur Laufzeitüberwachung nötig sind)

## Umsetzungsstatus

- [x] `audit_chemistry_color_reactivity`
- [x] `implement_element_color_reactivity`
- [x] `implement_center_particle_template_logic`
- [x] `integrate_element_reactivity_in_generation`
- [x] `document_chemistry_compass_concept`
- [x] `validate_element_template_consistency`
- [x] Maussteuerung fuer Orbit, Zoom, Linksklick-Spawn und Element-Rechtsdrag wiederhergestellt

## Wissenschaftliche Validierungs-Ziellinie

- [x] Analytische Unit-Tests fuer Plummer-Gravitation, Inversquadratgesetz und Integrator
- [x] Impuls-, Drehimpuls-, Energie-, Schwerpunkt- und Massenerhaltung
- [x] Exakte Direktvergleichs- und Barnes-Hut-Fehlerschranken
- [x] Adaptive Produktionskraft und `Simulation::step()` direkt validiert
- [x] Disk-Equilibrium an dasselbe Softening wie die Laufzeitkraft gekoppelt
- [x] Aktive CPU-Gasdarstellung getestet; optionales GLSL-Artefakt kompiliert
- [x] Deterministische Struktur- und optionale Hardware-Performance-Gates
- [x] Python-Orchestrator und Windows-CI aktiviert
