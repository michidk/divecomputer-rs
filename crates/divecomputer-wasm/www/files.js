// File handling shared by the browser tools. No network requests carry dive data.
export const MAX_FILE_BYTES = 64 * 1024 * 1024;

export function checkFile(file) {
  if (!file || file.size === 0) throw new Error("Choose a non-empty file.");
  if (file.size > MAX_FILE_BYTES) throw new Error("Choose a file smaller than 64 MiB.");
}

export function sourceFormat(text) {
  const start = text.trimStart();
  if (start.startsWith("<")) return "uddf";
  if (start.startsWith("{")) return "divejson";
  throw new Error("Choose a DiveJSON document or a UDDF XML file.");
}

export function outputName(name, extension) {
  return `${name.replace(/\.[^.]+$/, "") || "dives"}.${extension}`;
}

export function save(text, type, name) {
  const url = URL.createObjectURL(new Blob([text], { type }));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export function readSavedDives(text, parse) {
  const saved = JSON.parse(text);
  if (!saved || !Array.isArray(saved.dives) || !saved.dives.length) {
    throw new Error("Choose a raw JSON download saved by the Shearwater downloader. Use the converter for DiveJSON or UDDF.");
  }
  const bytes = (hex, label, exactLength) => {
    if (typeof hex !== "string" || !/^(?:[0-9a-f]{2})+$/i.test(hex) || (exactLength && hex.length !== exactLength * 2)) {
      throw new Error(`Invalid ${label}: expected ${exactLength ? `${exactLength} bytes of ` : ""}hexadecimal data.`);
    }
    return Uint8Array.from(hex.match(/../g), (byte) => Number.parseInt(byte, 16));
  };
  const dives = saved.dives.map((entry, index) => {
    if (!entry || typeof entry !== "object") throw new Error(`Invalid dive ${index + 1}.`);
    const data = bytes(entry.data, `data for dive ${index + 1}`);
    const fingerprint = bytes(entry.fingerprint, `fingerprint for dive ${index + 1}`, 4);
    try {
      return { data, fingerprint, dive: parse(data) };
    } catch (error) {
      return { data, fingerprint, parseError: error.message };
    }
  });
  return { ...saved, deviceName: saved.device, dives };
}
