// Report minified + gzipped size of the ESM bundle (with and without fflate inlined).
import { gzipSync } from "node:zlib";
import { build } from "esbuild";

async function measure(label, external) {
  const r = await build({
    entryPoints: ["src/index.ts"],
    bundle: true,
    minify: true,
    format: "esm",
    target: "es2022",
    platform: "neutral",
    external,
    write: false,
    treeShaking: true,
  });
  const code = r.outputFiles[0].contents;
  const gz = gzipSync(code, { level: 9 }).length;
  console.log(`${label.padEnd(28)} min ${(code.length / 1024).toFixed(1).padStart(6)} KiB   min+gz ${(gz / 1024).toFixed(1).padStart(5)} KiB`);
}

await measure("bundle (incl. fflate)", []);
await measure("library only (fflate ext.)", ["fflate"]);
