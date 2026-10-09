#!/usr/bin/env node
// Sets the app version in the three files that must agree (plan §13.1) and syncs
// Cargo.lock. Usage: pnpm version:set 1.0.0
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";

/** The files that carry the version, relative to the repo root. */
export const FILES = ["package.json", "src-tauri/Cargo.toml", "src-tauri/tauri.conf.json"];

const VERSION = /^\d+\.\d+\.\d+$/;

/**
 * Returns each file's contents with its version line changed, and only that line.
 * @param {Record<string, string>} files contents by file name
 * @param {string} version the new version, like 1.0.0
 */
export function setVersion(files, version) {
  if (!VERSION.test(version)) throw new Error(`Use a version like 1.0.0, not "${version}"`);
  const out = {};
  for (const [name, text] of Object.entries(files)) {
    // Cargo.toml's [package] version starts its line; dependency versions don't. The JSON
    // files have one top-level "version", and it comes before any nested one.
    const toml = name.endsWith("Cargo.toml");
    const line = toml ? /^version = "[^"]*"/m : /^(\s*)"version": "[^"]*"/m;
    if (!line.test(text)) throw new Error(`No version line found in ${name}`);
    out[name] = text.replace(line, toml ? `version = "${version}"` : `$1"version": "${version}"`);
  }
  return out;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const root = fileURLToPath(new URL("../", import.meta.url));
    const files = Object.fromEntries(FILES.map((f) => [f, readFileSync(root + f, "utf8")]));
    const out = setVersion(files, process.argv[2] ?? "");
    for (const [name, text] of Object.entries(out)) writeFileSync(root + name, text);
    execFileSync("cargo", ["update", "--workspace", "--offline"], {
      cwd: root + "src-tauri",
      stdio: "inherit",
    });
    console.log(`Version ${process.argv[2]} set in ${FILES.join(", ")} and Cargo.lock.`);
  } catch (e) {
    console.error(e instanceof Error ? e.message : e);
    process.exit(1);
  }
}
