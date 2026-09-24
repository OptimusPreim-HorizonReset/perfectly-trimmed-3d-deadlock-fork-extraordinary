# Barnes-Hut

This is the official repository for the code shown in [How to make HUGE N-Body Simulations (N=1,000,000+)](https://youtu.be/nZHjD3cI-EU)

This repository consists of three branches:

1. [The master branch](https://github.com/DeadlockCode/barnes-hut).

   This is the code shown in the video and is my (mostly) faithful implementation of the original algorithm as described in the Barnes-Hut paper.

2. [The improved branch](https://github.com/DeadlockCode/barnes-hut/tree/improved).

   This modifies the original algorithm by a) storing the nodes in a cache friendly order and b) allowing multiple bodies to inhabit the same leaf node.

3. [The parallel branch](https://github.com/DeadlockCode/barnes-hut/tree/parallel).

   This is a crude attempt at parallelizing the improved branch to show its potential.

## Guide

1. Install [Rust](https://www.rust-lang.org/tools/install)
2. Clone the repository
3. If you're **not** on Windows, follow [this](https://github.com/DeadlockCode/n-body/issues/1)
4. Checkout the desired branch
5. Open the folder in a terminal
6. Run 'cargo run --release'
7. Enjoy

This fork includes an optional galaxy-as-atom simulation mode that transforms the initial dataset from paired galaxy disks into a structured periodic-like sequence of galactic atoms. Each system uses a central SMBH + bulge nucleus with orbiting disk and halo bodies that map to orbital and valence electron analogs.

## Run Release Build

1. Open PowerShell or CMD im Projektordner.
2. Erstelle den Release-Build:
   - `cargo build --release`
   - oder führe `.uild-release.ps1` aus, wenn `cargo` in der Shell nicht gefunden wird
3. Starte das fertige Programm:
   - `target\release\barnes-hut.exe`
   - oder doppel-klicke `start-release.bat`
   - oder führe `run-release.ps1` in PowerShell aus
## Neue Features

- 3D-Physik mit einem echten Octree für den Barnes-Hut-Algorithmus
- individuelle rotierende Körper im Nullschwerfeld
- rotationsabhängige Formanpassung (äquatorial aufgebläht, polar abgeflacht)
- Rotationsachsen aller Partikel stehen senkrecht zur Ursprungsscheibe
- Parallelisierte Aktualisierung der Körperbeschleunigung und Positionsupdates
- 3D-Kollisionsbroadphase über eine räumliche Gitter-Hashmap
- Dynamische Performance-Skalierung für hohe Körperzahlen und Render-LOD
- Leichtgewichtige SPH-inspirierte Gasdynamik für rotierende Scheiben und kollisionsgetriebene Gaswolken
- Billboard-basierte Gasvisualisierung mit Dichte-/Temperatur-Färbung und Motion-Blur
- GPU-billboards werden über die Quarkstrom-Instanced-Circle-Pipeline gerendert
- Optionaler GPU-Billboard-Shader-Vorlage für Gasvisualisierung in `shaders/gas_billboard.glsl`
- Hochlast-Performance-Modus mit adaptiver Octree-Reserve und optionalen High-Count-spezifischen Optimierungen
- Wissenschaftliche Element-Gravitation (optional): Anziehung pro Element aus Atomgewicht, Elektronegativitaet, Radius und Reaktivitaet abgeleitet
- Molekulare Bindungen (optional): Bildung persistenter Molekuele aus unterschiedlichen Elementen sowie Zweiatom-Molekuele wie H₂, N₂, O₂
- Bibliothek natuerlicher Verbindungen (H₂O, NaCl, CO₂, CH₄, Glukose, Minerale, einfache Organische) mit Stoechiometrie-Erkennung
## Wenn `cargo` nicht erkannt wird

- Starte das Terminal neu, damit die PATH-Änderung wirksam wird.
- Falls das nicht hilft, benutze `.uild-release.ps1`.

## Startverknüpfung

- Erstelle eine Windows-Verknüpfung auf `start-release.bat`.
- Ziel: `f:\Games\deadlock_fork_but_niice2\start-release.bat`
- Start in: `f:\Games\deadlock_fork_but_niice2`

> Tipp: Ziehe `start-release.bat` mit der rechten Maustaste auf den Desktop und wähle "Verknüpfung hier erstellen".

## Controls

- WASD to move the camera target
- Q/E to move the camera up/down
- Middle mouse button to orbit the camera around the scene
- Scroll to zoom the camera in/out
- Left mouse button to spawn a body on the current view plane
- Drag while holding left mouse to set the body's mass
- `^` opens element sample mode; right-drag places the selected element with launch velocity
- Space to pause/continue
- F1 to open a menu where you can enable the octree visualization

## Scientific validation finish line

All deterministic repository changes are gated by analytic gravity references,
physical invariants, direct checks of the adaptive production path, property
tests, production rendering checks, regression orbits, and structural
performance limits in `src/scientific_validation.rs`. The separate GLSL file is
an optional artifact and is compile-validated without being presented as the
active renderer.

Run the complete finish line:

```powershell
python ".\ki_autonomie_testfälle.py"
```

List or run one category:

```powershell
python ".\ki_autonomie_testfälle.py" --list
python ".\ki_autonomie_testfälle.py" --category physics
```

Hardware timing is intentionally opt-in and must be calibrated for the target
machine:

```powershell
python ".\ki_autonomie_testfälle.py" --performance --budget-ms 250
```

The ordinary suite never treats a machine-specific millisecond value as a
universal physics requirement.

The principal acceptance thresholds are:

- exact Octree traversal: relative force error below `2e-5`,
- Barnes-Hut at `theta=0.5`: RMS force error below `5%`,
- adaptive mixed-precision orders: RMS force error below `8%`,
- isolated binary: energy drift below `5e-4` per reference orbit,
- full production-step binary: radial deviation below `1%` and energy drift below `0.5%`,
- body storage at most `192 B` and Octree node growth at most `16 N`.

Gravity uses Plummer softening consistently in runtime force evaluation and
disk-equilibrium circular velocities.
