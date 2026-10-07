#!/usr/bin/env node
// Reproducible UX-catalog completeness counter + control generator (Researcher, pasada 4/cierre).
// Runs from ANY cwd (paths resolved from this file's location).
//   Usage (from repo root or anywhere):
//     node "Atlas OS/research/ux-catalog/count.mjs"
//     node "C:/Users/Max/Desktop/atlas-os/Atlas OS/research/ux-catalog/count.mjs"
// It (1) counts every "## " ficha header in ux-catalog/*.md by tag, (2) maps the 93
// priority-A entries of the catalog to their ficha state, and (3) writes _pending-A.md.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));            // .../Atlas OS/research/ux-catalog
const CATALOG = path.join(HERE, '..', '64 - UX reference catalog.md');
const CONTROL = path.join(HERE, '_pending-A.md');

const files = fs.readdirSync(HERE).filter(f => f.endsWith('.md') && f !== '_pending-A.md' && f !== '_funcion-x-referente.md').sort();

function tagOf(title) {
  if (/\[P\]/.test(title)) return 'P';
  if (/\[Os parcial\]/i.test(title)) return 'parcial';
  if (/\[Os\]/i.test(title)) return 'Os';
  return 'sin_tag';
}
const rows = [];
for (const file of files) {
  const lines = fs.readFileSync(path.join(HERE, file), 'utf8').split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    const m = lines[i].match(/^#{2,3}\s+(.+)/);
    if (!m) continue;
    const title = m[1].trim();
    if (/^(Cobertura|Enlaces)/i.test(title)) continue;
    let motivo = '';
    for (let j = i + 1; j < Math.min(i + 4, lines.length); j++) if (/Motivo \[P\]|no (abierta|verificad)|HTTP \d|DNS |sin repo|sin doc/i.test(lines[j])) { motivo = lines[j].replace(/\*\*/g, '').replace(/\s+/g, ' ').slice(0, 190); break; }
    rows.push({ file, title, tag: tagOf(title), motivo });
  }
}
const per = {};
for (const r of rows) { per[r.file] = per[r.file] || { Os: 0, parcial: 0, P: 0, sin_tag: 0, total: 0 }; per[r.file][r.tag]++; per[r.file].total++; }
const tot = rows.reduce((a, r) => { a[r.tag]++; return a; }, { Os: 0, parcial: 0, P: 0, sin_tag: 0 });

// ---- map the 93 A entries ----
const cat = fs.readFileSync(CATALOG, 'utf8').split(/\r?\n/);
const aEntries = [];
for (const ln of cat) {
  if (!/^\|\s*\d+\s*\|/.test(ln)) continue;
  const c = ln.split('|').map(s => s.trim());
  if (c[c.length - 2] === 'A') aEntries.push({ name: c[2], capa: c[3] });
}
const norm = s => s.toLowerCase().replace(/\(.*?\)/g, '').replace(/[^a-z0-9 ]+/g, ' ').replace(/\s+/g, ' ').trim();
const LINKED = ['herdr', 'orca', 'genspark', 'zed', 'langfuse', 'vibe kanban', 'n8n', 'vs code', 'visual studio code', 'jetbrains', 'linear', 'hermes', 'conductor', 'tmux', 'zellij', 'grafana', 'notion', 'cursor', 'dify / flowise / n8n', 'qdrant'];
const fichas = rows.map(r => ({ n: norm(r.title), tag: r.tag, title: r.title, file: r.file }));
function findFicha(name) {
  const n = norm(name);
  if (!n) return null;
  let hit = fichas.find(f => f.n.includes(n)) || fichas.find(f => n.includes(f.n) && f.n.length > 5);
  if (hit) return hit;
  const w = n.split(' ').slice(0, 2).join(' ');
  hit = fichas.find(f => w.length > 4 && f.n.includes(w));
  if (hit) return hit;
  const w1 = n.split(' ')[0];
  return w1.length > 4 ? fichas.find(f => f.n.split(' ')[0] === w1) : null;
}
function isLinked(name) { const n = norm(name); return LINKED.some(k => n.includes(k)); }
const aState = aEntries.map(a => {
  if (isLinked(a.name)) return { ...a, state: 'enlazada' };
  const f = findFicha(a.name);
  return { ...a, state: f ? f.tag : 'SIN_FICHA', ficha: f ? f.file : '' };
});
const aCount = aState.reduce((o, a) => { o[a.state] = (o[a.state] || 0) + 1; return o; }, {});

// ---- print ----
console.log('TOTAL fichas:', rows.length, '| [Os]:', tot.Os, '| [Os parcial]:', tot.parcial, '| [P]:', tot.P, '| sin_tag:', tot.sin_tag);
console.log('A entries:', aEntries.length, JSON.stringify(aCount));

// ---- write control ----
const md = [];
md.push('# ux-catalog — control de completitud de prioridad A (cierre, script `count.mjs`)');
md.push('');
md.push('> **Generado por `Atlas OS/research/ux-catalog/count.mjs`** (ejecutable desde cualquier cwd). No editar a mano: re-ejecutar el script.');
md.push('');
md.push('## 0. Mínimo de profundidad para **[Os]**');
md.push('Una ficha es **[Os]** si cumple **las dos**: (1) URL oficial abierta en esta sesión (citada); (2) **≥4 de 7** campos con contenido concreto citado (funcionalidades · layout/navegación · estados/feedback · aprobaciones HITL · atajos · onboarding/vacío/error · accesibilidad). Sin (1) → **[P]** con motivo; con (1) pero <4 → **[Os parcial]**.');
md.push('');
md.push('## 1. Fichas por fichero (conteo del script)');
md.push('');
md.push('| Fichero | total | [Os] | [Os parcial] | [P] | sin_tag |');
md.push('|---|---:|---:|---:|---:|---:|');
for (const f of files) if (per[f]) md.push('| `' + f + '` | ' + per[f].total + ' | ' + per[f].Os + ' | ' + per[f].parcial + ' | ' + per[f].P + ' | ' + per[f].sin_tag + ' |');
md.push('| **TOTAL** | **' + rows.length + '** | **' + tot.Os + '** | **' + tot.parcial + '** | **' + tot.P + '** | **' + tot.sin_tag + '** |');
md.push('');
md.push('## 2. Las 93 entradas de prioridad A y su estado');
md.push('');
md.push('Estado: **enlazada** = cubierta en `docs/design/CONSENSUS_AUDIT.md`; **[Os]** / **[Os parcial]** / **[P]** = ficha en `ux-catalog/`; **SIN_FICHA** = no mapeada.');
md.push('');
md.push('| # | Entrada A | Capa | Estado | Ficha |');
md.push('|---|---|---|---|---|');
aState.forEach((a, i) => md.push('| ' + (i + 1) + ' | ' + a.name.replace(/\|/g, '\\|') + ' | ' + a.capa + ' | ' + a.state + ' | ' + (a.ficha ? '`' + a.ficha + '`' : '') + ' |'));
md.push('');
md.push('**Recuento A por estado:** ' + Object.entries(aCount).map(([k, v]) => k + '=' + v).join(' · ') + ' (total ' + aEntries.length + ').');
md.push('');
md.push('## 3. [P] con motivo (del script)');
md.push('');
for (const r of rows.filter(r => r.tag === 'P')) md.push('- [`' + r.file + '`] ' + r.title.slice(0, 70) + (r.motivo ? ' — ' + r.motivo : ''));
fs.writeFileSync(CONTROL, md.join('\n') + '\n');
console.log('WROTE', CONTROL);
console.log('UNMATCHED A:', aState.filter(a => a.state === 'SIN_FICHA').map(a => a.name).join(' | ') || '(none)');
