import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

export function versionFromTag(tag) {
  if (!/^v\d+\.\d+\.\d+$/.test(tag)) throw new Error("Expected a stable vX.Y.Z tag");
  return tag.slice(1);
}

export function verifyProvenance(provenance, version, commit, executable) {
  if (provenance.version !== version || provenance.commit !== commit || provenance.dirty !== false) {
    throw new Error("Release source provenance does not match the clean tagged commit");
  }
  if (sha256(executable) !== provenance.executableSha256) throw new Error("Executable digest mismatch");
}

export function writeAsset(path, bytes) {
  writeFileSync(path, bytes);
  writeFileSync(`${path}.sha256`, `${sha256(bytes)}  ${basename(path)}\n`);
}

function gh(args) {
  return execFileSync("gh", args, { maxBuffer: 512 * 1024 * 1024 });
}

function windows(tag, input, output, commit) {
  const version = versionFromTag(tag);
  const name = `Fuzzy-Music-${version}-windows-x64`;
  const zip = join(input, `${name}.zip`);
  const expected = readFileSync(`${zip}.sha256`, "utf8").trim().split(/\s+/)[0];
  if (sha256(readFileSync(zip)) !== expected) throw new Error("Portable ZIP digest mismatch");
  const entry = (name) => execFileSync("unzip", ["-p", zip, name], { maxBuffer: 64 * 1024 * 1024 });
  const executable = entry("Fuzzy Music.exe");
  const provenance = JSON.parse(entry("SOURCE-PROVENANCE.json").toString().replace(/^\uFEFF/, ""));
  verifyProvenance(provenance, version, commit, executable);
  if (executable.subarray(0, 2).toString() !== "MZ") throw new Error("Not a Windows executable");
  mkdirSync(output, { recursive: true });
  writeAsset(join(output, `${name}.exe`), executable);
  writeAsset(join(output, "setup-windows-runtime.ps1"), entry("setup-windows-runtime.ps1"));
  writeAsset(join(output, `${name}.provenance.json`), `${JSON.stringify(provenance, null, 2)}\n`);
}

export function verifyAssets(directory) {
  const files = readdirSync(directory);
  if (!files.length) throw new Error("No assets to publish");
  for (const name of files) {
    const local = readFileSync(join(directory, name));
    if (name.endsWith(".sha256")) {
      const target = name.slice(0, -7);
      if (local.toString() !== `${sha256(readFileSync(join(directory, target)))}  ${target}\n`) {
        throw new Error(`Invalid checksum: ${name}`);
      }
    } else if (!files.includes(`${name}.sha256`)) {
      throw new Error(`Missing checksum: ${name}`);
    }
  }
  return files;
}

function upload(tag, directory) {
  versionFromTag(tag);
  const files = verifyAssets(directory);
  const repo = process.env.GITHUB_REPOSITORY;
  if (!repo) throw new Error("GITHUB_REPOSITORY is required");
  const release = JSON.parse(gh(["api", `repos/${repo}/releases/tags/${tag}`]));
  if (release.immutable || release.draft) throw new Error("Release must be published and mutable");
  const sourceName = `Fuzzy-Music-${versionFromTag(tag)}-corresponding-source.zip`;
  if (!release.assets.some((asset) => asset.name === sourceName)) throw new Error("Matching application source archive is missing");
  for (const name of files) {
    const local = readFileSync(join(directory, name));
    const existing = release.assets.find((asset) => asset.name === name);
    if (existing) {
      const published = gh(["release", "download", tag, "--repo", repo, "--pattern", name, "--output", "-"]);
      if (sha256(published) !== sha256(local)) throw new Error(`Refusing to replace published asset: ${name}`);
      console.log(`Already published, identical: ${name}`);
    } else {
      gh(["release", "upload", tag, join(directory, name), "--repo", repo]);
      console.log(`Published: ${name}`);
    }
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [mode, ...args] = process.argv.slice(2);
  if (mode === "windows") windows(...args);
  else if (mode === "upload") upload(...args);
  else throw new Error("Expected windows or upload mode");
}
