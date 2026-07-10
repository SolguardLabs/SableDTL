const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const { resolve } = require("node:path");

const repoRoot = resolve(__dirname, "..", "..");

function runScenario(name) {
    const args = ["run", "--quiet", "--", name];
    const stdout =
        process.platform === "win32"
            ? execFileSync("cmd.exe", ["/d", "/c", commandLine("cargo", args)], {
                  cwd: repoRoot,
                  encoding: "utf8",
                  stdio: ["ignore", "pipe", "pipe"],
              })
            : execFileSync("cargo", args, {
                  cwd: repoRoot,
                  encoding: "utf8",
                  stdio: ["ignore", "pipe", "pipe"],
              });
    return JSON.parse(stdout);
}

function commandLine(command, args) {
    return [command, ...args].map(quoteCmdArg).join(" ");
}

function quoteCmdArg(part) {
    const text = String(part);
    if (/^[A-Za-z0-9_.:=-]+$/.test(text)) {
        return text;
    }
    return `"${text.replaceAll('"', '""')}"`;
}

function cents(value) {
    return value.cents;
}

function assertDigest(report) {
    assert.equal(typeof report.digest, "string");
    assert.match(report.digest, /^[0-9a-f]{64}$/);
}

function assertConserved(report) {
    assert.equal(report.conservation_ok, true);
    assert.equal(report.ledger.balanced, true);
    assert.equal(cents(report.ledger.total_debits), cents(report.ledger.total_credits));
}

module.exports = {
    assertConserved,
    assertDigest,
    cents,
    runScenario,
};
