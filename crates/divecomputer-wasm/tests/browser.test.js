import { afterAll, expect, spyOn, test } from "bun:test";
import { Window } from "happy-dom";
import init, { convertDiveJsonToUddf, convertUddfToDiveJson, parseShearwaterDive } from "../www/pkg/divecomputer_wasm.js";
import { checkFile, MAX_FILE_BYTES, outputName, readSavedDives, sourceFormat } from "../www/files.js";

await init({ module_or_path: await Bun.file(new URL("../www/pkg/divecomputer_wasm_bg.wasm", import.meta.url)).arrayBuffer() });

const uddf = `<uddf version="3.2.3"><profiledata><repetitiongroup><dive id="one">
<informationbeforedive><datetime>2026-04-19T08:12:00+02:00</datetime><surfacepressure>500000</surfacepressure></informationbeforedive>
<informationafterdive><greatestdepth>18.4</greatestdepth></informationafterdive>
</dive></repetitiongroup></profiledata></uddf>`;
const empty = '{"format":"divejson","version":"1.0","exported_at":"2026-04-19T10:00:00Z"}';

test("real WebAssembly converts a dive in both directions and reports decisions", () => {
  const json = convertUddfToDiveJson(uddf);
  expect(json.dive_count).toBe(1);
  expect(JSON.parse(json.text).dives[0].max_depth).toBe(18.4);
  expect(json.notes.some((note) => note.code === "surface_pressure_out_of_range")).toBe(true);
  expect(json.notes.every((note) => typeof note.kind === "string" && typeof note.message === "string")).toBe(true);
  const xml = convertDiveJsonToUddf(json.text);
  expect(xml.dive_count).toBe(1);
  expect(xml.text).toContain("<greatestdepth>18.4</greatestdepth>");
  expect(JSON.parse(convertUddfToDiveJson(xml.text).text).dives[0].max_depth).toBe(18.4);
});

test("invalid documents cannot produce a download", () => {
  expect(() => convertDiveJsonToUddf('{"format":"divejson","format":"divejson"}')).toThrow();
  expect(() => convertDiveJsonToUddf('{"format":"other"}')).toThrow();
  expect(() => convertUddfToDiveJson('<other/>')).toThrow();
  expect(() => convertUddfToDiveJson('<uddf>')).toThrow();
  expect(() => convertUddfToDiveJson('<!DOCTYPE uddf><uddf/>')).toThrow();
  expect(convertDiveJsonToUddf(empty).dive_count).toBe(0);
});

test("file detection handles a BOM and rejects unsupported input and oversize files", () => {
  expect(sourceFormat(`\ufeff \n${uddf}`)).toBe("uddf");
  expect(sourceFormat(empty)).toBe("divejson");
  expect(() => sourceFormat("csv,data")).toThrow();
  expect(() => checkFile({ size: 0 })).toThrow();
  expect(() => checkFile({ size: MAX_FILE_BYTES + 1 })).toThrow();
  expect(outputName("holiday.dives.json", "uddf")).toBe("holiday.dives.uddf");
});

test("raw imports validate hex and reparse bytes rather than trusting stored summaries", () => {
  const saved = JSON.stringify({ dives: [{ fingerprint: "00000001", data: "0102", dive: { max_depth: 999 } }] });
  const imported = readSavedDives(saved, parseShearwaterDive);
  expect(imported.dives[0].data).toEqual(new Uint8Array([1, 2]));
  expect(imported.dives[0].dive).toBeUndefined();
  expect(imported.dives[0].parseError).toBeString();
  expect(() => readSavedDives(saved.replace('"0102"', '"xyz"'), parseShearwaterDive)).toThrow();
  expect(() => readSavedDives(saved.replace('"00000001"', '"00"'), parseShearwaterDive)).toThrow();
  expect(() => readSavedDives(empty, parseShearwaterDive)).toThrow();
  expect(() => readSavedDives("null", parseShearwaterDive)).toThrow();
});

const window = new Window({ settings: { disableJavaScriptFileLoading: true, disableCSSFileLoading: true, disableComputedStyleRendering: true } });
afterAll(() => window.happyDOM.abort());
async function select(input, text, name) {
  Object.defineProperty(input, "files", { configurable: true, value: [{ name, size: text.length, text: async () => text }] });
  input.dispatchEvent(new window.Event("change"));
  await new Promise((resolve) => setTimeout(resolve, 0));
}

test("converter UI switches output formats and removes stale downloads after failure", async () => {
  window.document.body.innerHTML = await Bun.file(new URL("../www/convert.html", import.meta.url)).text();
  globalThis.document = window.document;
  await import("../www/converter.js");
  const input = document.getElementById("source");
  expect(input.disabled).toBe(false);
  await select(input, uddf, "holiday.uddf");
  expect(document.getElementById("save").textContent).toBe("Download DiveJSON");
  expect(document.getElementById("result").hidden).toBe(false);
  expect(document.getElementById("notes").children.length).toBeGreaterThan(0);
  await select(input, empty, "empty.json");
  expect(document.getElementById("save").textContent).toBe("Download UDDF");
  let blob;
  let filename;
  const createUrl = spyOn(URL, "createObjectURL").mockImplementation((value) => {
    blob = value;
    return "blob:test";
  });
  const click = spyOn(window.HTMLAnchorElement.prototype, "click").mockImplementation(function () {
    filename = this.download;
  });
  try {
    document.getElementById("save").click();
    expect(filename).toBe("empty.uddf");
    expect(await blob.text()).toContain("<uddf");
    await select(input, '{"not":"divejson"}', "bad.json");
    document.getElementById("save").click();
    expect(createUrl).toHaveBeenCalledTimes(1);
  } finally {
    createUrl.mockRestore();
    click.mockRestore();
  }
  expect(document.getElementById("status").textContent).toContain("Conversion failed");
  expect(document.getElementById("result").hidden).toBe(true);
});

test("downloader supports raw import when Web Bluetooth is unavailable", async () => {
  window.document.body.innerHTML = await Bun.file(new URL("../www/index.html", import.meta.url)).text();
  globalThis.document = window.document;
  Object.defineProperty(globalThis, "navigator", { configurable: true, value: {} });
  await import("../www/downloader.js");
  const input = document.getElementById("import");
  expect(input.disabled).toBe(false);
  expect(document.getElementById("download").disabled).toBe(true);
  await select(input, JSON.stringify({ dives: [{ fingerprint: "00000001", data: "0102" }] }), "raw.json");
  expect(document.getElementById("dives").hidden).toBe(false);
  expect(document.querySelectorAll("tbody tr").length).toBe(1);
  expect(document.getElementById("save").hidden).toBe(false);
  expect(document.getElementById("save-uddf").hidden).toBe(true);
  await select(input, "{}", "bad.json");
  expect(document.getElementById("status").textContent).toContain("Import failed");
  expect(document.getElementById("save").hidden).toBe(true);
});
