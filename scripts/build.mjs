#!/usr/bin/env node
// Builds Sovirae for every platform this computer can produce and gathers
// the results in build/ (each platform folder is cleared and refilled):
//
//   build/macos/     Sovirae.app, the DMG, and Sovirae_<version>_macos.zip (on a Mac)
//   build/windows/   Sovirae_<version>_x64-setup.exe       installer (on Windows, or
//                    on a Mac after `npm run setup:windows-cross`)
//                    Sovirae_<version>_x64-portable.zip    unzip and run, no install
//                    (on Windows)
//   build/ext/       the Chrome extension, for Load unpacked
//   build/Sovirae_<version>_chrome-extension.zip   the same, zipped for the store
//
// The Windows builds include Kokoro's NVIDIA GPU libraries (~1.3 GB) once
// `npm run fetch:cuda` has run; --no-gpu leaves them out.
//
//   node scripts/build.mjs                 every platform this computer can build, with tests
//   node scripts/build.mjs --skip-tests    build only
//   node scripts/build.mjs --no-install    reuse node_modules instead of `npm ci`
//   node scripts/build.mjs --mac           only macOS  (also: --windows)
//   node scripts/build.mjs --no-gpu        Windows without the NVIDIA GPU libraries
//   node scripts/build.mjs --collect       copy the last builds into build/ without rebuilding

import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, renameSync, rmSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
process.chdir(ROOT);

const args = new Set(process.argv.slice(2));
const HOST = process.platform === "darwin" ? "mac" : process.platform === "win32" ? "windows" : process.platform;
const WIN_TARGET = "x86_64-pc-windows-msvc";
const CROSS_MARKER = join(homedir(), ".sovirae-build", "xwin-license-accepted");
const VERSION = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8")).version;
// prepare-windows.mjs (Tauri's before-build command) reads this.
if (args.has("--no-gpu")) process.env.SOVIRAE_NO_CUDA = "1";

const bold = (s) => (process.stdout.isTTY ? `\x1b[1m${s}\x1b[0m` : s);
const step = (s) => console.log(`\n${bold(`==> ${s}`)}`);
const warn = (s) => console.warn(`${process.stderr.isTTY ? "\x1b[33mwarning:\x1b[0m" : "warning:"} ${s}`);
function fail(s) {
  console.error(`${process.stderr.isTTY ? "\x1b[31merror:\x1b[0m" : "error:"} ${s}`);
  process.exit(1);
}

/** Runs a command with inherited output; exits on failure unless `soft`. */
function run(cmd, cmdArgs, { env, soft = false } = {}) {
  const r = spawnSync(cmd, cmdArgs, {
    stdio: soft ? "pipe" : "inherit",
    env: { ...process.env, ...env },
    // npm/npx are .cmd shims on Windows and need a shell.
    shell: HOST === "windows",
    encoding: "utf8",
  });
  if (soft) return r.status === 0 ? (r.stdout ?? "").trim() : null;
  if (r.status !== 0) fail(`${cmd} ${cmdArgs.join(" ")} failed.`);
  return "";
}
const has = (cmd, cmdArgs = ["--version"]) => run(cmd, cmdArgs, { soft: true }) !== null;

// ---- What to build -----------------------------------------------------------

/** Tools a Mac needs to cross-build the Windows installer, or null when ready. */
function crossMissing() {
  const missing = [];
  if (!existsSync(CROSS_MARKER)) missing.push("Microsoft SDK license not accepted");
  if (!has("cargo", ["xwin", "--version"])) missing.push("cargo-xwin");
  if (!has("makensis", ["-VERSION"])) missing.push("NSIS");
  if (!llvmBin()) missing.push("LLVM");
  if (!(run("rustup", ["target", "list", "--installed"], { soft: true }) ?? "").includes(WIN_TARGET)) {
    missing.push(`Rust target ${WIN_TARGET}`);
  }
  return missing.length ? missing : null;
}

function llvmBin() {
  const prefix = run("brew", ["--prefix", "llvm"], { soft: true });
  const bin = prefix && join(prefix, "bin");
  return bin && existsSync(join(bin, "lld-link")) ? bin : null;
}

function chooseTargets() {
  const asked = ["mac", "windows"].filter((t) => args.has(`--${t}`));
  if (HOST === "mac") {
    if (asked.length === 0) {
      const missing = crossMissing();
      if (missing) {
        warn(`Skipping the Windows installer (missing: ${missing.join(", ")}). Set up once with: npm run setup:windows-cross`);
        return ["mac"];
      }
      return ["mac", "windows"];
    }
    if (asked.includes("windows")) {
      const missing = crossMissing();
      if (missing) fail(`Cross-building for Windows needs: ${missing.join(", ")}. Set up once with: npm run setup:windows-cross`);
    }
    return asked;
  }
  if (HOST === "windows") {
    if (asked.includes("mac")) fail("The macOS app can only be built on a Mac.");
    return ["windows"];
  }
  fail(`Building on ${process.platform} is not supported yet.`);
}

// ---- Steps -------------------------------------------------------------------

function checkPrerequisites(targets) {
  step("Checking prerequisites");
  if (!has("cargo")) fail("Rust is missing. Install it from https://rustup.rs");
  const nodeMajor = Number(process.versions.node.split(".")[0]);
  if (nodeMajor < 22) fail(`Node.js 22 or later is required (found ${process.version}).`);
  if (HOST === "mac") {
    if (!has("xcode-select", ["-p"])) fail("Xcode Command Line Tools are missing. Run: xcode-select --install");
    if (!has("espeak-ng")) warn("espeak-ng is not installed. The app builds, but Kokoro voices need it: brew install espeak-ng");
  }
  if (HOST === "windows") {
    const host = (run("rustc", ["-vV"], { soft: true }) ?? "").match(/host: (\S+)/)?.[1];
    if (host !== WIN_TARGET) {
      fail(`Rust must use the ${WIN_TARGET} toolchain (found ${host ?? "unknown"}). Install Visual Studio Build Tools with "Desktop development with C++", then: rustup default stable-msvc`);
    }
  }
  const rust = run("rustc", ["--version"], { soft: true });
  console.log(`${rust}, node ${process.version}, building: ${targets.join(", ")}`);
}

/**
 * Native modules (`.node`) loaded by a running process, e.g. `npm run app:dev`.
 * Windows cannot delete them, and `npm ci` would stop halfway through
 * emptying node_modules. Renaming a loaded DLL fails the same way, so each is
 * renamed and put back.
 */
function lockedNativeModules() {
  if (HOST !== "windows" || !existsSync("node_modules")) return [];
  const locked = [];
  const scan = (dir, depth) => {
    for (const e of readdirSync(dir, { withFileTypes: true })) {
      const p = join(dir, e.name);
      if (e.isDirectory() && depth < 2 && e.name !== ".bin") scan(p, depth + 1);
      else if (e.isFile() && e.name.endsWith(".node")) {
        try {
          renameSync(p, `${p}.probe`);
          renameSync(`${p}.probe`, p);
        } catch {
          locked.push(p);
        }
      }
    }
  };
  scan("node_modules", 0);
  return locked;
}

function install() {
  step("Installing JavaScript packages");
  const locked = lockedNativeModules();
  if (locked.length) {
    fail(`${locked.join(", ")} is in use, probably by \`npm run app:dev\`. Stop it first, or keep node_modules with: npm run build:all -- --no-install`);
  }
  run("npm", existsSync("package-lock.json") ? ["ci", "--no-audit", "--no-fund"] : ["install", "--no-audit", "--no-fund"]);
}

function test() {
  step("Running tests");
  if (HOST === "windows") {
    // The Chrome native host is ported in a later phase
    // (spec/steps/14-windows-support.md).
    run("cargo", ["test", "--workspace", "--exclude", "sovirae-native-host"]);
    run("node", ["--test", "ext/tests/*.test.js"]);
  } else {
    run("npm", ["test"]);
  }
}

function buildMac() {
  step("Building the macOS app, workers, and DMG");
  run("npx", ["tauri", "build"]);
}

function buildWindows() {
  if (HOST === "windows") {
    step("Building the Windows installer");
    run("npx", ["tauri", "build"]);
    return;
  }
  step("Cross-building the Windows installer (experimental)");
  const sep = process.platform === "win32" ? ";" : ":";
  run("npx", ["tauri", "build", "--runner", "cargo-xwin", "--target", WIN_TARGET, "--config", "src-tauri/tauri.windows.conf.json"], {
    env: { PATH: `${llvmBin()}${sep}${process.env.PATH}`, XWIN_ACCEPT_LICENSE: "1" },
  });
}

// ---- Collect -----------------------------------------------------------------

const BUNDLE = "target/release/bundle";

function collectMac() {
  const app = `${BUNDLE}/macos/Sovirae.app`;
  if (!existsSync(app)) fail(`${app} is missing. Build it first.`);
  for (const f of ["sovirae", "sovirae-kokoro-worker", "sovirae-pocket-worker", "sovirae-native-host"]) {
    if (!existsSync(`${app}/Contents/MacOS/${f}`)) fail(`${f} is missing from the app bundle.`);
  }
  // The Kokoro worker links WebGPU (Dawn) for GPU voices and cannot start without it.
  if (!existsSync(`${app}/Contents/Frameworks/libwebgpu_dawn.dylib`)) fail("libwebgpu_dawn.dylib is missing from the app bundle.");
  if (!existsSync(`${app}/Contents/Resources/ext/manifest.json`)) fail("The Chrome extension is missing from the app bundle.");

  rmSync("build/app", { recursive: true, force: true }); // layout before build/macos existed
  rmSync("build/macos", { recursive: true, force: true });
  mkdirSync("build/macos", { recursive: true });
  // ditto keeps the bundle's symlinks, permissions, and signature intact.
  run("ditto", [app, "build/macos/Sovirae.app"]);
  for (const f of files(`${BUNDLE}/dmg`, ".dmg")) cpSync(f, join("build/macos", basename(f)));
  zip("build/macos/Sovirae.app", `build/macos/Sovirae_${VERSION}_macos.zip`, { parent: true });
}

function collectWindows() {
  // Native builds land in target/release; cross-builds in target/<triple>/release.
  const dirs = [`${BUNDLE}/nsis`, `target/${WIN_TARGET}/release/bundle/nsis`];
  const installers = dirs.flatMap((d) => files(d, "-setup.exe"));
  if (installers.length === 0) fail("No Windows installer was found. Build it first.");
  // Keep only the newest installer per file name when both locations exist.
  const newest = new Map();
  for (const f of installers) {
    const prev = newest.get(basename(f));
    if (!prev || statSync(f).mtimeMs > statSync(prev).mtimeMs) newest.set(basename(f), f);
  }
  rmSync("build/windows", { recursive: true, force: true });
  mkdirSync("build/windows", { recursive: true });
  for (const [name, f] of newest) cpSync(f, join("build/windows", name));

  // The portable zip holds what the installer puts in the install folder:
  // the app, the staged helpers, and the extension. It comes from the same
  // build as the newest installer (…/release/bundle/nsis/<installer>).
  const latest = [...newest.values()].sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs)[0];
  const release = dirname(dirname(dirname(latest)));
  const exe = join(release, "sovirae.exe");
  if (!existsSync(exe)) fail(`${exe} is missing. Build it first.`);
  if (!existsSync("target/windows-bundle/sovirae-kokoro-worker.exe")) fail("target/windows-bundle is missing. Build it first.");
  const dir = "target/portable/Sovirae";
  rmSync("target/portable", { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  cpSync(exe, join(dir, "Sovirae.exe"));
  cpSync("target/windows-bundle", dir, { recursive: true });
  cpSync("build/ext", join(dir, "ext"), { recursive: true });
  zip(dir, `build/windows/Sovirae_${VERSION}_x64-portable.zip`, { parent: true });
  rmSync("target/portable", { recursive: true, force: true });
}

function collectExtension() {
  rmSync("build/ext", { recursive: true, force: true });
  const skip = new Set(["tests", "package.json", "node_modules", ".DS_Store"]);
  cpSync("ext", "build/ext", { recursive: true, filter: (src) => !skip.has(basename(src)) });
  if (!existsSync("build/ext/manifest.json")) fail("The Chrome extension was not copied.");
  // For the Chrome Web Store: manifest.json at the root of the zip.
  zip("build/ext", `build/Sovirae_${VERSION}_chrome-extension.zip`, { parent: false });
}

/**
 * Zips `dir` into `out`: as a folder when `parent`, else its contents at the
 * zip root. Uses Windows' bundled bsdtar (named in full, since Git Bash's GNU
 * tar cannot write zips) or macOS's ditto.
 */
function zip(dir, out, { parent }) {
  rmSync(out, { force: true });
  let r;
  if (HOST === "windows") {
    const tar = join(process.env.SystemRoot || "C:\\Windows", "System32", "tar.exe");
    const [cwd, entries] = parent ? [dirname(dir), [basename(dir)]] : [dir, readdirSync(dir)];
    r = spawnSync(tar, ["-a", "-c", "-f", resolve(out), ...entries], { cwd, stdio: "inherit" });
  } else {
    r = spawnSync("ditto", ["-c", "-k", "--sequesterRsrc", ...(parent ? ["--keepParent"] : []), dir, out], { stdio: "inherit" });
  }
  if (r.status !== 0 || !existsSync(out)) fail(`Could not create ${out}.`);
}

function files(dir, suffix) {
  return existsSync(dir) ? readdirSync(dir).filter((f) => f.endsWith(suffix)).map((f) => join(dir, f)) : [];
}
function basename(p) {
  return p.split(/[\\/]/).pop();
}

function summary() {
  step("Done");
  const size = (f) => `${(statSync(f).size / 1048576).toFixed(1)} MB`;
  for (const dir of ["build/macos", "build/windows"]) {
    if (!existsSync(dir)) continue;
    for (const f of readdirSync(dir)) console.log(`${join(dir, f).padEnd(48)} ${f.endsWith(".app") ? "" : size(join(dir, f))}`);
  }
  if (existsSync("build/ext")) console.log(`${"build/ext/".padEnd(48)} Load unpacked in chrome://extensions`);
  for (const f of files("build", ".zip")) console.log(`${f.padEnd(48)} ${size(f)}`);
}

// ---- Main --------------------------------------------------------------------

if (args.has("--collect")) {
  step("Collecting the last builds into build/");
  collectExtension(); // first: the portable Windows zip includes it
  let any = false;
  if (existsSync(`${BUNDLE}/macos/Sovirae.app`)) (collectMac(), (any = true));
  if ([`${BUNDLE}/nsis`, `target/${WIN_TARGET}/release/bundle/nsis`].some((d) => files(d, "-setup.exe").length)) {
    collectWindows();
    any = true;
  }
  if (!any) fail("Nothing has been built yet. Run npm run build:all first.");
  summary();
} else {
  const targets = chooseTargets();
  checkPrerequisites(targets);
  if (!args.has("--no-install")) install();
  if (!args.has("--skip-tests")) test();
  if (targets.includes("mac")) buildMac();
  if (targets.includes("windows")) buildWindows();
  step("Collecting outputs into build/");
  collectExtension(); // first: the portable Windows zip includes it
  if (targets.includes("mac")) collectMac();
  if (targets.includes("windows")) collectWindows();
  summary();
}
