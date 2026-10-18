// node scripts/release-notes.mjs vX.Y.Z: imprime las notas de esa versión desde CHANGELOG.md.
// Las usa release.yml como cuerpo del borrador del release.
import { readFileSync } from "node:fs";
import process from "node:process";
import { notesFor } from "./release-lib.mjs";

const tag = process.argv[2];
if (!tag || !/^v\d+\.\d+\.\d+$/.test(tag)) {
  console.error("Uso: node scripts/release-notes.mjs vX.Y.Z");
  process.exit(1);
}
const changelog = readFileSync(new URL("../CHANGELOG.md", import.meta.url), "utf8");
const notes = notesFor(changelog, tag.slice(1));
if (!notes) {
  console.error(`CHANGELOG.md no tiene una sección para ${tag.slice(1)}.`);
  process.exit(1);
}
process.stdout.write(notes);
