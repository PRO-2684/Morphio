import init, { morphFont } from "./pkg/morphio.js";

const MORPHED_FAMILY = "MorphioPreview";
const DEFAULT_PREVIEW = `banana
bandana
bananarama

Try typing the source word in different contexts here.`;

const state = {
    wasmReady: false,
    sourceBytes: null,
    sourceName: "",
    outputBytes: null,
    fontUrl: null,
    morphToken: 0,
};

const elements = {
    fileInput: document.querySelector("#font-file"),
    fromWord: document.querySelector("#from-word"),
    toWord: document.querySelector("#to-word"),
    fromCount: document.querySelector("#from-count"),
    toCount: document.querySelector("#to-count"),
    fontName: document.querySelector("#font-name"),
    fontMeta: document.querySelector("#font-meta"),
    status: document.querySelector("#status"),
    statusText: document.querySelector("#status-text"),
    preview: document.querySelector("#preview"),
    previewNote: document.querySelector("#preview-note"),
    downloadButton: document.querySelector("#download-button"),
    resetPreviewButton: document.querySelector("#reset-preview"),
};

async function boot() {
    wireUi();
    syncCounts();

    try {
        await init();
        state.wasmReady = true;
        setStatus("ready", "WebAssembly ready. Upload a font to begin.");
        scheduleMorph();
    } catch (error) {
        setStatus("error", `Failed to load WebAssembly: ${formatError(error)}`);
    }
}

function wireUi() {
    elements.fileInput.addEventListener("change", onFileChange);
    elements.fromWord.addEventListener("input", syncCounts);
    elements.toWord.addEventListener("input", syncCounts);
    elements.fromWord.addEventListener("input", scheduleMorph);
    elements.toWord.addEventListener("input", scheduleMorph);
    elements.downloadButton.addEventListener("click", downloadFont);
    elements.resetPreviewButton.addEventListener("click", () => {
        elements.preview.value = DEFAULT_PREVIEW;
    });
}

async function onFileChange(event) {
    const [file] = event.currentTarget.files ?? [];
    clearOutputFont();

    if (!file) {
        state.sourceBytes = null;
        state.sourceName = "";
        elements.fontName.textContent = "No font selected";
        elements.fontMeta.textContent = "Upload a font to start morphing.";
        setStatus(
            state.wasmReady ? "idle" : "loading",
            state.wasmReady
                ? "Upload a font to begin."
                : "Loading WebAssembly…",
        );
        return;
    }

    const buffer = await file.arrayBuffer();
    state.sourceBytes = new Uint8Array(buffer);
    state.sourceName = file.name;
    elements.fontName.textContent = file.name;
    elements.fontMeta.textContent = `${formatBytes(file.size)} / ${file.type || "font file"}`;
    scheduleMorph();
}

function syncCounts() {
    elements.fromCount.textContent = `${[...elements.fromWord.value].length} chars`;
    elements.toCount.textContent = `${[...elements.toWord.value].length} chars`;
}

function scheduleMorph() {
    if (!state.wasmReady) {
        return;
    }
    if (!state.sourceBytes) {
        clearOutputFont();
        setStatus("idle", "Upload a font to begin.");
        return;
    }

    void morphCurrentFont();
}

async function morphCurrentFont() {
    const token = ++state.morphToken;
    const fromWord = elements.fromWord.value;
    const toWord = elements.toWord.value;

    setStatus("loading", "Morphing font…");
    elements.downloadButton.disabled = true;
    elements.previewNote.textContent = "Morphing in progress…";

    await nextFrame();

    try {
        const morphed = morphFont(state.sourceBytes, fromWord, toWord);
        if (token !== state.morphToken) {
            return;
        }

        state.outputBytes = morphed;
        applyPreviewFont(morphed);
        elements.downloadButton.disabled = false;
        elements.previewNote.textContent = "Preview uses the morphed font.";
        setStatus("ready", "Font morphed successfully.");
    } catch (error) {
        if (token !== state.morphToken) {
            return;
        }

        clearOutputFont();
        elements.previewNote.textContent = "Preview is using the default font.";
        setStatus("error", formatError(error));
    }
}

function applyPreviewFont(fontBytes) {
    clearPreviewFontUrl();
    const blob = new Blob([fontBytes], { type: "font/ttf" });
    const url = URL.createObjectURL(blob);
    state.fontUrl = url;

    const styleId = "morphio-preview-style";
    let style = document.getElementById(styleId);
    if (!style) {
        style = document.createElement("style");
        style.id = styleId;
        document.head.append(style);
    }

    style.textContent = `
        @font-face {
            font-family: "${MORPHED_FAMILY}";
            src: url("${url}");
        }
    `;

    elements.preview.style.fontFamily = `"${MORPHED_FAMILY}", "Iowan Old Style", serif`;
    elements.preview.classList.add("ready");
}

function clearOutputFont() {
    state.outputBytes = null;
    clearPreviewFontUrl();
    elements.downloadButton.disabled = true;
    elements.preview.style.fontFamily = "";
    elements.preview.classList.remove("ready");
    elements.previewNote.textContent = "Waiting for a morphed font.";
}

function clearPreviewFontUrl() {
    if (state.fontUrl) {
        URL.revokeObjectURL(state.fontUrl);
        state.fontUrl = null;
    }
}

function downloadFont() {
    if (!state.outputBytes) {
        return;
    }

    const url = URL.createObjectURL(
        new Blob([state.outputBytes], { type: "application/octet-stream" }),
    );
    const link = document.createElement("a");
    link.href = url;
    link.download = buildOutputName(state.sourceName);
    link.click();
    URL.revokeObjectURL(url);
}

function buildOutputName(name) {
    if (!name) {
        return "morphio-morphed-font.ttf";
    }

    const dot = name.lastIndexOf(".");
    if (dot <= 0) {
        return `${name}-morphed`;
    }

    return `${name.slice(0, dot)}-morphed${name.slice(dot)}`;
}

function setStatus(stateName, message) {
    elements.status.dataset.state = stateName;
    elements.status.dataset.busy = stateName === "loading" ? "true" : "false";
    elements.statusText.textContent = message;
}

function formatBytes(size) {
    if (size < 1024) {
        return `${size} B`;
    }
    if (size < 1024 * 1024) {
        return `${(size / 1024).toFixed(1)} KB`;
    }
    return `${(size / (1024 * 1024)).toFixed(2)} MB`;
}

function formatError(error) {
    return error instanceof Error ? error.message : String(error);
}

function nextFrame() {
    return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

boot();
