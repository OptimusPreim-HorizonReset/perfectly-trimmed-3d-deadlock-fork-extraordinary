use std::fs;
use std::path::Path;

fn visit_dir(dir: &Path, cb: &mut dyn FnMut(&Path)) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                visit_dir(&p, cb);
            } else {
                cb(&p);
            }
        }
    }
}

#[test]
fn gpdm_instrumentation_invariant() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = repo_root.join("src");
    assert!(src.exists(), "src directory not found");

    let mut issues = Vec::new();

    visit_dir(&src, &mut |p| {
        if let Some(ext) = p.extension() {
            if ext != "rs" { return; }
        } else { return; }

        let content = match fs::read_to_string(p) {
            Ok(s) => s,
            Err(_) => return,
        };
        let lines: Vec<&str> = content.lines().collect();

        // precompute impl Simulation ranges (naive)
        let mut impl_ranges = Vec::new();
        for (idx, line) in lines.iter().enumerate() {
            if line.contains("impl") && line.contains("Simulation") && line.contains('{') {
                let mut brace = 0isize;
                let mut end_idx = lines.len() - 1;
                for j in idx..lines.len() {
                    brace += lines[j].matches('{').count() as isize;
                    brace -= lines[j].matches('}').count() as isize;
                    if j > idx && brace <= 0 {
                        end_idx = j;
                        break;
                    }
                }
                impl_ranges.push((idx, end_idx));
            }
        }

        for (i, raw) in lines.iter().enumerate() {
            let line = raw.trim();
            if line.is_empty() { continue; }
            if line.starts_with("//") || line.starts_with("/*") { continue; }

            let mut is_mutation = false;
            let patterns = [".bodies.push(", ".bodies.insert(", ".bodies.remove(", ".bodies.clear(", ".bodies.pop(", ".bodies.swap_remove("];
            for ptn in patterns.iter() {
                if line.contains(ptn) { is_mutation = true; break; }
            }

            if !is_mutation {
                if line.contains("self.bodies[") {
                    if let Some(idx) = line.find("self.bodies[") {
                        if let Some(rb) = line[idx..].find(']') {
                            let after = &line[idx + rb + 1..];
                            if after.trim_start().starts_with('=') { is_mutation = true; }
                        }
                    }
                }
                if !is_mutation && line.contains("simulation.bodies") {
                    if let Some(pos) = line.find("simulation.bodies") {
                        let after = &line[pos + "simulation.bodies".len()..];
                        if after.trim_start().starts_with('=') { is_mutation = true; }
                    }
                }
            }

            if !is_mutation {
                if (line.contains(".mass") || line.contains(".segment_type")) && (line.contains("self.bodies") || line.contains("simulation.bodies") || line.contains(".bodies[") || line.contains(".bodies.")) {
                    if line.contains(".mass") {
                        if let Some(pos) = line.find(".mass") {
                            if line[pos..].contains('=') { is_mutation = true; }
                        }
                    }
                    if !is_mutation && line.contains(".segment_type") {
                        if let Some(pos) = line.find(".segment_type") {
                            if line[pos..].contains('=') { is_mutation = true; }
                        }
                    }
                } else if line.contains("body.mass") || line.contains("body.segment_type") {
                    let bind_lookback = 12usize;
                    let start = if i > bind_lookback { i - bind_lookback } else { 0 };
                    let mut found_bind = false;
                    for k in start..i {
                        let bl = lines[k].trim();
                        if bl.contains("for ") && (bl.contains("in &mut self.bodies") || bl.contains("in &mut simulation.bodies") || bl.contains("in &mut sim.bodies")) {
                            found_bind = true; break;
                        }
                        if bl.contains("let ") && bl.contains("= &mut") && (bl.contains("self.bodies[") || bl.contains("simulation.bodies[")) {
                            found_bind = true; break;
                        }
                    }
                    if found_bind && line.contains('=') { is_mutation = true; }
                }
            }

            if !is_mutation { continue; }

            let lookback = 40usize;
            let start_check = if i > lookback { i - lookback } else { 0 };
            let mut found_instr = false;
            for j in start_check..i {
                let ll = lines[j].trim();
                if ll.contains("record_gpdm_event") || ll.contains("record_body_state_change") { found_instr = true; break; }
            }
            if !found_instr {
                let mut in_sim_impl = false;
                for &(s, e) in impl_ranges.iter() {
                    if s <= i && i <= e { in_sim_impl = true; break; }
                }
                issues.push(format!("{}:{}: {} (in_impl_simulation={})", p.display(), i+1, line, in_sim_impl));
            }
        }
    });

    if !issues.is_empty() {
        panic!("GPDM instrumentation invariant violated:\n{}", issues.join("\n"));
    }
}
