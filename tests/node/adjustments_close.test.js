const assert = require("node:assert/strict");
const test = require("node:test");

const { assertConserved, assertDigest, cents, runScenario } = require("../helpers/runScenario");

test("ajustes aprobados se reflejan en cierre contable", () => {
    const report = runScenario("adjustments");

    assertDigest(report);
    assertConserved(report);
    assert.equal(report.invoices.count, 1);
    assert.equal(cents(report.invoices.accounting_open), 0);
    assert.equal(cents(report.invoices.memos), 250_000);
    assert.equal(report.invoices.statuses.Paid, 1);
    assert.equal(report.settlement.claims, 0);
});

test("cierre mensual conserva trial balance y estados de factura", () => {
    const report = runScenario("monthly-close");

    assertDigest(report);
    assertConserved(report);
    assert.equal(report.close.status, "Closed");
    assert.equal(report.close.period, "2026-01");
    assert.equal(report.invoices.statuses.Closed, 2);
    assert.equal(report.invoices.statuses.PartiallyCollected, 1);
    assert.equal(cents(report.invoices.accounting_open), 560_000);
    assert.equal(cents(report.invoices.presentation_open), 605_000);
});
