import assert from "node:assert/strict";
import test from "node:test";
import { sha256, verifyProvenance, versionFromTag } from "./release-assets.mjs";

test("only stable version tags are accepted", () => {
  assert.equal(versionFromTag("v0.1.0"), "0.1.0");
  for (const tag of ["main", "v0.1.0/other", "0.1.0", "v0.1.0-rc.1"]) {
    assert.throws(() => versionFromTag(tag));
  }
});

test("standalone EXE must be identical to the clean tagged portable build", () => {
  const executable = Buffer.from("MZ-fixture");
  const provenance = { version: "0.1.0", commit: "abc", dirty: false, executableSha256: sha256(executable) };
  verifyProvenance(provenance, "0.1.0", "abc", executable);
  for (const change of [{ dirty: true }, { commit: "def" }, { version: "0.2.0" }, { executableSha256: "bad" }]) {
    assert.throws(() => verifyProvenance({ ...provenance, ...change }, "0.1.0", "abc", executable));
  }
});
