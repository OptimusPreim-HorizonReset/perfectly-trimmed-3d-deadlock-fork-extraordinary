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
- Adaptive, dichte-skalierte Gravitationssoftening mit Kernel-basierten SPH-Hydrodynamikkräften
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
- Space to pause/continue
- E to open a menu where you can enable the octree visualization
