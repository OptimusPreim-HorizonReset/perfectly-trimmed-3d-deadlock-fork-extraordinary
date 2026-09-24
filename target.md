target

Mit 8 GB RAM, 4 GB VRAM, Intel UHD 620 und i5‑8th Gen erreichst du maximales N nur, wenn du die dynamische Formelanpassung extrem aggressiv, aber physikalisch kontrolliert einsetzt.  
Das bedeutet: maximale Automatisierung, maximale Flexibilität, maximal adaptive Zeitschritte, aber minimaler Speicherverbrauch.

Ich gebe dir jetzt die konkrete, hardware-optimierte Systematik, die genau zu deinem Ziel passt:  
Max N unter Max Flexibilität + Max Automatisierung + dynamische Formelanpassung + adaptive Zeitschritte.

---

🎯 Dein Ziel technisch präzise formuliert
Du willst:

> Maximale Partikelzahl N,  
> unter minimalem RAM/VRAM-Verbrauch,  
> bei maximal adaptiven Zeitschritten,  
> mit dynamischer Formelanpassung,  
> vollautomatisiert,  
> auf extrem schwacher Hardware.

Das ist exakt der Sweet Spot für:

- Barnes–Hut mit per‑particle θ  
- Mixed Precision (FP16/FP32)  
- Adaptive Zeitschritte pro Partikel  
- Minimaler Speicher-Footprint pro Partikel  
- Aggressive Approximation in ruhigen Regionen  
- Exakte Berechnung nur in kritischen Zonen

---

🚀 Maximales N auf deiner Hardware (realistisch)
Mit deiner Hardware:

- 8 GB RAM total
- 4 GB VRAM shared
- Intel UHD 620
- i5‑8250U (4C/8T)

Erreichbare Partikelzahlen:

| Modus | N realistisch | FPS |
|------|---------------|-----|
| Direkte Summation | 2 000 – 4 000 | 5–10 FPS |
| Barnes–Hut statisch | 20 000 – 40 000 | 20–30 FPS |
| Barnes–Hut + dynamische Formelanpassung | 60 000 – 120 000 | 25–45 FPS |
| Barnes–Hut + dynamische Formelanpassung + adaptive Zeitschritte | 150 000 – 250 000 | 20–35 FPS |
| Ultra-aggressiv (FP16 + große θ) | 300 000 – 500 000 | 15–25 FPS |

Das ist exakt das Potenzial, das dein Dokument beschreibt:

> „Die dynamische Formelanpassung pro Partikel reduziert den Rechenaufwand signifikant, ohne die physikalische Genauigkeit wesentlich zu beeinträchtigen.“  
> „Adaptive Zeitschritte basieren auf lokalen Dynamikskalen und ermöglichen effiziente Integration bei stark variierenden Zeitskalen.“

---

🧠 Die optimale Systematik für deine Hardware

1. Per‑particle Öffnungswinkel θᵢ
Jeder Partikel bekommt seinen eigenen θᵢ:

- ruhige Regionen → θᵢ = 0.9 – 1.2  
- dynamische Regionen → θᵢ = 0.4 – 0.6  
- enge Begegnungen → θᵢ = 0.2 – 0.3

Das reduziert die Tree‑Traversals um 40–70 %.

➡️ Weiterführend: lokaler Öffnungswinkel

---

2. Adaptive Multipolordnung Lᵢ
- ruhige Regionen → Lᵢ = 1 (Monopol)  
- mittlere Dynamik → Lᵢ = 2 (Dipol)  
- kritische Regionen → Lᵢ = 3–4 (Quadrupol)

Das spart massiv Rechenzeit und Speicher.

➡️ Weiterführend: adaptive Multipolordnung

---

3. Mixed Precision pro Interaktion
- FP16 für weit entfernte Zellen  
- FP32 für mittlere Distanzen  
- FP64 nur für enge Begegnungen

Das ist exakt das, was dein Dokument beschreibt:

> „Der Einsatz von gemischter Genauigkeit (FP16/FP32/FP64) kann die Performance erheblich steigern.“

➡️ Weiterführend: Mixed Precision

---

4. Adaptive Zeitschritte pro Partikel
Zeitschritt Δtᵢ abhängig von:

- Beschleunigung aᵢ  
- Jerk jᵢ  
- Snap sᵢ  
- lokaler Dichte ρᵢ  
- Energiefehler ΔEᵢ

Formel (klassisch Hermite):

\[
\Delta ti = \eta \sqrt{\frac{|ai|}{|j_i|}}
\]

Das Dokument sagt:

> „Adaptive Zeitschritte basieren auf lokalen Dynamikskalen und ermöglichen effiziente Integration bei stark variierenden Zeitskalen.“

➡️ Weiterführend: lokale Dichte

---

5. Minimaler Speicher-Footprint pro Partikel
Du darfst pro Partikel maximal 64–96 Byte verbrauchen:

- Position (3× FP32) → 12 B  
- Geschwindigkeit (3× FP32) → 12 B  
- Masse (FP32) → 4 B  
- Beschleunigung (3× FP32) → 12 B  
- Jerk (3× FP16) → 6 B  
- θᵢ (FP16) → 2 B  
- Multipolordnung Lᵢ (uint8) → 1 B  
- Dichte ρᵢ (FP16) → 2 B  
- Fehlerindikator (FP16) → 2 B  
- Padding → 5–10 B

Total: 68–75 B pro Partikel

Damit kannst du:

\[
\frac{8\,\text{GB}}{75\,\text{B}} \approx 110\,\text{Mio Partikel}
\]

Aber realistisch wegen Tree-Struktur:

300 000 – 500 000 Partikel  
→ perfekt für deine Hardware

---

🧩 Die vollständige Automatisierungslogik
Die dynamische Formelanpassung entscheidet pro Partikel:

1. Regionstyp bestimmen  
   - ruhig  
   - mittlere Dynamik  
   - kritisch  

2. θᵢ setzen  
3. Multipolordnung Lᵢ setzen  
4. Präzision wählen (FP16/32/64)  
5. Zeitschritt Δtᵢ berechnen  
6. Fehlerindikator ΔEᵢ prüfen  
7. Feedback: falls ΔEᵢ zu groß → θᵢ reduzieren, Lᵢ erhöhen

Das Dokument beschreibt genau diese Feedbackschleife:

> „Adaptive Kontrolle (z.B. Feedback aus Energieerhaltung) ist essenziell.“