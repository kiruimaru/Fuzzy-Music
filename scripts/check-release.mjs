import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";

export function validateVersions(frontend, tauri, cargo, tag) {
  assert.match(frontend, /^\d+\.\d+\.\d+$/, "Expected a stable release version");
  assert.equal(tauri, frontend, "Tauri and npm versions differ");
  assert.equal(cargo, frontend, "Rust shell and npm versions differ");
  if (tag !== undefined) assert.equal(tag, `v${frontend}`, "Release tag and version differ");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const root = new URL("../", import.meta.url);
  const readJson = path => JSON.parse(readFileSync(new URL(path, root), "utf8"));
  const frontend = readJson("package.json").version;
  const tauri = readJson("src-tauri/tauri.conf.json").version;
  const metadata = JSON.parse(execFileSync("cargo",
    ["metadata", "--no-deps", "--locked", "--format-version", "1"],
    { cwd: fileURLToPath(root), encoding: "utf8" }));
  const cargo = metadata.packages.find(item => item.name === "fuzzy-shell")?.version;
  const tag = process.env.GITHUB_REF_TYPE === "tag" ? process.env.GITHUB_REF_NAME : undefined;
  validateVersions(frontend, tauri, cargo, tag);
  assert.ok(existsSync(new URL(`docs/releases/v${frontend}.md`, root)), "Release notes are missing");
  console.log(`Release versions verified: ${frontend}${tag ? ` (${tag})` : ""}`);
}
