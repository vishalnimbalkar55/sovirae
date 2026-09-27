#!/usr/bin/env node
// Stages what the Windows app needs next to Sovirae.exe in
// target/windows-bundle/, which tauri.windows.conf.json bundles:
//
//   sovirae-kokoro-worker.exe, sovirae-pocket-worker.exe   inference workers
//   espeak-ng/   eSpeak NG (GPL-3.0, a separate program) for Kokoro
//                pronunciation, unpacked from the official MSI
//
// Tauri runs this before `tauri dev` and `tauri build` on Windows, and
// before a cross-build from a Mac (which needs `brew install msitools`).

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
process.chdir(ROOT);

const ESPEAK = {
  version: "1.52.0",
  msi: "https://github.com/espeak-ng/espeak-ng/releases/download/1.52.0/espeak-ng.msi",
  msiSha256: "7f673c709ea5dd579d3b5ebb98688cc575328a6ab7438d2bc405b88cedaeafb9",
  license: "https://raw.githubusercontent.com/espeak-ng/espeak-ng/1.52.0/COPYING",
  licenseSha256: "8ceb4b9ee5adedde47b31e975c1d90c73ad27b6b165a1dcd80c7c545eb65b903",
};
const WORKERS = ["sovirae-kokoro-worker", "sovirae-pocket-worker"];
const BUNDLE = resolve("target/windows-bundle");
const CACHE = resolve("target/download-cache");
const HOST_WINDOWS = process.platform === "win32";
const TRIPLE = process.env.TAURI_ENV_TARGET_TRIPLE || "x86_64-pc-windows-msvc";

function fail(s) {
  console.error(`error: ${s}`);
  process.exit(1);
}

function run(cmd, args) {
  const r = spawnSync(cmd, args, { stdio: "inherit" });
  if (r.status !== 0) fail(`${cmd} ${args.join(" ")} failed.`);
}

const sha256 = (file) => createHash("sha256").update(readFileSync(file)).digest("hex");

/** Downloads `url` into the cache once and checks its SHA-256. */
async function fetchPinned(url, sha, name) {
  mkdirSync(CACHE, { recursive: true });
  const file = join(CACHE, name);
  if (existsSync(file) && sha256(file) === sha) return file;
  console.log(`Downloading ${url}`);
  const res = await fetch(url);
  if (!res.ok) fail(`Could not download ${url} (HTTP ${res.status}).`);
  writeFileSync(file, Buffer.from(await res.arrayBuffer()));
  const got = sha256(file);
  if (got !== sha) {
    rmSync(file, { force: true });
    fail(`${name} does not match its pinned checksum (got ${got}).`);
  }
  return file;
}

/** The directory under `dir` that holds `file`, searched depth-first. */
function findDirWith(dir, file) {
  if (existsSync(join(dir, file))) return dir;
  for (const entry of readdirSync(dir)) {
    const sub = join(dir, entry);
    if (statSync(sub).isDirectory()) {
      const found = findDirWith(sub, file);
      if (found) return found;
    }
  }
  return null;
}

function buildWorkers() {
  const packages = WORKERS.flatMap((w) => ["-p", w]);
  if (HOST_WINDOWS) {
    run("cargo", ["build", "--release", ...packages]);
    return "target/release";
  }
  run("cargo", ["xwin", "build", "--release", "--target", TRIPLE, ...packages]);
  return `target/${TRIPLE}/release`;
}

async function stageEspeak() {
  const dest = join(BUNDLE, "espeak-ng");
  const stamp = join(dest, "VERSION");
  if (existsSync(stamp) && readFileSync(stamp, "utf8").trim() === ESPEAK.version) return;

  const msi = await fetchPinned(ESPEAK.msi, ESPEAK.msiSha256, `espeak-ng-${ESPEAK.version}.msi`);
  const license = await fetchPinned(ESPEAK.license, ESPEAK.licenseSha256, `espeak-ng-${ESPEAK.version}-COPYING`);

  // Unpack without installing: an administrative install only extracts files.
  const tmp = join(CACHE, "espeak-ng-unpacked");
  rmSync(tmp, { recursive: true, force: true });
  mkdirSync(tmp, { recursive: true });
  if (HOST_WINDOWS) {
    run("msiexec", ["/a", msi, "/qn", `TARGETDIR=${tmp}`]);
  } else {
    run("msiextract", ["-C", tmp, msi]);
  }
  const unpacked = findDirWith(tmp, "espeak-ng.exe");
  if (!unpacked || !existsSync(join(unpacked, "espeak-ng-data"))) fail("The eSpeak NG MSI did not contain espeak-ng.exe and espeak-ng-data.");

  rmSync(dest, { recursive: true, force: true });
  mkdirSync(dest, { recursive: true });
  for (const f of ["espeak-ng.exe", "libespeak-ng.dll"]) copyFileSync(join(unpacked, f), join(dest, f));
  cpSync(join(unpacked, "espeak-ng-data"), join(dest, "espeak-ng-data"), { recursive: true });
  copyFileSync(license, join(dest, "COPYING"));
  writeFileSync(stamp, `${ESPEAK.version}\n`);
  rmSync(tmp, { recursive: true, force: true });
}

mkdirSync(BUNDLE, { recursive: true });
const out = buildWorkers();
for (const w of WORKERS) copyFileSync(join(out, `${w}.exe`), join(BUNDLE, `${w}.exe`));
await stageEspeak();
console.log(`Windows helpers staged in ${BUNDLE}`);
