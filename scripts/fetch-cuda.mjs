#!/usr/bin/env node
// Fetches the NVIDIA libraries the Kokoro worker's CUDA path needs on Windows
// (ONNX Runtime 1.28's CUDA 13 build) from NVIDIA's redistributable archives,
// checks their SHA-256, and copies the DLLs next to the built workers:
//
//   cudart64_13.dll                     CUDA runtime 13.1
//   cublas64_13.dll, cublasLt64_13.dll  cuBLAS 13.2
//   cufft64_12.dll                      cuFFT 12.1 (ships with CUDA 13.1)
//   cudnn*64_9.dll                      cuDNN 9.14 for CUDA 13
//
// About 950 MB to download and 1.5 GB unpacked, so nothing runs this
// implicitly. Only the display driver needs to be installed, not the CUDA
// toolkit. Licenses are copied to target/cuda/licenses/.
//
//   npm run fetch:cuda   (or: node scripts/fetch-cuda.mjs [dir ...], default
//                         target/release target/debug)

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, createReadStream, createWriteStream, existsSync, mkdirSync, readdirSync, rmSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
process.chdir(ROOT);

const CUDA = "https://developer.download.nvidia.com/compute/cuda/redist";
const CUDNN = "https://developer.download.nvidia.com/compute/cudnn/redist";
const ARCHIVES = [
  [`${CUDA}/cuda_cudart/windows-x86_64/cuda_cudart-windows-x86_64-13.1.80-archive.zip`, "e6f76dba2be1850a08168e90d868b12368aacb6c7d71871efb019f2d9648e877"],
  [`${CUDA}/libcublas/windows-x86_64/libcublas-windows-x86_64-13.2.1.1-archive.zip`, "b00be5cda504da494a5f6b72a24f53254f97d4d47e8933ecc3993ad58b6b2ad2"],
  [`${CUDA}/libcufft/windows-x86_64/libcufft-windows-x86_64-12.1.0.78-archive.zip`, "b0a3df65c6ce60dc1aa695f47159a26d5172ac7411045abaeea2fbe948a42746"],
  [`${CUDNN}/cudnn/windows-x86_64/cudnn-windows-x86_64-9.14.0.64_cuda13-archive.zip`, "9b98b51bcead704e32640eca1770cadecec1bdf67c6db4093ff4fbcc4a206bd8"],
];
const CACHE = resolve("target/download-cache");
const CUDA_DIR = resolve("target/cuda");

function fail(s) {
  console.error(`error: ${s}`);
  process.exit(1);
}

async function sha256(file) {
  const hash = createHash("sha256");
  await pipeline(createReadStream(file), hash);
  return hash.digest("hex");
}

/** Downloads `url` into the cache once, streamed, and checks its SHA-256. */
async function fetchPinned(url, sha) {
  mkdirSync(CACHE, { recursive: true });
  const file = join(CACHE, url.split("/").pop());
  if (existsSync(file) && (await sha256(file)) === sha) return file;
  console.log(`Downloading ${url}`);
  const res = await fetch(url);
  if (!res.ok) fail(`Could not download ${url} (HTTP ${res.status}).`);
  await pipeline(Readable.fromWeb(res.body), createWriteStream(file));
  const got = await sha256(file);
  if (got !== sha) {
    rmSync(file, { force: true });
    fail(`${file} does not match its pinned checksum (got ${got}).`);
  }
  return file;
}

function* walk(dir) {
  for (const entry of readdirSync(dir)) {
    const p = join(dir, entry);
    if (statSync(p).isDirectory()) yield* walk(p);
    else yield p;
  }
}

if (process.platform !== "win32") fail("The CUDA libraries are fetched on the Windows PC.");

const dests = (process.argv.length > 2 ? process.argv.slice(2) : ["target/release", "target/debug"]).filter((d) => existsSync(d));
if (dests.length === 0) fail("Build the Kokoro worker first; no target directory exists.");

rmSync(CUDA_DIR, { recursive: true, force: true });
mkdirSync(join(CUDA_DIR, "bin"), { recursive: true });
mkdirSync(join(CUDA_DIR, "licenses"), { recursive: true });
for (const [url, sha] of ARCHIVES) {
  const zip = await fetchPinned(url, sha);
  const name = url.split("/").pop().replace(/-archive\.zip$/, "");
  const tmp = join(CACHE, name);
  rmSync(tmp, { recursive: true, force: true });
  mkdirSync(tmp, { recursive: true });
  // Windows 10+ ships bsdtar, which unpacks zip archives. Named in full so
  // Git Bash's GNU tar, which reads `E:` as a remote host, is not used.
  const tar = join(process.env.SystemRoot || "C:\\Windows", "System32", "tar.exe");
  if (spawnSync(tar, ["-xf", zip, "-C", tmp], { stdio: "inherit" }).status !== 0) fail(`Could not unpack ${zip}.`);
  for (const f of walk(tmp)) {
    const base = f.split(/[\\/]/).pop();
    // nvblas and cuFFTW are wrappers ONNX Runtime does not load.
    if (/\.dll$/i.test(base) && /[\\/]bin[\\/]/i.test(f) && !/^(nvblas|cufftw)/i.test(base)) copyFileSync(f, join(CUDA_DIR, "bin", base));
    if (/^LICENSE/i.test(base)) copyFileSync(f, join(CUDA_DIR, "licenses", `${name}-${base}`));
  }
  rmSync(tmp, { recursive: true, force: true });
}

const dlls = readdirSync(join(CUDA_DIR, "bin"));
for (const d of dests) for (const f of dlls) copyFileSync(join(CUDA_DIR, "bin", f), join(d, f));
console.log(`${dlls.length} CUDA libraries copied to ${dests.join(", ")}`);
