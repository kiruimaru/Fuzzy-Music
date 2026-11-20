#!/usr/bin/env bash
set -euo pipefail
packaging=$(cd -- "$(dirname -- "$0")/.." && pwd)
app=$(realpath "${1:?application checkout required}")
tag=${2:?release tag required}
output=$(realpath -m "${3:?output directory required}")
[[ $tag =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo 'Invalid release tag' >&2; exit 1; }
version=${tag#v}
test "$version" = "$(node -p "require(process.argv[1]).version" "$app/src-tauri/tauri.conf.json")"
test -z "$(git -C "$app" status --porcelain --untracked-files=no)"
test "$(git -C "$app" rev-parse HEAD)" = "$(git -C "$app" rev-list -n1 "$tag")"
mkdir -p "$output"
test -z "$(find "$output" -mindepth 1 -maxdepth 1 -print -quit)"
stage=$(mktemp -d)
trap 'rm -rf -- "$stage"' EXIT
install -m755 "$packaging/scripts/linux-runtime-tool.sh" "$stage/mpv"
install -m755 "$packaging/scripts/linux-runtime-tool.sh" "$stage/yt-dlp"
cp "$app/LICENSE" "$stage/LICENSE"
cp "$packaging/docs/linux-appimage.md" "$stage/README.md"
mkdir "$stage/npm-licenses"
while IFS= read -r -d '' file; do
  relative=${file#"$app/node_modules/"}
  mkdir -p "$stage/npm-licenses/$(dirname "$relative")"
  cp "$file" "$stage/npm-licenses/$relative"
done < <(find "$app/node_modules" -type f \( -iname 'license*' -o -iname 'licence*' -o -iname 'copying*' -o -iname 'notice*' \) -print0)
export STAGE="$stage" APP="$app"
node --input-type=module <<'NODE'
import { writeFileSync } from 'node:fs';
const stage = process.env.STAGE;
writeFileSync(`${stage}/appimage.json`, JSON.stringify({ bundle: { active: true, targets: ['appimage'], linux: { appimage: { files: {
  '/usr/bin/tools/mpv': `${stage}/mpv`,
  '/usr/bin/tools/yt-dlp': `${stage}/yt-dlp`,
  '/usr/share/doc/fuzzy-music/LICENSE': `${stage}/LICENSE`,
  '/usr/share/doc/fuzzy-music/README.md': `${stage}/README.md`,
  '/usr/share/doc/fuzzy-music/npm-licenses': `${stage}/npm-licenses`
} } } } }));
NODE
cd "$app"
npm run check:release
APPIMAGE_EXTRACT_AND_RUN=1 npm run tauri -- build --bundles appimage --config "$stage/appimage.json"
images=(target/release/bundle/appimage/*.AppImage)
test "${#images[@]}" = 1
name="Fuzzy-Music-$version-linux-x86_64.AppImage"
install -m755 "${images[0]}" "$output/$name"
cd "$stage"
"$output/$name" --appimage-extract >/dev/null
test -x squashfs-root/usr/bin/tools/mpv
test -x squashfs-root/usr/bin/tools/yt-dlp
# linuxdeploy strips ELF metadata and patches library lookup paths. Verify code
# and read-only application data, then record the actual packaged binary hash.
for section in .text .rodata; do
  objcopy -O binary --only-section="$section" "$app/target/release/fuzzy-shell" "$stage/original$section"
  objcopy -O binary --only-section="$section" squashfs-root/usr/bin/fuzzy-shell "$stage/packaged$section"
  test -s "$stage/original$section"
  if [[ $section == .rodata ]]; then
    node "$packaging/scripts/release-assets.mjs" linux-rodata "$stage/original$section" "$stage/packaged$section"
  else
    cmp "$stage/original$section" "$stage/packaged$section"
  fi
done
./squashfs-root/usr/bin/tools/mpv --version
./squashfs-root/usr/bin/tools/yt-dlp --version

# Preserve exact Ubuntu source-package coordinates and retrieval instructions
# for bundled system libraries, rather than claiming they are application code.
manifest="$stage/LINUX-SYSTEM-SOURCES.txt"
{
  echo 'Build system: Ubuntu 22.04 x86_64 (GitHub Actions)'
  echo 'Bundled library source: enable Ubuntu jammy/jammy-updates/jammy-security deb-src repositories.'
  echo 'Retrieve each exact source with: apt-get source SOURCE=SOURCE_VERSION'
  echo 'Source code, patches and debian build recipes are provided by those source packages.'
  echo 'Installed package / version / source / source-version:'
  dpkg-query -W -f='${binary:Package}\t${Version}\t${source:Package}\t${source:Version}\n'
} > "$manifest"
mkdir "$stage/system-copyright"
# Keep package names: many packages use the same copyright filename.
find /usr/share/doc -mindepth 2 -maxdepth 2 -name copyright -type f -print0 | while IFS= read -r -d '' file; do
  package=$(basename "$(dirname "$file")")
  cp "$file" "$stage/system-copyright/$package.txt"
done
rm -f "$stage/system-copyright/copyright"
export OUTPUT="$output" TAG="$tag" PACKAGING="$packaging" EXTRACTED="$stage/squashfs-root/usr/bin/fuzzy-shell"
node --input-type=module <<'NODE'
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
const git = (dir, ...args) => execFileSync('git', ['-C', dir, ...args], { encoding: 'utf8' }).trim();
const hash = (file) => createHash('sha256').update(readFileSync(file)).digest('hex');
const { APP: app, OUTPUT: output, TAG: tag, PACKAGING: packaging, EXTRACTED: extracted } = process.env;
const name = `Fuzzy-Music-${tag.slice(1)}-linux-x86_64`;
const provenance = { version: tag.slice(1), applicationCommit: git(app, 'rev-parse', 'HEAD'), applicationTree: git(app, 'rev-parse', 'HEAD^{tree}'), packagingCommit: git(packaging, 'rev-parse', 'HEAD'), executableSha256: hash(`${app}/target/release/fuzzy-shell`), packagedExecutableSha256: hash(extracted), appimageSha256: hash(`${output}/${name}.AppImage`), runtimeTools: 'host mpv and yt-dlp; not redistributed', correspondingSource: `Fuzzy-Music-${tag.slice(1)}-corresponding-source.zip`, buildSystem: 'Ubuntu 22.04 x86_64' };
writeFileSync(`${output}/${name}.provenance.json`, `${JSON.stringify(provenance, null, 2)}\n`);
NODE
mkdir "$stage/packaging-source"
git -C "$packaging" archive HEAD scripts docs/linux-appimage.md .github/workflows/platform-assets.yml LICENSE | tar -x -C "$stage/packaging-source"
cp "$manifest" "$stage/packaging-source/"
cp -r "$stage/system-copyright" "$stage/packaging-source/"
cp "$output/"*.provenance.json "$stage/packaging-source/"
tar -czf "$output/Fuzzy-Music-$version-linux-packaging-source.tar.gz" -C "$stage/packaging-source" .
cd "$output"
for file in *; do sha256sum "$file" > "$file.sha256"; done
sha256sum --check -- *.sha256
