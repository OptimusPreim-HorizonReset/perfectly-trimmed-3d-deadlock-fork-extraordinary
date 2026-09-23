# GPDM-Implementierungshandoff

## Zweck und verbindliche Architekturentscheidung

Dieses Dokument ist der Implementierungshandoff fuer das **Galactic
Parliamentary Dynamics Model (GPDM)**. Es kombiniert die Konzepte aus
[`smbh_feedback.txt`](./smbh_feedback.txt), [`tisch_model.txt`](./tisch_model.txt),
[`galactic_structure_levels.txt`](./galactic_structure_levels.txt),
[`gpdm_spec.txt`](./gpdm_spec.txt) und
[`architecture_evolution.txt`](./architecture_evolution.txt) zu einem vollstaendigen
Implementierungsplan.

Das Ergebnis ist ein neues, eigenstaendiges Rust-Modul unter `src/gpdm/`. Es
verwaltet Flusskonten, Tische, Institutionen, Delegierte, Mehr-Uhren-Taktung,
Kontaktparlamente und Historie selbst. Der vorhandene N-Body-Code bleibt die
autoritative Quelle fuer Position, Geschwindigkeit, Masse, Kollisionen und
Gravitation.

**Nicht verhandelbare Integrationsgrenze**

1. Das GPDM besitzt keinen `Body` und erweitert `Body` nicht um GPDM-Felder.
   Der Zustand liegt ausschliesslich in `GpdmRuntime` und ist ueber die stabile
   `Body.id` adressiert.
2. Das GPDM liest den Host nur ueber einen kleinen Snapshot-/Event-Adapter und
   gibt nur validierte, endliche `HostEffects` zurueck.
3. Bestehende Gravitation verbleibt im vorhandenen `Octree`; das GPDM baut
   keinen zweiten Teilchen-Gravitationssolver. Sein Repräsentantenbaum
   beschleunigt ausschliesslich die Kommunikation zwischen Tabellen.
4. Der alte Simulationspfad muss bei `enable_gpdm: false` exakt unveraendert
   bleiben. Der deaktivierte Pfad darf weder Zufallszahlen ziehen noch
   Teilchen, Konfiguration oder Renderer-Globals anfassen.
5. Bei `enable_disk_equilibrium_mode: true` ist GPDM standardmaessig ein
   beobachtendes und bilanzierendes Subsystem. Physische Rueckwirkungen sind
   gesperrt, bis `gpdm_allow_equilibrium_backreaction: true` explizit gesetzt
   wurde und die zugehoerigen Akzeptanztests bestehen.

Damit ist das Modul vollstaendig eigenstaendig, aber sauber an den vorhandenen
Host angeschlossen.

## Ausgangslage im aktuellen Code

| Vorhandene Komponente | Relevanter Ist-Zustand | Verbindlicher GPDM-Anschluss |
|---|---|---|
| [`src/body.rs`](./src/body.rs) | `Body.id` ist stabil; Position, Geschwindigkeit, Masse, Drehachse, Gaswerte und `ParticleSegmentType` sind vorhanden. | Nur lesen; `Body.id` wird `ParticipantId`. Keine GPDM-Felder in `Body` einbauen. |
| [`src/simulation.rs`](./src/simulation.rs) | `Simulation::step` orchestriert Integration, Kollision, Gravitation und Gas; Merges und Gaszuendung mutieren die Body-Liste. | Ein `GpdmRuntime`-Feld, ein Event-Puffer und zwei schmale Hooks; siehe Abschnitt "Host-Adapter". |
| [`src/quadtree.rs`](./src/quadtree.rs) | Octree aggregiert Masse, Schwerpunkt und Quadrupol fuer die Gravitation. | Unveraendert als Gravitationsbaum lassen. GPDM verwendet eigene Tabellen-Delegierte und keine `Node`-Erweiterung. |
| [`src/gas.rs`](./src/gas.rs) | SPH-inspirierte Dichte, Druck, Abkuehlung, Kondensation und Merge-Kandidaten existieren. | Gas-Snapshots liefern die Eingaben fuer Akkretion, Heizung, Wind und institutionelle Gasfluesse. |
| [`src/galaxy_templates.rs`](./src/galaxy_templates.rs) | Equilibrium-Erzeugung nutzt sech²-Hoehenprofil, weichgemachte Kreisgeschwindigkeit und asymmetrischen Drift. | GPDM muss diese Anfangsdynamik respektieren und im Equilibrium-Modus keine ungeprueften Impulse anwenden. |
| [`src/config.rs`](./src/config.rs) | Konfiguration, Parser, Defaults, Sanitisierung und effektive Equilibrium-Werte befinden sich hier. | Neue GPDM-Felder vollstaendig in Struct, Default, Parser, Sanitisierung und Tests verdrahten. |
| [`src/execution.rs`](./src/execution.rs) und [`src/scientific_validation.rs`](./src/scientific_validation.rs) | Vorhandene wissenschaftliche Auswertung und Invarianztests. | GPDM-spezifische Bilanz-, Determinismus-, Equilibrium- und Skalierungstests dort ergaenzen. |
| [`src/main.rs`](./src/main.rs) und [`src/renderer.rs`](./src/renderer.rs) | Hot reload basiert auf Konfigurationshash; Renderer zeigt Einstellungen und Overlays. | Alle GPDM-Konfigurationswerte hashen und eine diagnostische GPDM-Anzeige bereitstellen. |

## Konzept-zu-Implementierungsmatrix

| Konzeptquelle | Verbindliche Umsetzung |
|---|---|
| `gpdm_spec.txt` | Konten fuer Masse, Energie, Impuls, Drehimpuls, Ladung, Information und Entropie; Flusskapazitaeten; Verfassung; Delegierte; asynchrone Tische; Kontaktparlament. |
| `tisch_model.txt` | Teilnehmerrollen, lokale/regional/galaktische Tische, harte Erhaltungsgesetze, Institutionen und hierarchische Repräsentanten. |
| `galactic_structure_levels.txt` | Sieben Schichten: Teilnehmer, Konten, Institutionen, Verfassung, Delegierte, Evolution und Kompression. |
| `smbh_feedback.txt` | Akkretion ist begrenzt; Energie wird in thermisches und kinetisches Feedback umgewandelt; Winds sind radial, Jets folgen der SMBH-Spinnachse; nur ein Teil der einstroemenden Masse wird akkrediert. |
| `architecture_evolution.txt` | Stabilitaetsfunktion, institutionelle Fitness, Gedächtnis, offene Hierarchie und Kompressionsgrad sind beobachtbare GPDM-Metriken, keine neuen Naturgesetze. |

Die Begriffe "Information", "Entropie" und "Kompression" erhalten in Version 1
bewusst **diagnostische**, nicht gravitative Bedeutung. Der aktuelle Host hat
keine physikalisch kalibrierte Informationsthermodynamik oder Ladungsdynamik;
das GPDM darf daraus keine Kraft, Masse oder Energie erfinden.

## Zielstruktur des neuen Moduls

```text
src/gpdm/
  mod.rs              # Oeffentliche Fassade, GpdmRuntime und Lifecycle
  ids.rs              # Typisierte IDs und deterministische Reihenfolgen
  host.rs             # Snapshot/Event/Effect-Vertrag zum bestehenden Host
  accounts.rs         # ConservationAccounts und atomare Buchungen
  flow.rs             # FlowProposal, Limits, Commit und abgelehnte Buchungen
  constitution.rs     # Invarianten, Toleranzen und Commit-Pruefungen
  participant.rs      # Rollen, Momentaufnahme und Rollenwahrscheinlichkeiten
  table.rs            # TableKind, TableState, Mitgliedschaft und Bilanzen
  institution.rs      # SMBH, Halo, Gas, Stern, Spiralarm und Balken
  delegate.rs         # Verdichtete Repräsentanten und Repräsentantenbaum
  scheduler.rs        # Deterministische Mehr-Uhren-Taktung
  contact.rs          # Kontaktparlamente und Kollisionsphasen
  evolution.rs        # Stabilitaet, Fitness, Geschichte und Kompression
  diagnostics.rs      # Snapshots, Kennzahlen, Fehler und Renderer-Daten
  tests.rs            # Moduluebergreifende Unit- und Property-Tests
```

Es werden keine neuen externen Dependencies benoetigt. Fuer Vektoren wird die
bereits vorhandene `ultraviolet::Vec3`-Version verwendet. Karten, Sets und
Reihenfolgen mit Auswirkung auf Simulationsergebnisse verwenden `BTreeMap` und
`BTreeSet`; `HashMap` ist nur fuer reine, nicht beobachtbare Caches erlaubt.

## Datenmodell und fachliche Regeln

### 1. IDs, Teilnehmer und Konten

```rust
ParticipantId = Body.id
TableId       = monoton erzeugte, nie wiederverwendete ID
InstitutionId = monoton erzeugte, nie wiederverwendete ID
ContactId     = kanonische Paar-ID zweier TableId-Werte plus Epoche
```

`ParticipantState` enthält ausschliesslich einen abgeleiteten Snapshot:

- Identitaet, Segmenttyp und optionale Elementmetadaten,
- `position`, `velocity`, `mass`, `rotation_axis`, `angular_speed`,
- `gas_density`, `gas_pressure`, `gas_temperature` und Glättungsradius,
- Mitgliedschaft in Lokal-, Regions-, Galaxie- und Kontakt-Tischen,
- Rollenverteilung: `Accretor`, `Transporter`, `Heater`, `Compressor`,
  `Wind`, `Jet`, `Escaper`, `Reservoir`.

Die Rollen sind Gewichte in `[0, 1]`, ihre Summe ist pro Teilnehmer innerhalb
von `gpdm_role_sum_tolerance` eins. Sie sind keine zweite Bewegungsintegration.
Ihre Berechnung muss deterministisch und ohne Zufallszahl erfolgen:

- hohe Dichte + geringe spezifische Drehimpulsmenge -> hohes
  `Accretor`-Gewicht,
- hohe Dichte + thermische Energie -> hohes `Heater`-Gewicht,
- positiver Feedbackdruck oberhalb lokaler Bindung -> `Wind`,
- hohe Magnetisierungs-Proxymetrik und Core-Ausrichtung -> `Jet`,
- schwache Bindungsmetrik + positive radiale Geschwindigkeit -> `Escaper`.

`ConservationAccounts` verwendet `f64` fuer Bilanzgenauigkeit:

```text
mass, energy, linear_momentum: Vec3, angular_momentum: Vec3,
charge, information, entropy
```

Quellwerte werden aus dem Host abgeleitet:

- Masse ist `Body.mass`.
- Linearimpuls ist `mass * velocity`.
- Drehimpuls ist `position x linear_momentum` um den Schwerpunkt des
  jeweiligen Tisches.
- Kinetische Energie ist `0.5 * mass * |velocity|²`.
- Gasthermie ist eine klar dokumentierte, konfigurierte
  `temperature_to_energy_scale`-Abbildung; ohne aktivierte Skala bleibt sie
  ausschliesslich diagnostisch.
- Ladung startet bei `0.0`, bis der Host eine Ladung anbietet.
- Information, Entropie und Kompression sind dimensionslose diagnostische
  Salden; sie werden nie in die physische Energieerhaltung eingerechnet.

### 2. Verfassung und atomare Flussbuchung

`Constitution` ist fuer jeden Tisch vorhanden und definiert:

- relative und absolute Toleranz fuer Masse, Energie, Impuls und Drehimpuls,
- Nichtnegativitaet von Masse und physischer Energie,
- maximale Abgabe pro Tick (`gpdm_max_flow_fraction`),
- erlaubte Institutionen und Koppelrichtungen,
- ob der Tisch nur beobachtet oder Effekte an den Host abgeben darf.

Ein `FlowProposal` benennt Quelle, Senke, Konto, Menge, Ursache,
Institution, Host-Effekt und Zeitstempel. `FlowTransaction::commit` muss in
dieser Reihenfolge arbeiten:

1. IDs, Endlichkeit, Vorzeichen und Kapazitaet pruefen.
2. Den kompletten Batch gegen eine Kopie aller betroffenen Konten vorpruefen.
3. Fuer physische Konten Quelle und Senke paarweise und exakt mit
   entgegengesetztem Vorzeichen buchen.
4. Summen vor/nachher mit der Verfassung pruefen.
5. Erst dann Konten, Historie und optionalen `HostEffect` gemeinsam
   veroeffentlichen.

Eine unzulaessige Buchung wird **nicht** still verworfen: Sie erzeugt ein
`DiagnosticEvent::RejectedTransaction` mit Regel, IDs und Betragsgrenze. Ein
Teilbatch darf nie implizit erfolgreich sein.

### 3. Tische und Ebenen

`TableKind` umfasst mindestens:

```text
Local, Region, Galaxy, Cluster, Supercluster, Universe, Contact
```

Implementierungsreihenfolge:

1. Bei Initialisierung pro Body ein `ParticipantState`, pro raeumlicher
   Nachbarschaft ein `Local`-Tisch und pro Galaxy-Range ein `Galaxy`-Tisch.
2. `Region` gruppiert lokale Tische deterministisch nach Zelle und
   Stabilitaet. `Cluster` und `Supercluster` entstehen aus Delegierten,
   nicht aus direkten Body-Scans.
3. `Universe` ist genau ein read-only Wurzeltisch. Er addiert alle
   Delegierten und ist ein globaler Bilanzanker.
4. `Contact` wird zeitlich begrenzt nur zwischen zwei Galaxie-Tischen
   erzeugt. Er besitzt eigene Konten, Historie und eine abweichende
   Zeituhr.

Die initiale Galaxiezuordnung kommt nicht aus Body-Indizes im GPDM. Der Host
uebergibt sie als `GalaxyMembership { participant_id, galaxy_id }`, die
`Simulation` aus ihren bereits gepflegten Pair-Ranges erzeugt. Nach einem
physikalischen Host-Merge ist der Eventstrom autoritativ; niemals versuchen,
die Zugehoerigkeit aus verschobenen Vec-Indizes zu erraten.

### 4. Institutionen

Gemeinsames Trait-Verhalten:

```text
observe(snapshot, table) -> InstitutionObservation
propose_flows(observation, accounts, constitution) -> Vec<FlowProposal>
fitness(history) -> f64
delegate() -> InstitutionDelegate
```

| Institution | Eingabe | Erlaubte Ausgaben |
|---|---|---|
| `SmbhInstitution` | zentraler Core, nahes Gas, Drehachse, Akkretions-/Bindungsmetriken | begrenzte Akkretion, thermische Rueckkopplung, radialer Wind, achsenparalleler Jet |
| `GasInstitution` | Dichte, Druck, Temperatur, Geschwindigkeitsdispersion | Druck-/Kuehlungsdiagnostik, Transport, Akkretionsangebot |
| `StellarInstitution` | Stellar-/Orbital-Teilnehmer und lokale Bindung | Energie- und Metall-/Informationsdiagnostik; keine neue Sternphysik ohne Host-Ereignis |
| `HaloInstitution` | Escaper, Radius, Bindungsmetrik | Reservoir, Rueckakkretionsangebot und langfristige Historie |
| `SpiralArmInstitution` | phasenstabile azimutale Ueberdichte und Drehimpulsgradient | radial gerichtete Drehimpuls-Transportangebote |
| `BarInstitution` | lang anhaltende quadrupolare Asymmetrie | Massentransport-Angebote zum Zentrum |

Spiralarm und Balken werden nicht in Version 1 erraten. Der Detector benoetigt
ein gleitendes Fenster, eine Mindestdauer, Mindestmitgliederzahl sowie
Hysterese fuer Gruendung und Aufloesung. Bis diese Kriterien erfuellt sind,
werden nur Kandidaten mit Konfidenz diagnostiziert.

### 5. SMBH Feeding und Feedback

Die SMBH-Institution setzt `smbh_feedback.txt` konkret um:

1. Sie ermittelt ein Gasangebot nur innerhalb des konfigurierten
   Akkretionsradius und begrenzt die Rate durch Akkretionskapazitaet,
   Zeitintervall und verfuegbare Gasmasse.
2. `accepted_mass` wird atomar vom Gas- zum SMBH-Massekonto gebucht.
3. `feedback_energy = efficiency * accepted_mass * feedback_energy_scale`
   wird nur aus einem expliziten Akkretionsenergie-Reservoir auf thermisches
   Gasfeedback, Wind und Jet aufgeteilt. Die Summe aller Ausgaben darf dieses
   Reservoir nie uebersteigen. Im Beobachtungsmodus ist dieses Reservoir eine
   reine Diagnosegroesse und erzeugt keinen Hosteffekt.
4. Windimpulse sind radial vom SMBH weg; Jetimpulse sind entlang
   `rotation_axis` mit entgegengesetzten Polen paarweise ausgeglichen.
   Dadurch bleibt der Nettolinearimpuls des isolierten Feedbackereignisses
   null.
5. Ein Feedbackeffekt ist eine begrenzte, deterministisch sortierte Liste
   betroffener Gas-IDs. Die Anwendung laeuft nur im expliziten
   Rueckwirkungsmodus.

Es gibt keine hartcodierte Lichtgeschwindigkeit oder SI-Energie in einer
dimensionslosen Hostsimulation. Alle Umrechnungen laufen ueber klar
benannte, standardmaessig konservative Skalierungsparameter und werden in
der Diagnose mit ausgegeben.

Bevor Phase 4 SMBH-Feedback physisch anwenden darf, muss der Hostadapter einen
`HostEnergyReservation` liefern. Er bestimmt die bei der Akkretion
aufgeloeste, weichgemachte Bindungsenergie vor und nach dem Massentransfer
mit derselben `effective_epsilon`-Konvention wie die Scheibenerzeugung. Nur
eine positive, innerhalb der Energieverfassung geschlossene Reservation darf
in thermische oder kinetische Hosteffekte umgewandelt werden. Kann diese
Reservation nicht geschlossen werden, bleibt der Vorschlag beobachtend und
die Runtime protokolliert die Ablehnung; sie darf Gas niemals aus einer
unbedeckten Energiequelle aufheizen.

### 6. Delegierte, O(n log n) und Kompression

`TableDelegate` exportiert nur aggregierte Werte:

```text
table_id, kind, center_of_mass, velocity, mass, energy,
linear_momentum, angular_momentum, quadrupole, radius,
halo_extent, feedback_activity, stability, fitness,
compression_ratio, active_institution_counts
```

`RepresentativeTree` wird aus Delegierten aufgebaut und hat eigene Nodes mit
einer kanonisch sortierten ID-Liste. Fernkopplung und Clusterbildung traversieren
diesen Baum mit einem Oeffnungskriterium auf Tabellenradius/Entfernung. Nahe
Knoten werden aufgeloest, ferne durch ihren Delegierten vertreten. Der Baum
darf kein `Body` klonen und keine Teilchen-Gravitation berechnen.

`compression_ratio` muss operational definiert sein:

```text
K = source_member_count / max(delegate_parameter_count, 1)
```

Er wird als Messwert gespeichert; er entscheidet in Version 1 nicht ueber die
physische Lebensdauer eines Tisches.

### 7. Mehr-Uhren-Dynamik, Evolution und Geschichte

`MultiClockScheduler` arbeitet framebasiert und deterministisch:

- Jede Tabelle besitzt `period_frames >= 1`, `next_due_frame` und einen
  letzten erfolgreichen Commit.
- Es werden nur due Tables verarbeitet; die Reihenfolge ist
  `(next_due_frame, TableId)`.
- Untere Ebenen duerfen haeufiger ticken als ihre Eltern. Ein Kind exportiert
  nur seinen letzten gueltigen Delegierten, solange sein Eltern-Tick noch
  nicht faellig ist.
- Keine Nebenlaeufigkeit im GPDM-Tick, solange ein Commit Hosteffekte
  erzeugen kann. Das verhindert reihenfolgeabhaengige Salden.

`EvolutionState` enthaelt:

- History-Ringpuffer mit Gruendung, Merge, Shock, Akkretion, Feedback,
  Rollenwechsel und abgelehnten Buchungen,
- Stabilitaet aus Bilanzresidual, Mitgliedschaftsdauer, Flussvarianz und
  Delegiertenstabilitaet,
- institutionelle Fitness aus erfolgreich abgewickeltem, erhaltendem Fluss
  pro Tick und ohne Bilanzverletzung,
- Kandidaten zur Gruendung hoeherer Ebenen, die erst nach konfigurierter
  Dauer und Hysterese materialisiert werden.

Historie darf verdichtet werden, aber nie die Summen oder die letzten
`gpdm_history_retention_events` Ereignisse verlieren. Jede Verdichtung
schreibt einen `HistoryCompaction`-Eintrag mit vorheriger/nachheriger
Ereigniszahl und K.

### 8. Kontaktparlamente

Ein Kontakt entsteht bei zwei Galaxie-Delegierten, wenn der Abstand kleiner
als die konfigurierte Summe ihrer Halo-/Kontaktradien ist. `ContactParliament`
folgt zwingend diesen Phasen:

```text
Approaching -> Overlapping -> Negotiating -> Settling -> Dissolved | Merged
```

- `Approaching` und `Overlapping` sind rein beobachtend.
- Erst `Negotiating` kann konservative Flussangebote fuer Gas, Energie und
  Drehimpuls erzeugen.
- Ein GPDM-Kontakt darf keine Hostverschmelzung erzwingen.
- Wenn der Host zwei Bodies/Galaxiezentren tatsaechlich verschmilzt, liefert
  er `HostEvent::Merged`. Das Parlament schliesst dann mit einer finalen
  Bilanz, ueberfuehrt seine Historie in den neuen Galaxie-Tisch und beendet
  seine Mitgliedschaft.
- Ohne Host-Merge loest sich ein Kontakt nach Distanz- und Hysteresegrenze
  auf. Reservoire und Historie bleiben in den Ursprungstischen.

## Schmaler Host-Adapter

`src/gpdm/host.rs` ist die einzige Stelle, die die Semantik des vorhandenen
Simulators kennt. Seine DTOs enthalten primitive Werte und GPDM-IDs, aber
keine Referenz auf `Body`:

```rust
pub struct HostSnapshot { frame: usize, dt: f32, bodies: Vec<HostBody>, galaxies: Vec<GalaxyMembership> }
pub enum HostEvent { Spawned, Merged, GasIgnited, Removed, ConfigReloaded }
pub struct HostEffects { velocity_impulses: Vec<VelocityImpulse>, gas_heat: Vec<GasHeat>, mass_transfers: Vec<MassTransfer> }
```

### Minimaler Host-Diff

1. In [`src/main.rs`](./src/main.rs) `mod gpdm;` deklarieren.
2. In [`src/simulation.rs`](./src/simulation.rs) ergaenzen:
   - `gpdm: gpdm::GpdmRuntime`,
   - `gpdm_events: Vec<gpdm::HostEvent>`,
   - bei `Simulation::new` die Runtime nach der vollstaendigen Body-Erzeugung
     mit dem ersten Snapshot erzeugen,
   - eine private Methode `record_gpdm_event`,
   - eine private Methode `gpdm_snapshot`,
   - eine private Methode `apply_gpdm_effects`.
3. Jede mutierende Hostoperation schreibt ein Ereignis mit **IDs vor der
   Vec-Mutation**:
   - `insert_body_at` -> `Spawned`,
   - `merge_particle_bodies` und `merge_center_particles` -> `Merged` mit
     Gewinner-/Verlierer-ID, gebuchten Hostmassen und Merge-Art,
   - Gaszuendung -> `GasIgnited` mit vorheriger Gas- und nachfolgender
     Stellar-Rolle,
   - Entfernung/Pruning -> `Removed`.
4. Die Runtime wird am Ende von `Simulation::step`, nach `update_gas` und
   allen Body-Listenmutationen, mit Snapshot und drainiertem Eventpuffer
   getickt. So sieht sie den autoritativen Endzustand eines Frames.
5. Rueckwirkungsfaehige Effekte werden nur als `pending_gpdm_effects`
   gespeichert und am **Anfang des naechsten** `step` vor `iterate` in
   kanonischer ID-Reihenfolge angewendet. Das vermeidet eine zweite,
   versteckte Integrationsphase und doppelte Impulse im selben Frame.
6. `apply_gpdm_effects` prueft IDs, Endlichkeit, maximale Impuls-/Temperatur-
   und Massenrate ein zweites Mal. Ungueltige Effekte werden protokolliert
   und als abgelehnter Hosteffekt an die Runtime zurueckgemeldet.

`HostEffects` werden nicht direkt aus Rayon-Closures angewendet. Das GPDM
arbeitet auf einem unveraenderlichen Snapshot und der Host mutiert Bodies erst
nach Abschluss des GPDM-Commits.

## Konfiguration und Equilibrium-Kompatibilitaet

### Neue Konfigurationsfelder

In [`src/config.rs`](./src/config.rs) werden mindestens diese Felder
eingefuehrt:

```text
enable_gpdm: bool = false
gpdm_tick_interval: usize = 1
gpdm_local_cell_size_factor: f32 = 1.0
gpdm_region_min_tables: usize = 4
gpdm_max_flow_fraction: f32 = 0.02
gpdm_mass_tolerance: f64 = 1e-8
gpdm_energy_tolerance: f64 = 1e-6
gpdm_momentum_tolerance: f64 = 1e-6
gpdm_role_sum_tolerance: f32 = 1e-5
gpdm_enable_institution_detection: bool = true
gpdm_institution_window_frames: usize = 32
gpdm_institution_create_threshold: f32 = 0.80
gpdm_institution_destroy_threshold: f32 = 0.55
gpdm_contact_radius_factor: f32 = 1.0
gpdm_contact_min_frames: usize = 8
gpdm_history_retention_events: usize = 512
gpdm_smbh_accretion_radius_factor: f32 = 2.0
gpdm_smbh_max_accretion_fraction: f32 = 0.01
gpdm_feedback_efficiency: f32 = 0.05
gpdm_feedback_energy_scale: f32 = 1.0
gpdm_wind_fraction: f32 = 0.70
gpdm_jet_fraction: f32 = 0.20
gpdm_enable_backreaction: bool = false
gpdm_allow_equilibrium_backreaction: bool = false
```

`gpdm_wind_fraction + gpdm_jet_fraction` wird auf `[0, 1]` begrenzt; der
Rest ist thermische Rueckkopplung. Alle Radien, Zeitfenster und Toleranzen
muessen endlich und positiv sein. Ungueltige Werte werden analog zu den
existierenden Konfigurationsfeldern saniert und mit einer klaren
`eprintln!`-Meldung begruendet.

### Equilibrium-Vertrag

Der bestehende Equilibrium-Modus hat bereits harte, relevante Garantien:
weiches Plummer-Softening, eigene Theta-Wahl, sech²-Vertikalprofil,
asymmetrischen Drift, feste Force-/Collision-Intervalle und deaktivierte
High-Count-Abkuerzungen. GPDM ergaenzt diese Garantien wie folgt:

1. `enable_disk_equilibrium_mode && enable_gpdm` erzeugt vollstaendige
   Teilnehmer, Tische, Delegierte, Historie und Diagnosen.
2. Ohne beide ausdruecklichen Flags
   `gpdm_enable_backreaction` **und**
   `gpdm_allow_equilibrium_backreaction` muss `HostEffects` leer sein.
3. Ist eines der beiden Flags gesetzt, aber nicht beide, wird GPDM im
   Beobachtungsmodus fortgesetzt und eine Diagnose geschrieben; es gibt kein
   stilles physisches Fallback.
4. Bei erlaubter Rueckwirkung gelten engere Grenzen:
   `flow_fraction <= gpdm_max_flow_fraction`, null Nettoimpuls pro
   SMBH-Feedbackbatch, keine direkte Aenderung von
   `disk_equilibrium_*`-Parametern und erneute Messung von Scheibendrift,
   Energie- und Drehimpulsfehlern.
5. Jeder Config-Reload, der ein GPDM-Feld aendert, muss in
   [`src/main.rs`](./src/main.rs) in `config_hash` aufgenommen werden,
   damit die Runtime keine Konfiguration alter und neuer Physik vermischt.

Die neuen Schluessel samt Erklaerung und einem sicheren Beispielblock werden
in [`informations.md`](./informations.md) dokumentiert. Der bestehende
Disk-Equilibrium-Block bleibt unveraendert und erhaelt nur einen Hinweis auf
den GPDM-Beobachtungsmodus.

## Implementierungsphasen und Abnahmekriterien

### Phase 0 - Fundament und unveraenderter Basispfad

1. Modulbaum, typisierte IDs, `HostSnapshot`, `HostEvent`, `HostEffects` und
   `GpdmRuntime::disabled()` anlegen.
2. Neue Konfiguration vollstaendig verdrahten: Struct, Default, Parser,
   Sanitisierung, `config_hash`, Konfigurationstests und Dokumentation.
3. `Simulation` nur um optionalen Runtime-/Eventpuffer und no-op Hooks
   erweitern.
4. Test: Bei deaktiviertem GPDM gibt es keine GPDM-Events, keine Effekte und
   keine Aenderung am existierenden Validation-Finish-Line-Ergebnis.

**Exit:** `cargo test` und die vorhandenen Disk-Equilibrium-Tests bestehen;
der neue Code verursacht bei deaktivierter Option weder Allokationen im
Frame-Pfad noch neue Zufallsaufrufe.

### Phase 1 - Teilnehmer, Konten und Verfassung

1. Teilnehmer aus dem Snapshot erzeugen und anhand von
   `ParticleSegmentType` rollenbasiert einordnen.
2. Kontenableitung, Rollenverteilung und atomaren
   `FlowTransaction::commit` implementieren.
3. Lokal-/Galaxie-/Universum-Tische mit rekursiver Aggregation bauen.
4. Diagnostik fuer Bilanzrest, abgelehnte Buchung und Rollensumme ausgeben.

**Exit:** Fuer einen handgebauten Snapshot stimmen Teilnehmer-,
Galaxie- und Universumssummen innerhalb der jeweiligen Toleranz; jeder
fehlerhafte oder ueberkapazitaere Flow wird vollstaendig abgelehnt.

### Phase 2 - Delegierte, Mehr-Uhren und Kontaktparlamente

1. Repräsentantenbaum und Oeffnungskriterium implementieren.
2. Deterministische Tabellenzeituhr implementieren.
3. Kontaktzustandsautomat, kanonische Paar-IDs und Abschlussbilanzen
   implementieren.
4. GalaxyMembership-Adapter aus `Simulation` implementieren und Host-Merge-
   Events bis in die Kontaktgeschichte durchreichen.

**Exit:** Zwei Galaxien erzeugen exakt einen Kontakt pro kanonischem Paar,
haben reproduzierbare Phasenwechsel und behalten bei Aufloesung oder Merge
eine bilanzierte Historie.

### Phase 3 - Institutionen und SMBH Feedback

1. SMBH-, Gas-, Stellar- und Halo-Institutionen zuerst als beobachtende
   Angebote implementieren.
2. Akkretionsbudget, Feedbackreservoir, Wind-/Jet-Geometrie und
   konservative Flowbuchungen implementieren.
3. InstitutionDetector mit Fenster und Hysterese fuer Spiralarm und Balken
   implementieren; keine Rueckwirkung bis alle Detektortests bestehen.
4. Historie, Stabilitaet, Fitness und Kompressionskennzahl erzeugen.

**Exit:** Eine Akkretionsscheibe produziert begrenzte Akkretions-, Heiz-,
Wind- und Jetangebote, ohne Masse, Energie, Impuls oder Drehimpuls im
GPDM-Ledger zu verlieren. Jets liefern paarweise keinen Nettoimpuls.

### Phase 4 - Gesteuerte Rueckwirkung

1. `HostEffects`-Preflight und verzogerte Anwendung implementieren.
2. Zunaechst nur Gasheizung und paarweise ausgeglichene Velocity-Impulse
   freischalten; Massenuebertragungen bleiben bis zu eigenen
   Massen-/Kollisionsregressionstests deaktiviert.
3. Danach Massenuebertragungen mit Hostevent-Quittierung aktivieren.
4. Im Equilibrium-Modus bleibt dies durch den Doppel-Opt-in gesperrt.

**Exit:** Jeder physisch angewandte Effekt hat einen erfolgreichen Ledger-
Commit, eine Host-Quittierung und bleibt innerhalb der konfigurierten Bounds.
Ein bewusst ungueltiger Effekt aendert keinen Body.

### Phase 5 - Evolution, UI und Betriebsreife

1. Bildung hoeherer Ebenen aus stabilen Institutionen implementieren.
2. GPDM-Zusammenfassung atomar an den Renderer geben: aktive Tische,
   Kontaktparlamente, Bilanzrest, SMBH-Aktivitaet und abgelehnte Transaktionen.
3. Im F1-Fenster Konfiguration, Modus (disabled/observe/backreaction) und
   harte Equilibrium-Sperre anzeigen. Ein eigener Overlay-Modus ist erst
   danach sinnvoll.
4. README um GPDM, sicheren Start, Diagnosen und Testbefehle ergaenzen.

**Exit:** Ein Lauf mit mehreren Galaxien zeigt nachvollziehbar die
hierarchischen Tische und Kontakte, ohne dass das Renderer- oder
Simulations-Threading Datenrennen erzeugt.

## Testplan

### Unit- und Property-Tests im Modul

- positive und negative Mengen, NaN/Infinity, unbekannte IDs und Kapazitaets-
  ueberlaeufe werden abgelehnt,
- jeder erfolgreiche Batch erhaelt Masse, Energie, linearen und
  Drehimpuls bis zur konfigurierten Toleranz,
- Rollen sind endlich, jeweils in `[0, 1]` und summieren sich auf eins,
- gleiches Snapshot-/Event-Input erzeugt bytegleich sortierte Transaktionen,
  Delegierte, Kontakt-IDs und Diagnosen,
- Kind-/Elternaggregation bleibt korrekt, auch wenn ein Kind keine Mitglieder
  mehr hat,
- Mehr-Uhren verarbeiten nur faellige Tabellen und niemals zweimal im selben
  Frame,
- Institutionen gruenden und loesen sich nur nach Fenster + Hysterese,
- SMBH-Akkretion ueberschreitet nie die Gas- oder Akkretionskapazitaet,
  Wind/Jet bleiben geometriert und Jetpaare haben Null-Nettoimpuls,
- Historienverdichtung bewahrt die aggregierten Kennzahlen.

### Host- und Regressionstests

- Deaktiviertes GPDM: bestehende `scientific_validation` inklusive
  Kraft-, Orbit-, Energie-, Gas- und Renderer-Tests bestehen unveraendert.
- GPDM-Beobachtung: ein Simulationsschritt erzeugt Tabellen und Diagnosen,
  veraendert aber keinen Body.
- Merge: beide Merge-Funktionen melden Gewinner und Verlierer vor der
  Listenmutation; es verbleibt kein verwaistes Konto.
- Gaszuendung und Spawn: Teilnehmer werden genau einmal angelegt bzw.
  umklassifiziert.
- Config reload: jede GPDM-Aenderung erzeugt einen anderen Hash und eine
  frische Runtime mit einem `ConfigReloaded`-Ereignis.
- Equilibrium: mit `enable_disk_equilibrium_mode: true` und GPDM-Beobachtung
  bleiben die vorhandenen weichgemachten Kreisgeschwindigkeits- und
  Schichttests gruen; `HostEffects` ist leer.
- Equilibrium mit bewusst aktiviertem Doppel-Opt-in: Diskenergie,
  Gesamtdrehimpuls und Schwerpunktdrift werden gegen den bestehenden
  Equilibrium-Referenzlauf gemessen und duerfen nur die vorab dokumentierten
  engen Grenzen ueberschreiten.
- Skalierung: Tabellen-/Delegiertenaufbau ist O(N + T log T), baut keinen
  zweiten Body-Octree und nutzt keine quadratische Galaxienpaar-Schleife.

### Abschlussbefehle fuer den implementierenden Agenten

```powershell
cargo fmt --check
cargo test
python ".\ki_autonomie_testfälle.py"
RUN_CONFIG_EVALUATION=1 cargo run --release
```

Der Performance-Modus bleibt maschinenabhaengig und wird nur bewusst
ausgefuehrt:

```powershell
python ".\ki_autonomie_testfälle.py" --performance --budget-ms 250
```

Bei einem Fehlschlag wird zuerst die fehlerhafte Invariante oder der
Hostadapter korrigiert; Tests duerfen nicht durch grosszuegig erhoehte
Toleranzen oder durch Abschalten des GPDM umgangen werden.

## Risiken und explizite Nicht-Ziele

| Risiko | Gegenmassnahme |
|---|---|
| Doppelte oder verlorene Salden bei Host-Merges | Hostevent vor Vec-Mutation, atomarer GPDM-Commit und Quittierung nach Host-Anwendung. |
| Equilibrium-Drift durch Feedbackimpulse | Beobachtungsmodus als Standard, Doppel-Opt-in, verzogerte Effekte, eigene Equilibrium-Regression. |
| Pseudophysik aus Information/Entropie | Diese Konten bleiben diagnostisch, bis ein kalibriertes physikalisches Modell existiert. |
| O(N²)-Kontaktpruefung | Kontaktkandidaten ausschliesslich aus dem Repräsentantenbaum. |
| Nichtdeterminismus durch Hashreihenfolge oder Rayon | BTree-Reihenfolgen, kanonische IDs und serieller GPDM-Commit. |
| Zu grosse Zustandskopplung | DTO-Adapter; `gpdm` kennt keine `Simulation`-Interna und besitzt keine Bodies. |

Nicht Teil dieses Handoffs sind ein relativistischer GR-Solver, MHD, echte
Strahlungstransportrechnung, ein neues Teilchen-Gravitationsgesetz oder eine
Behauptung physikalischer Kalibrierung der abstrakten Informationskonten.
Diese Erweiterungen duerfen erst auf der hier beschriebenen konservativen,
testbaren Buchhaltungsbasis aufsetzen.

## Reihenfolge fuer den naechsten Agenten

1. Diese Datei und die fuenf Konzeptquellen lesen.
2. Phase 0 vollstaendig inklusive Tests abschliessen und erst dann weitergehen.
3. In jeder weiteren Phase erst die beobachtende, bilanzierte Variante
   implementieren; Rueckwirkung nur nach den dort genannten Exit-Kriterien.
4. Keine vorhandenen Disk-Equilibrium- oder Octree-Parameter umdeuten.
5. Nach jeder Phase Formatter, passende Unit-Tests und am Ende die gesamte
   wissenschaftliche Finish-Line ausfuehren.
