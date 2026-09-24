import os
import re
import json

root = os.getcwd()
src_dir = os.path.join(root, 'src')
if not os.path.isdir(src_dir):
    print(json.dumps({'error': 'src directory not found', 'root': root}))
    raise SystemExit(1)

patterns = [
    r"\.bodies\.push\(",
    r"\.bodies\.insert\(",
    r"\.bodies\.remove\(",
    r"\.bodies\.clear\(",
    r"\.bodies\.pop\(",
    r"\.bodies\.swap_remove\(",
]

results = []

for dirpath, dirnames, filenames in os.walk(src_dir):
    for fn in filenames:
        if not fn.endswith('.rs'):
            continue
        path = os.path.join(dirpath, fn)
        try:
            with open(path, 'r', encoding='utf-8') as f:
                lines = f.readlines()
        except Exception as e:
            print(f"failed to read {path}: {e}")
            continue

        text = ''.join(lines)

        # find impl Simulation ranges
        impl_ranges = []
        for m in re.finditer(r"impl\s+Simulation\s*{", text):
            start_char = m.start()
            # compute start line index
            start_line = text.count('\n', 0, start_char)
            # now scan forward to find matching brace
            brace = 0
            end_line = start_line
            # start scanning from m.end()
            idx = m.end()
            i = idx
            # naive brace count on chars
            while i < len(text):
                ch = text[i]
                if ch == '{':
                    brace += 1
                elif ch == '}':
                    if brace == 0:
                        end_line = text.count('\n', 0, i)
                        break
                    brace -= 1
                i += 1
            else:
                end_line = len(lines) - 1
            impl_ranges.append((start_line, end_line))

        for i, raw in enumerate(lines):
            line = raw.strip()
            if not line:
                continue
            if line.startswith('//') or line.startswith('/*'):
                continue

            is_mutation = False
            # check patterns
            for p in patterns:
                if re.search(p, line):
                    is_mutation = True
                    break

            if not is_mutation:
                # check self.bodies[index] = ... pattern
                if 'self.bodies[' in line:
                    idx = line.find('self.bodies[')
                    rb = line.find(']', idx)
                    if rb != -1:
                        after = line[rb+1:]
                        if after.strip().startswith('='):
                            is_mutation = True
                # check simulation.bodies =
                if not is_mutation and 'simulation.bodies' in line:
                    pos = line.find('simulation.bodies')
                    after = line[pos+len('simulation.bodies'):]
                    if after.strip().startswith('='):
                        is_mutation = True

            if not is_mutation:
                # field assignments to mass/segment_type
                if ('.mass' in line or '.segment_type' in line) and (
                    'self.bodies' in line or 'simulation.bodies' in line or '.bodies[' in line or '.bodies.' in line
                ):
                    # check assignment char after field token
                    if '.mass' in line:
                        pos = line.find('.mass')
                        if '=' in line[pos+len('.mass'):]:
                            is_mutation = True
                    if not is_mutation and '.segment_type' in line:
                        pos = line.find('.segment_type')
                        if '=' in line[pos+len('.segment_type'):]:
                            is_mutation = True
                elif 'body.mass' in line or 'body.segment_type' in line:
                    # look back for binding to &mut bodies
                    bind_lookback = 12
                    start = max(0, i - bind_lookback)
                    found_bind = False
                    for k in range(start, i):
                        bl = lines[k].strip()
                        if 'for ' in bl and ('in &mut self.bodies' in bl or 'in &mut simulation.bodies' in bl or 'in &mut sim.bodies' in bl):
                            found_bind = True
                            break
                        if 'let ' in bl and '= &mut' in bl and ('self.bodies[' in bl or 'simulation.bodies[' in bl):
                            found_bind = True
                            break
                    if found_bind:
                        # ensure assignment operator present
                        if '=' in line:
                            is_mutation = True

            if not is_mutation:
                continue

            # search back up to 40 lines for instrumentation calls
            lookback = 40
            start_line = max(0, i - lookback)
            found_instr = False
            for j in range(start_line, i):
                ll = lines[j].strip()
                if 'record_gpdm_event' in ll or 'record_body_state_change' in ll:
                    found_instr = True
                    break
            if not found_instr:
                # determine if inside impl Simulation
                in_sim_impl = False
                for (s,e) in impl_ranges:
                    if s <= i <= e:
                        in_sim_impl = True
                        break
                results.append({'file': path, 'line_no': i+1, 'line': line, 'in_impl_simulation': in_sim_impl})

# Print results as json
print(json.dumps({'root': root, 'issues': results}, indent=2))
