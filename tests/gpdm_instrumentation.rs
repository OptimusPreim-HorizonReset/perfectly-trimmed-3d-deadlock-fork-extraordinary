use std::fs;
use std::path::Path;

#[test]
fn ensure_record_before_body_mutations() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_dir = Path::new(manifest_dir).join("src");

    let mut failures: Vec<String> = Vec::new();

    let mut rs_files: Vec<_> = fs::read_dir(&src_dir)
        .expect("reading src dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("rs"))
        .collect();

    rs_files.sort();

    for path in rs_files {
        let content = fs::read_to_string(&path).expect(&format!("read {:?}", path));
        let lines: Vec<&str> = content.lines().collect();

        for (i, &line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            // Skip commented lines
            if trimmed.starts_with("//") || trimmed.starts_with("/*") {
                continue;
            }

            // Detect only *vector-level* mutations to Simulation::bodies:
            // - .bodies.push( ... )
            // - .bodies.insert( ... )
            // - .bodies.remove( ... )
            // - .bodies.pop( ... )
            // - .bodies.swap_remove( ... )
            // - index assignment to the vector element: self.bodies[...] = <something>
            //   (we only treat bracket-assignment where ']' is followed by '=' as a mutation)
            // - direct reassignment of the entire vector: simulation.bodies = ...

            let mut is_mutation = false;

            if trimmed.contains(".bodies.push(")
                || trimmed.contains(".bodies.insert(")
                || trimmed.contains(".bodies.remove(")
                || trimmed.contains(".bodies.clear(")
                || trimmed.contains(".bodies.pop(")
                || trimmed.contains(".bodies.swap_remove(")
            {
                is_mutation = true;
            } else if let Some(idx) = trimmed.find("self.bodies[") {
                // check for a closing bracket followed by '=' (allow whitespace)
                if let Some(close_bracket) = trimmed[idx..].find(']') {
                    let after = &trimmed[idx + close_bracket + 1..];
                    if after.trim_start().starts_with('=') {
                        is_mutation = true;
                    }
                }
            } else if let Some(pos) = trimmed.find("simulation.bodies") {
                let after = &trimmed[pos + "simulation.bodies".len()..];
                if after.trim_start().starts_with('=') {
                    is_mutation = true;
                }
            }

            if !is_mutation {
                // additionally detect in-place changes to important body fields such as
                // mass or segment_type when operating on Simulation::bodies or a
                // mutable reference derived from it (e.g. `for body in &mut bodies`).
                // Only treat assignments to mass/segment_type as mutations (not reads such as `= ... .mass`).
                let mut is_field_assignment = false;
                if (trimmed.contains(".mass") || trimmed.contains(".segment_type")) && (
                    trimmed.contains("self.bodies")
                    || trimmed.contains("simulation.bodies")
                    || trimmed.contains(".bodies[")
                    || trimmed.contains(".bodies.")
                ) {
                    // Look for an assignment operator after the field token (e.g. `.mass =`, `.mass +=`, `.segment_type =`).
                    if let Some(pos) = trimmed.find(".mass") {
                        let after = &trimmed[pos + ".mass".len()..];
                        if after.contains('=') {
                            is_field_assignment = true;
                        }
                    }
                    if !is_field_assignment {
                        if let Some(pos) = trimmed.find(".segment_type") {
                            let after = &trimmed[pos + ".segment_type".len()..];
                            if after.contains('=') {
                                is_field_assignment = true;
                            }
                        }
                    }
                }

                if is_field_assignment {
                    is_mutation = true;
                } else if (trimmed.contains("body.mass") || trimmed.contains("body.segment_type")) {
                    // If this file binds a mutable body from the bodies Vec earlier
                    // (e.g. `for body in &mut bodies` or `let body = &mut self.bodies[idx]`),
                    // we treat subsequent `body.mass = ...` or `body.segment_type = ...`
                    // as mutations to Simulation::bodies. Look back a few lines for such bindings.
                    let bind_lookback = 12usize;
                    let bstart = if i >= bind_lookback { i - bind_lookback } else { 0 };
                    for k in (bstart..i).rev() {
                        let bl = lines[k].trim();
                        if (bl.contains("for ") && (bl.contains("in &mut self.bodies") || bl.contains("in &mut simulation.bodies") || bl.contains("in &mut sim.bodies"))) {
                            is_mutation = true;
                            break;
                        }
                        if (bl.contains("let ") && bl.contains("= &mut") && (bl.contains("self.bodies[") || bl.contains("simulation.bodies["))) {
                            is_mutation = true;
                            break;
                        }
                    }
                }

                if !is_mutation {
                    continue;
                }
            }

            // Look backwards up to N lines for instrumentation calls.
            // Accept either direct `record_gpdm_event(...)` or the helper
            // `record_body_state_change(...)` which itself records GPDM events.
            let lookback = 40usize;
            let start = if i >= lookback { i - lookback } else { 0 };
            let mut found = false;
            for j in (start..i).rev() {
                let ll = lines[j].trim();
                if ll.contains("record_gpdm_event") || ll.contains("record_body_state_change") {
                    found = true;
                    break;
                }
                // also allow comments or blank lines in between; continue searching
            }

            if !found {
                failures.push(format!(
                    "Missing GPDM instrumentation (record_gpdm_event or record_body_state_change) before mutation at {}:{} --> {}",
                    path.display(),
                    i + 1,
                    trimmed
                ));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "GPDM instrumentation invariant violated:\n{}",
            failures.join("\n")
        );
    }
}
