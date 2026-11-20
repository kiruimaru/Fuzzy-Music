import assert from "node:assert/strict";
import test from "node:test";
import { validateVersions } from "./check-release.mjs";

test("aligned stable versions accept branch builds and the matching tag", () => {
  validateVersions("0.1.0", "0.1.0", "0.1.0");
  validateVersions("0.1.0", "0.1.0", "0.1.0", "v0.1.0");
});

test("rejects stale Tauri or Cargo versions", () => {
  assert.throws(() => validateVersions("0.1.0", "0.1.0-alpha.1", "0.1.0"));
  assert.throws(() => validateVersions("0.1.0", "0.1.0", "0.1.0-alpha.1"));
});

test("rejects an unrelated or malformed release tag", () => {
  for (const tag of ["v0.2.0", "0.1.0", "v0.1.0/other", ""]) {
    assert.throws(() => validateVersions("0.1.0", "0.1.0", "0.1.0", tag));
  }
});

test("rejects prerelease versions and a missing shell version", () => {
  assert.throws(() => validateVersions("0.1.0-alpha.1", "0.1.0-alpha.1", "0.1.0-alpha.1"));
  assert.throws(() => validateVersions("0.1.0", "0.1.0", undefined));
});
