# Property Overview

Diese Datei listet die wichtigsten konfigurierbaren Eigenschaften und Parameter des Projekts auf.
Änderungen werden automatisch erkannt und die Simulation neu initialisiert (wenn möglich).

```config
# === SIMULATION BASICS ===
# dt: Zeitschrittweite pro Frame
# theta: Barnes-Hut-Winkelparameter für Octree-Approximation
# epsilon: Gravitational Softening für numerische Stabilität
# n: Anzahl der Disk-Partikel pro Galaxie (ohne Zentralmasse)

dt: 1.0
theta: 1.0
epsilon: 1.1
n: 100

# === GALAXY STRUCTURE ===
# inner_radius: Radius der zentralen Masse / des Bulge
# outer_radius: Außenradius der galaktischen Scheibe
# central_mass: Masse des zentralen schwarzen Lochs / Galaxiezentrums
# particle_mass_range: Min/Max Masse der Disk-Partikel (Format: min, max)

inner_radius: 1.0
outer_radius: 2.03
central_mass: 0.0025
particle_mass_range: 0.0000005, 0.000005

# === SPAWN ZONE CONFIGURATION ===
# outer_ring_spawn_zone_inner_ratio: Innerer Radius der Spawn-Zone (als Verhältnis zu outer_radius)
# outer_ring_spawn_zone_outer_ratio: Äußerer Radius der Spawn-Zone (als Verhältnis zu outer_radius)
# accretion_spawn_rate: Wahrscheinlichkeit pro Step für einen neuen Partikel-Spawn

outer_ring_spawn_zone_inner_ratio: 0.33
outer_ring_spawn_zone_outer_ratio: 1.0
accretion_spawn_rate: 0.00

# === HYDRODYNAMIC FORCES ===
# inflow_strength: Stärke der radialen Einströmungskraft
# restore_strength: Stärke der vertikalen Rückstellungskraft
# adaptive_softening_enabled: Verwende adaptive, dichte-abhängige Softening-Länge
# sph_enabled: Aktiviert SPH-/Hydro-Kernkräfte zur Kopplung von Gas und Sternen
# softening_scale_factor: Basis-Smoothing-Länge in Vielfachen von epsilon
# softening_min_factor: Minimaler Faktor für adaptive Softening-Länge
# softening_max_factor: Maximaler Faktor für adaptive Softening-Länge
# hydro_pressure_strength: Stärke des SPH-Druckterms
# hydro_kernel_factor: Wie weit die Nachbarschaft für SPH-Kernel-Interaktionen reicht (in h)
# render_update_interval: Frames zwischen Renderer-Datenaktualisierungen (höhere Werte sparen CPU)
# performance_mode: Schaltet einen schnellen Modus ein, der SPH/Adaptive-Softening deaktiviert und Intervalle erhöht
# ultra_performance_mode: Schaltet den maximalen FPS-Modus ein, der alle Zusatzphysiken minimiert und die Render-Aktualisierung stark ausdünnt

inflow_strength: 0.0000040
restore_strength: 0.00000011
adaptive_softening_enabled: true
sph_enabled: true
softening_scale_factor: 4.0
softening_min_factor: 0.5
softening_max_factor: 4.0
hydro_pressure_strength: 0.02
hydro_kernel_factor: 2.0
render_update_interval: 2
performance_mode: true
ultra_performance_mode: false

# === GALAXY INTERACTION ===
# galaxy_separation_factor: Abstand zwischen Galaxien (Multipliziert mit outer_radius)
# galaxy_count: Gesamtzahl der initial erzeugten Galaxien (paarweise angeordnet, auf gerade Zahl abgerundet)
# galaxy_volume_scatter_factor: Skaliert das Volumen, in dem alle Galaxienpaare chaotisch verteilt werden
# prob_merge: Wahrscheinlichkeit für verschmelzungsnahe Begegnungen
# prob_repeated: Wahrscheinlichkeit für periodisch wiederkehrende Begegnungen
# prob_flyby: Wahrscheinlichkeit für intensive Flybys

galaxy_separation_factor: 25.0
galaxy_count: 20
galaxy_volume_scatter_factor: 6.0
prob_merge: 0.30
prob_repeated: 0.35
prob_flyby: 0.35

# === INTERACTION SPEED FACTORS ===
# merge_speed_factor: Skalierung der Relativgeschwindigkeit für Merge-Szenarien
# repeated_speed_factor: Skalierung der Relativgeschwindigkeit für wiederkehrende Szenarien
# flyby_speed_factor: Skalierung der Relativgeschwindigkeit für Flybys

merge_speed_factor: 01.0
repeated_speed_factor: 01.0
flyby_speed_factor: 01.60

# === INTERACTION ANGLES ===
# merge_angle: Annäherungswinkel für Merge-Szenarien
# repeated_angle: Annäherungswinkel für wiederkehrende Szenarien
# flyby_angle: Annäherungswinkel für Flybys

merge_angle: 5.25
repeated_angle: 9.45
flyby_angle: 10.05

# === COMPUTATION INTERVALS ===
# collision_interval: Wie oft pro Frame Kollisionen berechnet werden (höher = weniger oft)
# attract_interval: Wie oft pro Frame Gravitationskräfte berechnet werden (höher = weniger oft)

collision_interval: 16
attract_interval: 1

# === PARTICLE PHYSICS ===
# orbital_speed_multiplier: Multiplikator für die Angular-Speed von neuen Partikeln
# spawn_angular_speed_base: Basis-Wert für Angular-Speed bei Spawns
# spawn_angular_speed_range: Zufällige Variation der Angular-Speed bei Spawns
# spin_speed_multiplier: Globale Skalierung der Eigendrehung aller Partikel

orbital_speed_multiplier: 0.025
spawn_angular_speed_base: 0.3
spawn_angular_speed_range: 0.5
spin_speed_multiplier: 8.0

# === BODY DEFORMATION ===
# equatorial_growth_factor: Wie viel der Äquator-Radius bei Rotation wächst
# polar_flattening_factor: Wie viel der Polar-Radius bei Rotation flacht ab
# radius_scale: Skalierungsfaktor für alle Partikel-Radii

equatorial_growth_factor: 0.18
polar_flattening_factor: 0.18
radius_scale: 0.5

# === DEPTH PERCEPTION ===
# depth_scale_factor: Faktor für die Z-Position bei der Tiefenwahrnehmung
# spacetime_dilation_factor: Stärke der Gravitationswellenstrahlung für spiralisierende Annäherung der Zentren (höher = schnellere Spirale)

depth_scale_factor: 0.002
spacetime_dilation_factor: 0.9
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

- `f:/Games/deadlock_fork_but_niice2/src/galaxy_templates.rs`

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

- `f:/Games/deadlock_fork_but_niice2/src/simulation.rs`

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

- `f:/Games/deadlock_fork_but_niice2/src/body.rs`

## 4. Renderer- und Anzeigeoptionen

### `src/renderer.rs`

- `camera_target` / `camera_distance` / `camera_yaw` / `camera_pitch` / `camera_fov` – Kamera-Parameter.
- `camera_speed` / `camera_rotate_speed` – Kamerabewegungsgeschwindigkeit.
- `show_bodies` – Anzeigen der Partikel.
- `show_quadtree` – Anzeigen der Octree-Struktur.
- `show_spin_axes` – Anzeigen der Spin-Achsen.

Direkte Änderungen:

- `f:/Games/deadlock_fork_but_niice2/src/renderer.rs`

## 5. Sonstige relevante Dateien

- `src/main.rs` – Startpunkt und Rendering-Schleife.
- `src/utils.rs` – Erzeugung der initialen Disc-Verteilung.

Direkte Änderungen:

- `f:/Games/deadlock_fork_but_niice2/src/main.rs`
- `f:/Games/deadlock_fork_but_niice2/src/utils.rs`

---

## Hinweise

- Für direkte Anpassungen sind die oben genannten Dateien die zentralen Punkte.
- `informations.md` dient als Einstieg, um schnell zwischen Konfigurationsparametern zu wechseln.
