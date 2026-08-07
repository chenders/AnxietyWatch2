import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const indexPath = path.join(__dirname, 'index.html');

const html = fs.readFileSync(indexPath, 'utf8');

function extractBlock(regexps) {
  const code = regexps.map(re => {
    const m = html.match(re);
    if (!m) throw new Error(`Regex not found: ${re}`);
    return m[0];
  }).join('\n');
  return new Function(`
    ${code}
    return { validateRestore, restoreRowModel, completedNightBounds, nightlyDayKeys };
  `)();
}

const fns = extractBlock([
  /function validateRestore\([\s\S]*?\n\}/,
  /function restoreRowModel\([\s\S]*?\n\}/,
  /function completedNightBounds\([\s\S]*?\n\}/,
  /function nightlyDayKeys\([\s\S]*?\n\}/
]);

const { validateRestore, restoreRowModel, completedNightBounds, nightlyDayKeys } = fns;

let failed = false;

function assertResult(name, rv, rvAge, expectOk, expectErrContains) {
  let caseFailed = false;
  const result = validateRestore(rv, rvAge);
  if (result.ok !== expectOk) {
    console.error(`FAIL: ${name} - expected ok=${expectOk}, got ${result.ok}`);
    caseFailed = true;
  }
  if (expectErrContains && (!result.err || !result.err.includes(expectErrContains))) {
    console.error(`FAIL: ${name} - expected err to include "${expectErrContains}", got "${result.err}"`);
    caseFailed = true;
  }
  if (!caseFailed) console.log(`PASS: validateRestore - ${name}`);
  else failed = true;
}

const VALID_RV = { ok: true, tables_checked: 5, checks: ["recovery_lsn", "pg_amcheck"] };

assertResult("valid boundary (age 0)", VALID_RV, 0, true);
assertResult("valid boundary (age exactly 45 days)", VALID_RV, 45 * 86400, true);
assertResult("negative age", VALID_RV, -1, false, "freshness is invalid");
assertResult("future age / NaN", VALID_RV, NaN, false, "freshness is invalid");
assertResult("stale age", VALID_RV, 45 * 86400 + 1, false, "freshness is invalid");
assertResult("fractional tables", { ...VALID_RV, tables_checked: 5.5 }, 0, false, "tables_checked is invalid");
assertResult("zero tables", { ...VALID_RV, tables_checked: 0 }, 0, false, "tables_checked is invalid");
assertResult("string tables", { ...VALID_RV, tables_checked: "5" }, 0, false, "tables_checked is invalid");
assertResult("failed evidence", { ...VALID_RV, ok: false }, 0, false, "evidence is not ok");
assertResult("missing evidence", null, 0, false, "no verified restore");
assertResult("string checks (not array)", { ...VALID_RV, checks: "recovery_lsn, pg_amcheck" }, 0, false, "missing required checks");
assertResult("missing pg_amcheck", { ...VALID_RV, checks: ["recovery_lsn", "something_else"] }, 0, false, "missing required checks");
assertResult("substring behavior test", { ...VALID_RV, checks: ["recovery_lsn_foo", "pg_amcheck_bar"] }, 0, false, "missing required checks");

function assertRow(name, rv, rvAge, expectOk) {
  let caseFailed = false;
  const rowModel = restoreRowModel(rv, rvAge);

  if (expectOk) {
    if (rowModel.error) {
      console.error(`FAIL: restoreRowModel - ${name} - expected no error, got: ${rowModel.error}`);
      caseFailed = true;
    }
    if (rowModel.value !== rvAge) {
      console.error(`FAIL: restoreRowModel - ${name} - expected value=${rvAge}, got: ${rowModel.value}`);
      caseFailed = true;
    }
    if (!rowModel.ok) {
       console.error(`FAIL: restoreRowModel - ${name} - expected ok=true, got ok=false`);
       caseFailed = true;
    }
  } else {
    if (!rowModel.error) {
      console.error(`FAIL: restoreRowModel - ${name} - expected error, got none`);
      caseFailed = true;
    }
    if (rowModel.value !== null) {
      console.error(`FAIL: restoreRowModel - ${name} - expected value=null, got: ${rowModel.value}`);
      caseFailed = true;
    }
    if (rowModel.ok) {
       console.error(`FAIL: restoreRowModel - ${name} - expected ok=false, got ok=true`);
       caseFailed = true;
    }
  }

  if (!caseFailed) console.log(`PASS: restoreRowModel - ${name}`);
  else failed = true;
}

assertRow("valid row", VALID_RV, 10, true);
assertRow("stale row", VALID_RV, 45 * 86400 + 1, false);
assertRow("missing row", null, null, false);
assertRow("invalid tables row", { ...VALID_RV, tables_checked: 0 }, 10, false);

function assertBounds(name, nowStr, expStart, expEnd) {
    let caseFailed = false;
    const nowMs = new Date(nowStr).getTime();
    const bounds = completedNightBounds(nowMs);
    const actStart = bounds.nightStart.toISOString();
    const actEnd = bounds.nightEnd.toISOString();

    if (actStart !== expStart) {
        console.error(`FAIL: completedNightBounds - ${name} - expected start ${expStart}, got ${actStart}`);
        caseFailed = true;
    }
    if (actEnd !== expEnd) {
        console.error(`FAIL: completedNightBounds - ${name} - expected end ${expEnd}, got ${actEnd}`);
        caseFailed = true;
    }
    if (!caseFailed) console.log(`PASS: completedNightBounds - ${name}`);
    else failed = true;
}

assertBounds("before noon UTC", "2026-08-05T10:00:00Z", "2026-08-03T12:00:00.000Z", "2026-08-04T12:00:00.000Z");
assertBounds("after noon UTC", "2026-08-05T14:00:00Z", "2026-08-04T12:00:00.000Z", "2026-08-05T12:00:00.000Z");

function assertKeys(name, days, nightStartStr, expKeys) {
    let caseFailed = false;
    const ns = new Date(nightStartStr);
    const keys = nightlyDayKeys(days, ns);

    if (JSON.stringify(keys) !== JSON.stringify(expKeys)) {
        console.error(`FAIL: nightlyDayKeys - ${name} - expected ${JSON.stringify(expKeys)}, got ${JSON.stringify(keys)}`);
        caseFailed = true;
    }
    if (!caseFailed) console.log(`PASS: nightlyDayKeys - ${name}`);
    else failed = true;
}

assertKeys("7 days window", 7, "2026-08-04T12:00:00Z", [
    "2026-07-29",
    "2026-07-30",
    "2026-07-31",
    "2026-08-01",
    "2026-08-02",
    "2026-08-03",
    "2026-08-04"
]);

if (failed) {
  process.exit(1);
} else {
  console.log("All restore and date helper checks passed.");
}
