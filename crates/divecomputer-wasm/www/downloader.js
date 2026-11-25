import { checkFile, readSavedDives, save } from "./files.js";
const status = document.getElementById("status");
const downloadButton = document.getElementById("download");
try {
  const { default: init, downloadShearwater, exportDiveJson, exportUddf, shearwaterServiceUuid, parseShearwaterDive } = await import("./pkg/divecomputer_wasm.js");
  await init();
  const importFile = document.getElementById("import");
  importFile.disabled = false;
  downloadButton.disabled = !navigator.bluetooth;

  // Advertised name prefixes from the specification (shearwater.md, "Devices").
  const NAME_PREFIXES = ["Predator", "Petrel", "NERD", "Perdix", "Teric", "Peregrine", "Tern"];

  const saveButton = document.getElementById("save");
  const saveDiveJsonButton = document.getElementById("save-divejson");
  const saveUddfButton = document.getElementById("save-uddf");
  const table = document.getElementById("dives");
  const rows = table.querySelector("tbody");
  let result = null;

  const hex = (bytes) => Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
  const setStatus = (text, isError = false) => {
    status.textContent = text;
    status.classList.toggle("error", isError);
  };

  function addRow(dive, index) {
    // The fingerprint is the dive start time, u32 big-endian seconds on the device's
    // local clock, so it is shown without a time zone.
    const seconds = new DataView(dive.fingerprint.buffer, dive.fingerprint.byteOffset).getUint32(0);
    const start = new Date(seconds * 1000).toISOString().replace("T", " ").slice(0, 19);
    const row = rows.insertRow();
    const parsed = dive.dive;
    const summary = parsed
      ? [
          parsed.max_depth == null ? "" : `${parsed.max_depth.toFixed(1)} m`,
          parsed.duration == null ? "" : `${Math.round(parsed.duration / 60000)} min`,
          parsed.mode ?? "",
          parsed.samples.length,
        ]
      : [`Parse error: ${dive.parseError}`, "", "", ""];
    for (const text of [index + 1, start, hex(dive.fingerprint), dive.data.length, ...summary]) {
      row.insertCell().textContent = text;
    }
  }

  if (!navigator.bluetooth) {
    downloadButton.disabled = true;
    setStatus("This browser has no Web Bluetooth. Use Chrome or Edge on desktop or Android.", true);
  } else {
    setStatus("Ready. Connect a computer or open a saved raw download.");
    try {
      if (navigator.bluetooth.getAvailability && !(await navigator.bluetooth.getAvailability())) {
        setStatus("No Bluetooth adapter is available. Turn Bluetooth on, or open a saved raw download.");
      }
    } catch {
      setStatus("Bluetooth availability could not be checked. You can still try connecting or open a saved raw download.");
    }
  }

  importFile.addEventListener("change", async () => {
    result = null;
    saveButton.hidden = saveDiveJsonButton.hidden = saveUddfButton.hidden = true;
    rows.replaceChildren();
    table.hidden = true;
    importFile.disabled = true;
    downloadButton.disabled = true;
    try {
      const file = importFile.files[0];
      checkFile(file);
      result = readSavedDives(await file.text(), parseShearwaterDive);
      result.dives.forEach(addRow);
      table.hidden = false;
      saveButton.hidden = false;
      saveDiveJsonButton.hidden = saveUddfButton.hidden = !result.dives.some((dive) => dive.dive);
      const failed = result.dives.filter((dive) => !dive.dive).length;
      setStatus(`Opened ${result.dives.length} dives from ${file.name}.${failed ? ` ${failed} could not be parsed; their raw data is preserved in the JSON download.` : ""}`);
    } catch (error) {
      result = null;
      setStatus(`Import failed: ${error.message}`, true);
    } finally {
      importFile.disabled = false;
      downloadButton.disabled = !navigator.bluetooth;
    }
  });

  downloadButton.addEventListener("click", async () => {
    let device;
    try {
      device = await navigator.bluetooth.requestDevice({
        filters: NAME_PREFIXES.map((namePrefix) => ({ namePrefix })),
        optionalServices: [shearwaterServiceUuid()],
      });
    } catch (error) {
      setStatus(`No device selected: ${error.message}`, true);
      return;
    }

    downloadButton.disabled = true;
    importFile.disabled = true;
    result = null;
    saveButton.hidden = true;
    saveDiveJsonButton.hidden = saveUddfButton.hidden = true;
    rows.replaceChildren();
    table.hidden = false;
    setStatus(`Connecting to ${device.name}…`);
    let count = 0;
    try {
      result = await downloadShearwater(device, null, (dive) => {
        addRow(dive, count++);
        setStatus(`Downloading… ${count} dive${count === 1 ? "" : "s"} so far`);
      });
      result.deviceName = device.name;
      setStatus(`${result.model}, serial ${result.serial}, firmware ${result.firmware}: ${result.dives.length} dives downloaded.`);
      saveButton.hidden = result.dives.length === 0;
      saveDiveJsonButton.hidden = saveUddfButton.hidden = !result.dives.some((dive) => dive.dive);
    } catch (error) {
      setStatus(`Download failed: ${error.message}`, true);
    } finally {
      downloadButton.disabled = false;
      importFile.disabled = false;
    }
  });

  saveButton.addEventListener("click", () => {
    const json = JSON.stringify({
      device: result.deviceName,
      model: result.model,
      serial: result.serial,
      firmware: result.firmware,
      dives: result.dives.map((dive) => ({ fingerprint: hex(dive.fingerprint), data: hex(dive.data), dive: dive.dive, parseError: dive.parseError })),
    }, null, 2);
    save(json, "application/json", `shearwater-${result.serial}.json`);
  });

  function saveExport(format, exporter, type, extension) {
    // Dives that failed to parse cannot be exported; the raw JSON save keeps them.
    const parsed = result.dives.filter((dive) => dive.dive);
    try {
      const text = exporter(parsed.map((dive) => dive.data));
      save(text, type, `shearwater-${result.serial}.${extension}`);
      const skipped = result.dives.length - parsed.length;
      setStatus(`Saved ${parsed.length} dives as ${format}${skipped ? `; ${skipped} that failed to parse were left out` : ""}.`);
    } catch (error) {
      setStatus(`${format} export failed: ${error.message}`, true);
    }
  }
  saveDiveJsonButton.addEventListener("click", () => saveExport("DiveJSON", exportDiveJson, "application/vnd.dive+json", "divejson"));
  saveUddfButton.addEventListener("click", () => saveExport("UDDF", exportUddf, "application/xml", "uddf"));
} catch (error) {
  downloadButton.disabled = true;
  document.getElementById("import").disabled = true;
  status.classList.add("error");
  status.textContent = `The downloader could not load. Reload the page to try again. ${error.message}`;
}
