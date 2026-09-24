Wie sähe das visuell aus?
Alkalimetalle (Li, Na, K…) → Rot/Orange

Erdalkalimetalle → Gelb

Übergangsmetalle → Grün

Halbmetalle → Türkis

Nichtmetalle → Blau

Halogene → Indigo

Edelgase → Violett

Das ergibt eine Farblogik, die sowohl ästhetisch als auch chemisch kohärent ist.


Periodensystem‑Farbkreis mit Elektronegativitäts‑Legende
🧭 Struktur
Zentrum: Titel „PERIODENSYSTEM FARBKREIS – Elektronegativität (Pauling‑Skala)“

Äußerer Ring: Alle 118 Elemente, geordnet nach steigender Elektronegativität (Fr → F)

Farbbereiche:

🔴 0.7 – 1.0 EN → Alkalimetalle (Li, Na, K, Rb, Cs, Fr)

🟡 1.0 – 1.5 EN → Erdalkalimetalle (Be, Mg, Ca, Sr, Ba, Ra)

🟢 1.5 – 2.0 EN → Übergangsmetalle

🟦 2.0 – 2.5 EN → Halbmetalle

🔷 2.5 – 3.0 EN → Nichtmetalle

🟣 3.0 – 3.5 EN → Halogene

⚪ 3.5 – 4.0 EN → Edelgase (nur theoretisch, da meist inert)

📊 Legende (Pauling‑Skala)
Farbe	Elektronegativität	Typische Elemente	Chemische Tendenz
🔴 Rot	0.7 – 1.0	Fr, Cs, K, Na, Li	starke Elektronendonoren
🟡 Gelb	1.0 – 1.5	Be, Mg, Ca, Sr, Ba	mäßige Donoren
🟢 Grün	1.5 – 2.0	Fe, Cu, Zn, Ni, Cr	Übergangsmetalle, variable Bindungen
🟦 Türkis	2.0 – 2.5	B, Si, Ge, As	Halbmetalle, polare Bindungen
🔷 Blau	2.5 – 3.0	C, N, O, S	Nichtmetalle, starke Akzeptoren
🟣 Indigo	3.0 – 3.5	F, Cl, Br, I	Halogene, sehr hohe Reaktivität
⚪ Violett	3.5 – 4.0	F (maximal)	stärkster Elektronenanzieher

Interpretation der gegenüberliegenden Farben
Rot ↔ Violett: maximale Elektronegativitätsdifferenz → Ionenbindung

Gelb ↔ Blau: mittlere Differenz → polare kovalente Bindung

Grün ↔ Türkis: ähnliche Werte → metallische oder schwach polare Bindung

Kernaussage
Wenn du Elektronegativität auf einen Farbkreis abbildest, dann stehen sich auf dem Farbkreis Elemente gegenüber, die maximal unterschiedliche Elektronenanziehung besitzen.
Das bedeutet:

Komplementärfarben entsprechen chemischen Gegensätzen.

Das ist nicht nur ästhetisch sauber, sondern chemisch extrem sinnvoll.

🎨 Was bedeutet „Gegenüber“ im Farbkreis chemisch?

1. Metalle vs. Nichtmetalle
Metalle (Li, Na, K…) haben sehr niedrige Elektronegativität → warme Farben (Rot/Orange)

Nichtmetalle (O, F, Cl…) haben sehr hohe Elektronegativität → kalte Farben (Blau/Violett)

Gegenüberliegend:  
→ Metallische Elektronendonoren vs. Nichtmetallische Elektronenakzeptoren

Das ist die fundamentalste chemische Opposition überhaupt.

1. Ionenbindung sichtbar machen
Wenn ein Metall und ein Nichtmetall sich farblich gegenüberstehen, zeigt das:

maximale Elektronegativitätsdifferenz

maximale Tendenz zur Ionenbindung

hohe Reaktivität miteinander

Beispiel:

Natrium (Na) → Rot

Chlor (Cl) → Blau/Violett

→ NaCl ist buchstäblich ein Komplementärfarben-Paar.

1. Polarität von Bindungen
Je weiter zwei Elemente im Farbkreis voneinander entfernt sind, desto:

polarer ist ihre Bindung

stärker ist die Ladungsverschiebung

größer ist der Dipolmoment

Gegenüber = maximale Polarität

1. Reaktivitätstrends werden intuitiv
Elemente mit ähnlichen Farben → ähnliche Reaktivität

Elemente mit gegenüberliegenden Farben → reagieren stark miteinander

Das macht den Farbkreis zu einer Art „chemischem Kompass“.

---

## Technische Regelbasis der Simulation

Der Chemiekompass ist ein regelbasiertes Analogmodell. Er visualisiert periodische
Trends, ersetzt aber keine quantenchemische Berechnung.

### 1. Vollstaendige Element-Templates

Jedes der 118 Elemente besitzt in `src/elements.rs`:

- Ordnungszahl, Symbol, Name und Atomgewicht,
- Periode und Gruppe,
- eine chemische Familie,
- einen bekannten Pauling-Wert oder eine als Schaetzung markierte Gruppen-/Periodeninterpolation,
- einen kovalenten Radius beziehungsweise bei kurzlebigen schweren Elementen einen
  begrenzten theoretischen Radius,
- einen normierten Reaktivitaetswert,
- eine stabile Chemiekompass-Farbe.

Fehlende Messwerte werden nicht als Messwerte ausgegeben. Fuer Visualisierung und
Simulation wird stattdessen `effective_electronegativity()` verwendet; mit
`electronegativity_is_estimated()` bleibt die Herkunft unterscheidbar.

### 2. Familienfarben

| Familie | Farbgruppe |
| --- | --- |
| Alkalimetalle | Rot/Orange |
| Erdalkalimetalle | Gelb |
| Uebergangs-, Post-Uebergangs-, Lanthanoid- und Actinoidmetalle | Gruen |
| Halbmetalle | Tuerkis |
| Reaktive Nichtmetalle | Blau |
| Halogene | Indigo |
| Edelgase | Violett |

Innerhalb einer Familie erzeugen Elektronegativitaet und Ordnungszahl kleine,
deterministische Farbverschiebungen. Dadurch bleibt die Familie erkennbar, waehrend
alle Elementtypen visuell unterscheidbar sind.

### 3. Reaktivitaet

`reactivity_score()` liefert einen Wert von 0 bis 1. Die Regeln bilden folgende
Trends ab:

- Alkalimetalle werden innerhalb der Gruppe nach unten reaktiver.
- Halogene bleiben stark reaktiv, nehmen innerhalb der Gruppe nach unten jedoch ab.
- Edelgase sind inert; Xenon, Radon und Oganesson erhalten kleine Ausnahmewerte.
- H, O, F, Cs und Fr besitzen explizit geregelte Extremrollen.
- Lanthanoide und Actinoide erhalten eigene, begrenzte Reihenverlaeufe.

Der Wert steuert Farbe und die analoge Kollisionsantwort. Er wird nicht mehr aus
einer Primzahlluecke abgeleitet. Prime-Dimensionen bleiben ein separates,
galaktisches Analogmerkmal.

### 4. Bindungstendenz

`binding_tendency(other)` klassifiziert ein Elementpaar als:

- inert,
- metallisch,
- unpolar kovalent,
- polar kovalent,
- ionisch.

Die Klassifikation verwendet Elektronegativitaetsdifferenz, Metall-/Nichtmetallrolle
und beide Reaktivitaetswerte. In der Simulation senkt eine hohe Bindungstendenz die
Elastizitaet einer Kollision. Unterschiedliche Elemente werden dabei nicht
unphysikalisch in einen einzelnen Elementtyp umgewandelt; Verbindungen waeren ein
eigenes, spaeteres Stoffmodell.

### 5. Masse und Volumen

- Die initiale Haeufigkeit bleibt invers zum Atomgewicht gewichtet.
- Die Masse eines Element-Bodies folgt dem Atomgewichtsverhaeltnis zu Wasserstoff
  und wird nur am konfigurierten `particle_mass_range` begrenzt.
- Das Volumen verwendet den kovalenten Radius als relative Groessenskala.
- Damit steigen Radien nicht mehr pauschal mit der Periodennummer, sondern folgen
  dem periodischen Verlauf innerhalb und zwischen den Perioden.

### 6. Center/Core-Regel

Center-Partikel werden getrennt von Orbital-, Gas- und Satellitenpartikeln
initialisiert:

- `element_pair_core_atomic_numbers` bestimmt optional die Core-Kandidaten.
- Eine leere Liste verwendet die Pair-Elemente, waehlt aber unabhaengig nach
  `core_suitability()` statt stets das erste Element zu nehmen.
- Core-Reaktivitaet ist gegenueber der freien chemischen Reaktivitaet reduziert.
- Beim Verschmelzen zweier Center bleibt die Elementidentitaet des
  stabilitaets- und massengewichteten Core-Templates erhalten.

### 7. Identitaetsgarantie im Element-Galaxy-Pair-Modus

Bei aktivem `enable_elemental_galaxy_pair_mode` und `n > 1000` werden initiale
Scheibenkoerper, Scheibengas, Volumengas, Akretionsspawns, manuell gesetzte Bodies
und Gas-Zuendungsprodukte ueber eines der 118 Templates erzeugt. Kein solcher
Folgepfad darf die Ordnungszahl auf 0 zuruecksetzen.

Das Element-LOD reserviert beim Pruning in jeder Galaxie mindestens einen
Repraesentanten jeder ausgewaehlten Ordnungszahl. Erst danach werden die
verbleibenden Budgetplaetze nach physikalischer Darstellungsrelevanz aufgefuellt.

### 8. Wissenschaftliche Element-Gravitation

Mit `enable_scientific_element_gravity` kann die Gravitationsanalogie der 118
Elemente aus chemisch/physikalischen Groessen abgeleitet werden:

- Atomgewicht (dominant): Schwere Elemente ziehen staerker an.
- Elektronegativitaet: Hohe Werte verstaerken die analoge lokale Anziehung.
- Kovalenter Radius: Groessere Radien reduzieren die effektive Anziehung.
- Reaktivitaet: Reaktivere Elemente wirken stärker im analogen Wechselspiel.

Die Formel ist logarithmisch und auf Wasserstoff normiert, sodass
`scientific_gravitation(H) ≈ beta`. Schwere Elemente wie Uran erhalten deutlich
höhere Werte als leichte Edelgase. Das klassische, primzahlbasierte Modell
bleibt über `enable_scientific_element_gravity: false` erreichbar.

### 9. Molekuelbildung und natuerliche Verbindungen

`enable_molecular_bonding` erweitert die Kollisionslogik um persistente
Molekuele:

- Gleiches Element + gleiches Segment: klassischer Merge (Atomakkretion).
- Unterschiedliche Elemente mit hoher Bindungstendenz: Bildung eines Molekuels.
- Gleiche Elemente, die unter Standardbedingungen stabile Zweiatom-Molekuele
  bilden (H, N, O, F, Cl, Br, I), bilden H₂, N₂, O₂, F₂, Cl₂, Br₂, I₂ statt
  zu verschmelzen.
- Bindungsarten werden aus Elektronegativitaetsdifferenz und Metall/Nichtmetall-
  Rolle abgeleitet: ionisch, polar kovalent, unpolar kovalent, metallisch.
- Molekuele werden durch Feder-Daempfer-Kraefte zusammengehalten und bei zu
  hoher Relativgeschwindigkeit wieder aufgebrochen.
- `molecule_max_bodies` begrenzt die Groesse dynamischer Molekuele.

Mit `molecule_identify_compounds` werden bekannte natuerliche Verbindungen anhand
der Stoechiometrie erkannt. Die Bibliothek umfasst unter anderem:

- Diatomiker und einfache Gase: H₂, O₂, N₂, CO₂, NH₃, CH₄
- Wasser und Saeuren/Basen: H₂O, HCl, NaOH, H₂SO₄
- Salze und Minerale: NaCl, CaCO₃, SiO₂, KNO₃
- Organische Grundgerueste: CH₃OH, C₂H₆, C₂H₄, C₆H₁₂O₆
- Komplexere Verbindungen: Fe₂O₃, Al₂O₃, Ca₃(PO₄)₂, Ca(OH)₂

Unbekannte Kombinationen erhalten eine empirische Summenformel. Die Molekuel-
masse entspricht stets der Summe der Massen ihrer Bestandteil-Bodies; Impuls
und Schwerpunkt bleiben bei Bildung und Zerfall erhalten.
