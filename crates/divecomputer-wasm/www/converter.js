import { checkFile, sourceFormat, outputName, save } from "./files.js";

const source = document.getElementById("source");
const status = document.getElementById("status");
const result = document.getElementById("result");
const saveButton = document.getElementById("save");
const notes = document.getElementById("notes");
let converted = null;
let revision = 0;
const setStatus = (text, error = false) => {
  status.textContent = text;
  status.classList.toggle("error", error);
};

try {
  const wasm = await import("./pkg/divecomputer_wasm.js");
  await wasm.default();
  source.disabled = false;
  setStatus("Ready. Choose a file to convert.");
  source.addEventListener("change", async () => {
    const current = ++revision;
    converted = null;
    result.hidden = true;
    notes.replaceChildren();
    try {
      const file = source.files[0];
      checkFile(file);
      setStatus(`Reading ${file.name}…`);
      const text = await file.text();
      if (current !== revision) return;
      const format = sourceFormat(text);
      const output = format === "uddf"
        ? wasm.convertUddfToDiveJson(text)
        : wasm.convertDiveJsonToUddf(text);
      const extension = format === "uddf" ? "divejson" : "uddf";
      const label = format === "uddf" ? "DiveJSON" : "UDDF";
      converted = { ...output, name: outputName(file.name, extension), type: format === "uddf" ? "application/vnd.dive+json" : "application/xml" };
      setStatus(`${file.name}: ${output.dive_count} dive${output.dive_count === 1 ? "" : "s"} ready to save as ${label}.`);
      saveButton.textContent = `Download ${label}`;
      document.getElementById("report-summary").textContent = `${output.notes.length} conversion note${output.notes.length === 1 ? "" : "s"}`;
      document.getElementById("report").open = output.notes.some((note) => note.kind === "dropped" || note.kind === "resolved");
      for (const note of output.notes) {
        const item = document.createElement("li");
        item.textContent = `${note.kind}: ${note.message} (${note.at}${note.member ? `; ${note.member}` : ""})`;
        notes.append(item);
      }
      result.hidden = false;
    } catch (error) {
      if (current === revision) setStatus(`Conversion failed: ${error.message}`, true);
    }
  });
} catch (error) {
  setStatus(`The converter could not load. Reload the page to try again. ${error.message}`, true);
}

saveButton.addEventListener("click", () => {
  if (converted) save(converted.text, converted.type, converted.name);
});
document.getElementById("save-report").addEventListener("click", () => {
  if (converted) save(JSON.stringify(converted.notes, null, 2), "application/json", outputName(converted.name, "report.json"));
});
