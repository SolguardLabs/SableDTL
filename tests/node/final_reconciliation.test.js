const assert = require("node:assert/strict");
const test = require("node:test");

const { assertConserved, assertDigest, cents, runScenario } = require("../helpers/runScenario");

test("reconciliacion final prepara claims y mantiene digest estable", () => {
    const report = runScenario("final");

    assertDigest(report);
    assertConserved(report);
    assert.equal(report.close.status, "Prepared");
    assert.equal(report.settlement.claims, 2);
    assert.equal(report.settlement.posted_claims, 2);
    assert.equal(cents(report.settlement.active_claim_total), 1_620_000);
    assert.equal(report.claim_rows.length, 2);
    assert.deepEqual(
        report.claim_rows.map((row) => cents(row.amount)),
        [1_350_000, 270_000],
    );
});
