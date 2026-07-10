const assert = require("node:assert/strict");
const test = require("node:test");

const { assertConserved, assertDigest, cents, runScenario } = require("../helpers/runScenario");

test("pagos validos cierran facturas y no dejan obligaciones", () => {
    const report = runScenario("valid");

    assertDigest(report);
    assertConserved(report);
    assert.equal(report.scenario, "valid");
    assert.equal(report.invoices.count, 2);
    assert.equal(cents(report.invoices.accounting_open), 0);
    assert.equal(cents(report.invoices.presentation_open), 0);
    assert.equal(cents(report.invoices.collected), 3_750_000);
    assert.equal(report.settlement.obligations, 0);
    assert.equal(report.invoice_rows.every((row) => row.status === "Paid"), true);
});

test("importacion de receipts mantiene saldo parcial esperado", () => {
    const report = runScenario("receipts");

    assertDigest(report);
    assertConserved(report);
    assert.equal(report.invoices.count, 2);
    assert.equal(cents(report.invoices.collected), 2_075_000);
    assert.equal(cents(report.invoices.accounting_open), 475_000);
    assert.equal(cents(report.invoices.presentation_open), 475_000);
    assert.equal(report.settlement.claims, 0);
});
