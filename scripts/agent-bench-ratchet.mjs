#!/usr/bin/env node
// RFC 63 §10 (item 12) — capability ratchet.
//
// Compares a fresh Harbor/Terminal-Bench report against the committed baseline
// and exits non-zero ONLY on a regression. This is a *ratchet*: there is no
// fixed threshold at the start (baseline pass_rate is 0.000), so the first runs
// cannot fail; the floor only rises as the agent improves.
//
// Usage: node scripts/agent-bench-ratchet.mjs <baseline.json> [report.json]
//   - no report path (or missing file) → prints a notice, exits 0.
//   - report worse than baseline     → exits 1.
//   - report >= baseline             → exits 0.

import { readFileSync, existsSync } from 'node:fs';

const [baselinePath, reportPath] = process.argv.slice(2);

function load(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

/** Find the first numeric `pass_rate` anywhere in a parsed report. */
function findPassRate(node) {
  if (node == null) return null;
  if (typeof node === 'object') {
    if (typeof node.pass_rate === 'number') return node.pass_rate;
    for (const value of Object.values(node)) {
      const found = findPassRate(value);
      if (found !== null) return found;
    }
  }
  return null;
}

if (!baselinePath || !existsSync(baselinePath)) {
  console.error(`ratchet: baseline not found at ${baselinePath ?? '<none>'}`);
  process.exit(2);
}

const baseline = load(baselinePath);
const baselineRate = baseline?.terminal_bench_2?.pass_rate ?? 0;

if (!reportPath || !existsSync(reportPath)) {
  console.info(
    `ratchet: no report supplied — baseline pass_rate=${baselineRate}. ` +
      `Run Terminal-Bench externally (tools/harbor_atlas/RUNBOOK.md) and pass its JSON to compare.`,
  );
  process.exit(0);
}

const report = load(reportPath);
const reportRate = findPassRate(report);

if (reportRate === null) {
  console.error('ratchet: report has no numeric pass_rate; cannot compare.');
  process.exit(2);
}

console.info(`ratchet: baseline=${baselineRate} report=${reportRate}`);

if (reportRate < baselineRate) {
  console.error(
    `ratchet: REGRESSION — pass_rate dropped from ${baselineRate} to ${reportRate}. ` +
      `Bump tools/harbor_atlas/baseline.json only when a run IMPROVES.`,
  );
  process.exit(1);
}

console.info('ratchet: ok (no regression).');
