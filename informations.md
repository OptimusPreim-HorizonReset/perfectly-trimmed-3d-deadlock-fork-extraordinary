# Property Overview

Diese Datei listet die wichtigsten konfigurierbaren Eigenschaften und Parameter des Projekts auf.
Das Programm lädt beim Start und bei Dateiaenderung automatisch den ersten gefundenen Konfigurationsblock aus informations.md.
Ungültige Werte werden ignoriert; für sie tritt der jeweilige Code-Default in Kraft.
Zusätzliche Ausführungsmodi können über Umgebungsvariablen aktiviert werden:
#   RUN_CONFIG_EVALUATION=1  -> kurzes disk-equilibrium-Validierungsprofil ausführen und beenden.
#   RUN_CONFIG_SWEEP=1       -> Toomre-Q-Sweep ausführen und beenden.

```config
# === SIMULATION BASICS ===
# dt: Zeitschrittweite pro Frame
# theta: Barnes-Hut-Winkelparameter für Octree-Approximation
# epsilon: Plummer-Softening für numerische Stabilität:
#   a = G*m*r / (r² + epsilon²)^(3/2)
# n: Anzahl der Disk-Partikel pro Galaxie (ohne Zentralmasse)

dt: 50000.0
theta: 0.1
epsilon: 1.0
n: 500000

# === GALAXY STRUCTURE ===
# inner_radius: Radius der zentralen Masse / des Bulge
# outer_radius: Außenradius der galaktischen Scheibe
# central_mass: Masse des zentralen schwarzen Lochs / Galaxiezentrums
# particle_mass_range: Min/Max-Masse der generierten Element-Teilchen (Format: min, max)
#   Dieser globale Bereich wirkt als einheitlicher Baseline-Regler für alle Elemente.
#   In der Galaxy-as-Atom-Erzeugung wird er je Element spezifisch interpretiert und
#   für Bulge-, Disk- und Halo-Partikel individuell ausgesteuert.
#
# enable_elemental_mass_dimension_visualization: Wenn aktiviert, werden Partikelfarben
#   stärker von Prime-/Massendimensionen bestimmt. Wenn deaktiviert, verwendet der
#   Renderer segmenttypische Galaxienfarben, damit SMBH, Bulge, Scheibe, Gas und
#   Satelliten als typische astronomische Regionen unterscheidbar bleiben.

inner_radius: 01.1
outer_radius: 14000.0
central_mass: 0.0002
particle_mass_range: 0.0000000001, 0.0000000002

# === SPAWN ZONE CONFIGURATION ===
# outer_ring_spawn_zone_inner_ratio: Innerer Radius der Spawn-Zone (als Verhältnis zu outer_radius)
# outer_ring_spawn_zone_outer_ratio: Äußerer Radius der Spawn-Zone (als Verhältnis zu outer_radius)
# accretion_spawn_rate: Wahrscheinlichkeit pro Schritt für einen neuen Partikel-Spawn

outer_ring_spawn_zone_inner_ratio: 0.05
outer_ring_spawn_zone_outer_ratio: 1.0
accretion_spawn_rate: 0.1

# === HYDRODYNAMIC FORCES ===
# inflow_strength: Stärke der radialen Einströmungskraft
# restore_strength: Stärke der vertikalen Rückstellungskraft

inflow_strength: 0.00000000000000005
restore_strength: 0.000000000000052

# === GAS DYNAMICS ===
# gas_enabled: Activate lightweight SPH-inspired gas particle dynamics
# gas_meta_particle_count: Target number of lightweight gas particles in galactic disks
# gas_target_neighbors: Desired neighbor count used to adapt smoothing radius
# gas_volume_enabled: Enable an additional volumetric gas component across galaxy pairs
# gas_volume_count: Number of volume gas particles distributed across the simulation region
# gas_volume_radius_factor: Radius factor used to spread volumetric gas around each galaxy pair
# gas_update_interval: Frames between full gas updates to trade detail for smoother FPS
# gas_h_min / gas_h_max: Minimum and maximum smoothing radius in world units
# gas_pressure_coefficient: Pressure strength multiplier for gas forces
# gas_condensation_strength: Strength of density-driven gas condensation toward dense regions
# gas_merge_distance_factor: Distance multiplier used for gas-gas merging checks
# gas_merge_velocity_max: Maximum relative velocity at which gas particles may merge
# gas_ignition_density: Density threshold above which dense gas may convert into a star body
# gas_ignition_temperature: Minimum temperature for gas ignition into a star body
# gas_ignition_mass: Minimum mass required for gas ignition into a star body
# gas_rest_density: Reference density for pressure calculation
# gas_cooling_rate: Rate at which gas temperature drops with density
# gas_relaxation_strength: Damping of out-of-plane gas motion to rebuild disks
# gas_render_opacity: Base opacity for gas billboards
# gas_motion_blur_strength: Motion blur effect intensity for gas rendering
# gas_density_color_scale: Strength of density-based color modulation
# gas_temperature_color_scale: Strength of temperature-based color modulation

gas_enabled: false
gas_meta_particle_count: 100000
gas_target_neighbors: 12
gas_volume_enabled: false
gas_volume_count: 180
gas_volume_radius_factor: 0.4
gas_update_interval: 2
gas_h_min: 0.3
gas_h_max: 7.0
gas_pressure_coefficient: 0.045
gas_condensation_strength: 0.12
gas_merge_distance_factor: 1.25
gas_merge_velocity_max: 0.14
gas_ignition_density: 0.0006
gas_ignition_temperature: 0.68
gas_ignition_mass: 0.00001
gas_rest_density: 0.3
gas_cooling_rate: 0.08
gas_relaxation_strength: 0.26
gas_render_opacity: 0.72
gas_motion_blur_strength: 0.38
gas_density_color_scale: 1.4
gas_temperature_color_scale: 0.9

# === GALAXY INTERACTION ===
# galaxy_separation_factor: Abstand zwischen Galaxien (multipliziert mit outer_radius)
# galaxy_count: Gesamtzahl der initial erzeugten Galaxien (paarweise angeordnet, auf gerade Zahl abgerundet)
# galaxy_volume_scatter_factor: Skaliert das Volumen, in dem alle Galaxienpaare chaotisch verteilt werden
# prob_merge: Wahrscheinlichkeit für verschmelzungsnahe Begegnungen
# prob_repeated: Wahrscheinlichkeit für periodisch wiederkehrende Begegnungen
# prob_flyby: Wahrscheinlichkeit für intensive Flybys

galaxy_separation_factor: 0.40
galaxy_count: 2
galaxy_volume_scatter_factor: 02.0
prob_merge: 0.35
prob_repeated: 0.35
prob_flyby: 0.30

# === INTERACTION SPEED FACTORS ===
# merge_speed_factor: Skalierung der Relativgeschwindigkeit für Merge-Szenarien
# repeated_speed_factor: Skalierung der Relativgeschwindigkeit für wiederkehrende Szenarien
# flyby_speed_factor: Skalierung der Relativgeschwindigkeit für Flybys

merge_speed_factor: 0.10
repeated_speed_factor: 0.70
flyby_speed_factor: 0.90

# === INTERACTION ANGLES ===
# merge_angle: Annäherungswinkel für Merge-Szenarien
# repeated_angle: Annäherungswinkel für wiederkehrende Begegnungen
# flyby_angle: Annäherungswinkel für Flybys

merge_angle: 0.05
repeated_angle: 0.25
flyby_angle: 0.35

# === COMPUTATION INTERVALS ===
# collision_interval: Wie oft pro Frame Kollisionen berechnet werden (höher = weniger oft)
# attract_interval: Wie oft pro Frame Gravitationskräfte berechnet werden (höher = weniger oft)

collision_interval: 1
attract_interval: 1

# === PERFORMANCE TUNING ===
# performance_dynamic_interval_scaling: Aktiviert eine automatische Anpassung der
#   collision/attract-Intervalle bei sehr vielen Körpern, um FPS zu verbessern.
# performance_body_count_threshold: Anzahl der Körper, ab der dynamische
#   Intervallskalierung beginnt.
# performance_max_interval_scale: Maximaler Multiplikator für die Intervalle.
# performance_render_body_limit: Maximal sichtbare Körper im Renderer. Bei größeren
#   Zahlen werden nur Stichproben gezeichnet, um die Framerate zu verbessern.
# performance_render_sample_ratio: Verhältnis der gerenderten Körper im Vergleich
#   zum berechneten Renderlimit (0.1 = 10 % der erlaubten Körper werden gezeichnet).
# performance_high_count_mode: Aktiviert zusätzliche Hochlast-Optimierungen bei sehr
#   großen Körperzahlen. Standardmäßig aus, damit die Simulation bei normalen Counts
#   maximal genau bleibt.
# performance_disable_adaptive_updates_in_high_count: Wenn High-Count-Modus aktiv
#   ist, überspringt dieser Schalter adaptive Theta/Multi-
#   pole- und adaptive Zeitschritt-Berechnungen bei sehr vielen Körpern.
# performance_octree_reserve_factor: Multiplikator für die Octree-Reserve-Kapazität.
#   Niedrigere Werte sparen Speicher & Reserve-Kosten für große Körperzahlen.
#
# Zusätzliche Laufzeitprofilerung kann über Umgebungsvariablen aktiviert werden:
#   PROFILE_RUNTIME=1          -> Durchschnittswerte über Schritte und Renderzeit
#   PROFILE_FRAMES=50          -> Anzahl der Frames je Profilzyklus
#   PROFILE_MAX_FRAMES=500     -> Simulation nach der Profilzahl beenden
#   PROFILE_STEP_COMPONENTS=1  -> Detaillierte Sub-Komponenten-Timings pro Schritt
#
# Intern werden Kollisionsgitterpuffer jetzt wiederverwendet, um die Step-Zeit
#   bei hohen Körperzahlen zu reduzieren.

performance_dynamic_interval_scaling: true
performance_body_count_threshold: 200000
performance_max_interval_scale: 2
performance_render_body_limit: 200000
performance_render_sample_ratio: 1.0
performance_high_count_mode: false
performance_disable_adaptive_updates_in_high_count: false
performance_octree_reserve_factor: 4.0

# === PARTICLE PHYSICS ===
# orbital_speed_multiplier: Multiplikator für die Angular-Speed von neuen Partikeln
# spawn_angular_speed_base: Basis-Wert für Angular-Speed bei Spawns
# spawn_angular_speed_range: Zufällige Variation der Angular-Speed bei Spawns
# spin_speed_multiplier: Globale Skalierung der Eigendrehung aller Partikel

orbital_speed_multiplier: 0.025
spawn_angular_speed_base: 0.3
spawn_angular_speed_range: 0.5
spin_speed_multiplier: 0.5

# === BODY DEFORMATION ===
# equatorial_growth_factor: Wie viel der Äquator-Radius bei Rotation wächst
# polar_flattening_factor: Wie viel der Polar-Radius bei Rotation flacht ab
# radius_scale: Skalierungsfaktor für alle Partikel-Radii

equatorial_growth_factor: 0.18
polar_flattening_factor: 0.14
radius_scale: 0.5

# === DEPTH PERCEPTION ===
# depth_scale_factor: Faktor für die Z-Position bei der Tiefenwahrnehmung
# spacetime_dilation_factor: Stärke der Gravitationswellenstrahlung für spiralisierende Annäherung der Zentren

depth_scale_factor: 0.002
spacetime_dilation_factor: 0.50

# === DISK EQUILIBRIUM MODE ===
# enable_disk_equilibrium_mode: Aktiviert einen stabilitätsorientierten Scheibenmodus mit sech²-Vertikalprofil und driftkorrigierten Umlaufgeschwindigkeiten.
#   Wenn aktiviert, werden einige legacy Solver-Einstellungen durch den Mode ersetzt,
#   u. a. `theta`, `epsilon`, `collision_interval`, `attract_interval`, dynamische Intervallskalierung
#   und High-Count-Abkürzungen.
# disk_equilibrium_toomre_q: Ziel-Toomre-Q für die Scheibendispersion.
# disk_equilibrium_scale_height_factor: Maßstabsfaktor für die sech²-Vertikalverteilung (als Anteil von outer_radius).
# disk_equilibrium_asymmetric_drift_strength: Stärke der asymmetrischen Driftkorrektur für die Umlaufgeschwindigkeit.
# disk_equilibrium_softening_ratio: Softening als Anteil von outer_radius, wenn der Modus aktiviert ist.
#   dt_min / dt_max werden im Modus aus dt abgeleitet, um lokale Zeitschritte konsistent zu halten.
# disk_equilibrium_theta: Barnes-Hut-Öffnungswinkel für den Modus.
# Kreisbahngeschwindigkeiten verwenden dasselbe Plummer-Softening wie die Laufzeitkraft.
enable_disk_equilibrium_mode: true
disk_equilibrium_toomre_q: 1.3
disk_equilibrium_scale_height_factor: 0.025
disk_equilibrium_asymmetric_drift_strength: 0.12
disk_equilibrium_softening_ratio: 0.025
disk_equilibrium_theta: 0.60

# === GALAXY-AS-ATOM SIMULATION ===
# enable_galactic_atom_simulation: true = use the galaxy-as-atom simulation instead of paired galaxy disks
# enable_elemental_galaxy_pair_mode: true = use 118 periodic-table body types for each galaxy in pair mode (requires n > 1000)
# enable_elemental_simulation: legacy alias for enable_galactic_atom_simulation
# element_atomic_numbers: comma-separated Liste der Ordnungszahlen, die als Elementarsysteme erzeugt werden sollen
#   z. B. 1,2,6,8,26; Leer = 1..element_max_atomic_number
# element_pair_atomic_numbers: comma-separated Liste der Ordnungszahlen, die im Galaxy-Pair-Elementmodus verwendet werden sollen
#   Leer = alle 1..118 Elemente
#   Das Element-LOD behaelt pro Galaxie mindestens einen Body jedes ausgewaehlten Elements.
# element_pair_core_atomic_numbers: gesonderte Elementauswahl fuer Galaxy-Center/Core-Partikel
#   Leer = Kandidaten aus element_pair_atomic_numbers; die Auswahl erfolgt unabhaengig und stabilitaetsgewichtet
# enable_chemistry_compass_coloring: nutzt die chemischen Familienfarben aus chemiekompass.md fuer alle Element-Bodies
# enable_elemental_binding_response: koppelt die Kollisionselastizitaet an die berechnete Bindungstendenz
# elemental_binding_response_strength: 0..1, Staerke dieser analogen, inelastischen Kollisionsantwort
# element_atomic_count: Gesamtzahl der simulierten Elementarsysteme; bei größeren Werten wird die Auswahl wiederholt
# element_max_atomic_number: Fallback-Maximum, wenn keine element_atomic_numbers angegeben sind
#   Die Galaxy-as-Atom-Erzeugung sorgt dafür, dass besonders leichte Elemente wie Wasserstoff
#   mindestens 400 Partikel umfassen, damit die Struktur des Kern-/Bulge-/Scheiben-/Halo-Systems
#   ausreichend detailliert sichtbar wird.
# element_universe_scatter_factor: multiplies outer_radius to distribute galaxies in space
# element segment merge rules: jede atomare Segmentgruppe (Core/Bulge/Orbital) verschmilzt nur mit dem gleichen Segmenttyp desselben Elements.
#   Core/SMBH-Zentren fusionieren zusätzlich erst nach mehr als 5 Frames dauerhafter Berührung.
# element_center_mass_unit: base mass unit for the central SMBH / nucleus scaling
# element_orbital_mass_unit: mass scale for spawned orbital particles in galaxy-as-atom systems
# element_radius_scale: base radius scaling for generated galactic structures
#   dieses Basismaß wird per Element weiter angepasst: die 7 Perioden erhalten
#   jeweils eine eigene Größen-Dimension, die mit der primzahlbasierten
#   Stabilitätsfrequenz und der positionellen Verteilung innerhalb der Periode
#   verrechnet wird.
# element_prime_alpha: weights the prime-dimension coupling for galaxy analog magnetism
# element_prime_beta: weights the prime-dimension coupling for bulge/SMBH curvature
# element_light_energy: light interaction scaling factor for emitted radiation analog
# enable_elemental_gravitation_tuning: use element mass dimension for Octree gravitation instead of raw mass
# enable_elemental_mass_dimension_visualization: allow mass-dimension coloring in the renderer
# element_initial_sorting_mode: 0 = none, 1 = atomic number, 2 = atomic weight
# element_initial_grouping_mode: 0 = none, 1 = morphology clusters, 2 = orbital shell bands
# element_group_spacing: relative spacing between grouped element clusters
# element_group_internal_velocity_scale: adjust relative velocity inside grouped element systems
# element_sample_orbit_radius_scale: adjusts the radius of spawned sample orbit systems
# element_sample_orbit_speed_factor: adjusts the internal orbital speed of spawned samples
enable_galactic_atom_simulation: false
enable_elemental_galaxy_pair_mode: true
enable_elemental_simulation: true
element_atomic_numbers: 1
element_pair_atomic_numbers:
element_pair_core_atomic_numbers:
enable_chemistry_compass_coloring: true
enable_elemental_binding_response: true
elemental_binding_response_strength: 0.35
element_atomic_count: 100000
# element_max_atomic_number: 118
#   Fallbackwert für den Fall, dass element_atomic_numbers leer bleibt.
element_universe_scatter_factor: 1.8
element_center_mass_unit: 01.0
element_orbital_mass_unit: 1.0
element_radius_scale: 5.0
element_prime_alpha: 0.09
element_prime_beta: 0.06
element_light_energy: 2.0
enable_elemental_gravitation_tuning: true
enable_elemental_mass_dimension_visualization: false
element_initial_sorting_mode: 0
element_initial_grouping_mode: 0
element_group_spacing: 0.65
element_group_internal_velocity_scale: 01.0
element_sample_orbit_radius_scale: 1.0
element_sample_orbit_speed_factor: 01.0

# === SCIENTIFIC ELEMENT GRAVITY ===
# enable_scientific_element_gravity: Wenn true, wird die Gravitationsanalogie jedes
#   Elements aus Atomgewicht, Elektronegativitaet, Radius und Reaktivitaet abgeleitet
#   statt aus der klassischen Primzahldimension. Schwere, reaktive Elemente ziehen
#   staerker an; Edelgase bleiben schwach. Wasserstoff wird auf beta normiert.
# element_gravity_*_weight: Gewichtung der vier Terme in der Gravitationsformel.
#   Standard 1.0 / 0.4 / 0.3 / 0.2. Aenderungen erfordern Neuabstimmung der
#   visuellen Skalierung.
enable_scientific_element_gravity: true
element_gravity_mass_weight: 1.0
element_gravity_electronegativity_weight: 0.4
element_gravity_radius_weight: 10.0
element_gravity_reactivity_weight: 0.2

# === MOLECULAR BONDING ===
# enable_molecular_bonding: Wenn true, bilden geeignete unterschiedliche Elemente
#   bei Ueberlappung persistente Molekuele statt nur elastisch abzuprallen.
# molecule_bond_strength_factor: Skalierung der Bindungsfestigkeit (0..5).
# molecule_break_energy_threshold: Relative Geschwindigkeitsschwelle, bei der eine
#   Bindung wieder bricht (0..10).
# molecule_max_bodies: Maximale Anzahl an Koerpern in einem dynamischen Molekuel.
# molecule_identify_compounds: Wenn true, werden bekannte natuerliche Verbindungen
#   (H2O, NaCl, CO2, CH4, ...) anhand der Stoechiometrie erkannt und benannt.
enable_molecular_bonding: true
molecule_bond_strength_factor: 1.0
molecule_break_energy_threshold: 0.8
molecule_max_bodies: 12
molecule_identify_compounds: true

# === INTERACTIVE ELEMENT SAMPLING ===
# Open the sample chooser with the `^` key (Grave) and select an element from 1 to 118.
# Place a new element sample by right-clicking and dragging in the viewport.
# If right-click is unavailable, press `P` while the selector is open to place the sample at the cursor.
# Normal mode: left-click or left-drag places one body on the camera-facing target plane; drag distance controls its mass.
# Middle-mouse drag orbits the camera. The mouse wheel zooms even when the HUD is visible.
# Pointer actions over an open settings window remain reserved for the UI.
#
# Overlay modes (number keys):
#   5 - Default: normal rendering with mass-dimension/segment coloring.
#   6 - Process Dynamics: highlights molecule bonds, collisions and merge events.
#   7 - Activity Heatmap: colors each particle by the currently dominant process
#       (gravity, collision/merge, gas dynamics, molecular bonding, adaptive stress).

# === ADAPTIVE OPTIMIZATION ===
# enable_adaptive_theta: Region-basierte per-particle Barnes-Hut-Winkel
# enable_adaptive_multipole: Adaptive Multipolordnung (1=Monopol, 2=Dipol-Hook, 3=Quadrupol-Hook)
# enable_mixed_precision: Vereinfachte Distanz-Quantisierung für ferne Zellen
# enable_adaptive_timestep: Per-particle Δt nach Hermite-artigem Kriterium
# adaptive_theta_*: θᵢ für ruhige / mittlere / kritische Regionen
# adaptive_multipole_*: Lᵢ für ruhige / mittlere / kritische Regionen
# adaptive_dynamics_*_threshold: Grenzwerte für Regionsklassifikation (0..1)
# eta_timestep: η-Faktor für Δtᵢ = η·sqrt(|a|/|j|)
# dt_min / dt_max: Clamp für lokale Zeitschritte (0.0 => automatisch aus dt abgeleitet)
# max_energy_error: Feedback-Schwelle für aggressivere Approximationen
enable_adaptive_theta: true
enable_adaptive_multipole: true
enable_mixed_precision: true
enable_adaptive_timestep: true
adaptive_theta_quiet: 1.0
adaptive_theta_medium: 0.5
adaptive_theta_critical: 0.1
adaptive_multipole_quiet: 1
adaptive_multipole_medium: 2
adaptive_multipole_critical: 3
adaptive_dynamics_quiet_threshold: 0.001
adaptive_dynamics_critical_threshold: 0.35
eta_timestep: 100000.0
dt_min: 0.0
dt_max: 0.0
max_energy_error: 0.5
```

## 1. Galaxie-Template / Spawn-Konfiguration

### `src/galaxy_templates.rs`

- `n` – Anzahl der Disc-Partikel ohne Zentralmasse.
- `inner_radius` – Radius der zentralen Masse / Bulge.
- `outer_radius` – Außenradius der galaktischen Scheibe.
- `central_mass` – Masse des zentralen Körpers (Schwarzes Loch / Galaxiezentrum).
- `particle_mass_range` – `(min, max)`-Werte für zufällig erzeugte Partikelmasse.
- `accretion_spawn_rate` – Wahrscheinlichkeit, dass ein neuer Partikel pro Schritt gespawnt wird.
- `outer_ring_spawn_zone_inner_radius` – innerer Radius der äußeren Ring-Spawn-Zone.
- `outer_ring_spawn_zone_outer_radius` – äußerer Radius der äußeren Ring-Spawn-Zone.

Direkte Änderungen:

- `src/galaxy_templates.rs`

## 2. Simulation und Zeitsteuerung

### `src/simulation.rs`

- `dt` – Zeitschrittweite der Simulation.
- `n` in `Simulation::new()` – Initiale Anzahl der Partikel in `uniform_disc()`.
- `theta` – Barnes-Hut-Winkelparameter für den Octree.
- `epsilon` – Softening-Faktor für Gravitationsberechnung.
- `spawn_accretion()` – Hier wird der automatische Spawn durchgeführt.
- `apply_hydrodynamic_inflow()` – Hier laufen die hydrodynamischen Rückführkräfte.

Wichtige Parameter in `spawn_accretion()`:

- `spawn_start_r` – Startradius der äußeren Ring-Spawn-Zone.
- `outer_r` – Außenradius des Spawn-Zonens.
- `accretion_spawn_rate` – Spawn-Wahrscheinlichkeit pro Schritt.

Wichtige Parameter zur Galaxien-Begegnung:

- `prob_merge` – Wahrscheinlichkeit für verschmelzungsnahe Begegnungen.
- `prob_repeated` – Wahrscheinlichkeit für periodisch wiederkehrende Begegnungen.
- `prob_flyby` – Wahrscheinlichkeit für intensive Flybys.
- `merge_speed_factor` – Geschwindigkeitsskalierung für Merge-Szenarien.
- `repeated_speed_factor` – Geschwindigkeitsskalierung für wiederkehrende Begegnungen.
- `flyby_speed_factor` – Geschwindigkeitsskalierung für Flybys.
- `merge_angle` – Annäherungswinkel für Merge-Szenarien.
- `repeated_angle` – Annäherungswinkel für wiederkehrende Begegnungen.
- `flyby_angle` – Annäherungswinkel für Flybys.

Wichtige Parameter in `apply_hydrodynamic_inflow()`:

- `inflow_strength` – Stärke des Lateraleinflusses von Norden/Süden.
- `restore_strength` – Stärke der vertikalen Rückführungsbeschleunigung.

Direkte Änderungen:

- `src/simulation.rs`

## 3. Partikel-Eigenschaften

### `src/body.rs`

- `pos` – Position im Raum.
- `vel` – Geschwindigkeit.
- `acc` – Beschleunigung.
- `mass` – Masse des Partikels.
- `base_radius` – Basisradius vor Formänderung durch Spin.
- `equatorial_radius` / `polar_radius` – Azimutale und polare Radien zur Darstellung.
- `rotation_axis` – Achse für Spin-Rotation.
- `angular_speed` – Rotationsgeschwindigkeit.

Direkte Änderungen:

- `src/body.rs`

## 4. Renderer- und Anzeigeoptionen

### `src/renderer.rs`

- `camera_target` / `camera_distance` / `camera_yaw` / `camera_pitch` / `camera_fov` – Kamera-Parameter.
- `camera_speed` / `camera_rotate_speed` – Kamerabewegungsgeschwindigkeit.
- `show_bodies` – Anzeigen der Partikel.
- `show_quadtree` – Anzeigen der Octree-Struktur.
- `show_spin_axes` – Anzeigen der Spin-Achsen.
- `show_mass_dimension_coloring` – Renderer-Färbung nach Massendimension.
- `show_element_debug_labels` – Anzeige zusätzlicher Elementdiagnosen.

Direkte Änderungen:

- `src/renderer.rs`

## 5. Sonstige relevante Dateien

- `src/main.rs` – Startpunkt und Rendering-Schleife.
- `src/utils.rs` – Erzeugung der initialen Disc-Verteilung.

Direkte Änderungen:

- `src/main.rs`
- `src/utils.rs`

---

## Hinweise

- Für direkte Anpassungen sind die oben genannten Dateien die zentralen Punkte.
- `informations.md` dient als Einstieg, um schnell zwischen Konfigurationsparametern zu wechseln.
- Ungültige Werte im `config`-Block werden verworfen; das Programm fällt auf Code-Default-Werte zurück.
- `dt_min` und `dt_max` werden automatisch aus `dt` abgeleitet, wenn sie `0.0` oder negativ gesetzt werden.
- Änderungen an `informations.md` werden zur Laufzeit erkannt, und die Simulation wird bei erkannter Änderung neu initialisiert.
