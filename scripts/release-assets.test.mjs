import assert from "node:assert/strict";
import test from "node:test";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { sha256, verifyAssets, verifyLinuxReadOnlyData, verifyProvenance, versionFromTag, writeAsset } from "./release-assets.mjs";

test("ELF constants may change only in Tauri's known AppImage bundle marker", () => {
  const original = Buffer.from('prefix\0__TAURI_BUNDLE_TYPE_VAR_UNK\0suffix\xff', 'latin1');
  const packaged = Buffer.from('prefix\0__TAURI_BUNDLE_TYPE_VAR_APP\0suffix\xff', 'latin1');
  verifyLinuxReadOnlyData(original, original);
  verifyLinuxReadOnlyData(original, packaged);
  assert.throws(() => verifyLinuxReadOnlyData(original, Buffer.from('changed\0__TAURI_BUNDLE_TYPE_VAR_APP\0suffix\xff', 'latin1')));
  assert.throws(() => verifyLinuxReadOnlyData(original, Buffer.from('prefix\0__TAURI_BUNDLE_TYPE_VAR_DEB\0suffix\xff', 'latin1')));
  assert.throws(() => verifyLinuxReadOnlyData(Buffer.alloc(0), Buffer.alloc(0)));
});

test("only stable version tags are accepted", () => {
  assert.equal(versionFromTag("v0.1.0"), "0.1.0");
  for (const tag of ["main", "v0.1.0/other", "0.1.0", "v0.1.0-rc.1"]) {
    assert.throws(() => versionFromTag(tag));
  }
});

test("validate the entire payload before uploading any file", () => {
  const directory = mkdtempSync(join(tmpdir(), "fuzzy-assets-"));
  try {
    assert.throws(() => verifyAssets(directory), /No assets/);
    writeAsset(join(directory, "a.exe"), Buffer.from("MZ-fixture"));
    assert.equal(verifyAssets(directory).length, 2);
    writeFileSync(join(directory, "z.exe"), "MZ-other");
    assert.throws(() => verifyAssets(directory), /Missing checksum/);
    writeFileSync(join(directory, "z.exe.sha256"), "bad\n");
    assert.throws(() => verifyAssets(directory), /Invalid checksum/);
  } finally {
    rmSync(directory, { recursive: true, force: true });
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
