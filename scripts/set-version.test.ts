import { describe, expect, it } from "vitest";
import { setVersion } from "./set-version.mjs";

const files = {
  "package.json":
    '{\n  "name": "wagecraft",\n  "version": "0.1.0",\n  "dependencies": { "react": "^19.1.0" }\n}\n',
  "src-tauri/Cargo.toml":
    '[package]\nname = "wagecraft"\nversion = "0.1.0"\nedition = "2021"\n\n[dependencies]\ntauri = { version = "2", features = [] }\n',
  "src-tauri/tauri.conf.json":
    '{\n  "productName": "Wagecraft",\n  "version": "0.1.0",\n  "build": { "devUrl": "http://localhost:1420" }\n}\n',
};

describe("setVersion", () => {
  it("writes the same version into the three files and changes nothing else", () => {
    const out = setVersion(files, "1.0.0");

    expect(out["package.json"]).toBe(
      '{\n  "name": "wagecraft",\n  "version": "1.0.0",\n  "dependencies": { "react": "^19.1.0" }\n}\n',
    );
    expect(out["src-tauri/Cargo.toml"]).toBe(
      '[package]\nname = "wagecraft"\nversion = "1.0.0"\nedition = "2021"\n\n[dependencies]\ntauri = { version = "2", features = [] }\n',
    );
    expect(out["src-tauri/tauri.conf.json"]).toBe(
      '{\n  "productName": "Wagecraft",\n  "version": "1.0.0",\n  "build": { "devUrl": "http://localhost:1420" }\n}\n',
    );
  });

  it("refuses a version that isn't three numbers", () => {
    for (const bad of ["1.0", "v1.0.0", "1.0.0-rc1", ""]) {
      expect(() => setVersion(files, bad)).toThrow(/like 1\.0\.0/);
    }
  });

  it("refuses to continue when a file has no version line", () => {
    const broken = { ...files, "src-tauri/Cargo.toml": '[package]\nname = "wagecraft"\n' };

    expect(() => setVersion(broken, "1.0.0")).toThrow(/Cargo\.toml/);
  });
});
