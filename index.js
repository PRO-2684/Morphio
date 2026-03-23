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
    isMorphing: false,
};

const elements = {
    fileInput: document.querySelector("#font-file"),
    fromWord: document.querySelector("#from-word"),
    toWord: document.querySelector("#to-word"),
    fromCount: document.querySelector("#from-count"),
    toCount: document.querySelector("#to-count"),
    fontSelection: document.querySelector("#font-selection"),
    fontName: document.querySelector("#font-name"),
    fontMeta: document.querySelector("#font-meta"),
    status: document.querySelector("#status"),
    statusText: document.querySelector("#status-text"),
    preview: document.querySelector("#preview"),
    morphButton: document.querySelector("#morph-button"),
    downloadButton: document.querySelector("#download-button"),
    resetPreviewButton: document.querySelector("#reset-preview"),
};

async function boot() {
    wireUi();
    syncCounts();

    try {
        await init();
        state.wasmReady = true;
        updateActionState();
        setStatus("idle", "Upload a font, then click Morph font.");
    } catch (error) {
        setStatus("error", `Failed to load WebAssembly: ${formatError(error)}`);
    }
}

function wireUi() {
    elements.fileInput.addEventListener("change", onFileChange);
    elements.fromWord.addEventListener("input", syncCounts);
    elements.toWord.addEventListener("input", syncCounts);
    elements.morphButton.addEventListener("click", morphCurrentFont);
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
        elements.fontSelection.classList.remove("ready");
        elements.fontName.textContent = "";
        elements.fontMeta.textContent = "";
        updateActionState();
        setStatus(
            state.wasmReady ? "idle" : "loading",
            state.wasmReady
                ? "Upload a font, then click Morph font."
                : "Loading WebAssembly…",
        );
        return;
    }

    const buffer = await file.arrayBuffer();
    state.sourceBytes = new Uint8Array(buffer);
    state.sourceName = file.name;
    elements.fontSelection.classList.add("ready");
    elements.fontName.textContent = file.name;
    elements.fontMeta.textContent = `${formatBytes(file.size)} / ${file.type || "font file"}`;
    updateActionState();
    setStatus("idle", "Font loaded. Click Morph font to rebuild it.");
}

function syncCounts() {
    elements.fromCount.textContent = `${[...elements.fromWord.value].length} chars`;
    elements.toCount.textContent = `${[...elements.toWord.value].length} chars`;
}

async function morphCurrentFont() {
    if (!state.wasmReady || !state.sourceBytes || state.isMorphing) {
        return;
    }

    state.isMorphing = true;
    updateActionState();
    setStatus("loading", "Morphing font…");

    try {
        await nextFrame();
        const morphed = morphFont(
            state.sourceBytes,
            elements.fromWord.value,
            elements.toWord.value,
        );

        state.outputBytes = morphed;
        applyPreviewFont(morphed);
        setStatus("ready", "Font morphed successfully. Preview and download are ready.");
    } catch (error) {
        clearOutputFont();
        setStatus("error", formatError(error));
    } finally {
        state.isMorphing = false;
        updateActionState();
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
    elements.preview.style.fontFamily = "";
    elements.preview.classList.remove("ready");
    updateActionState();
}

function clearPreviewFontUrl() {
    if (state.fontUrl) {
        URL.revokeObjectURL(state.fontUrl);
        state.fontUrl = null;
    }
}

function updateActionState() {
    const canMorph = state.wasmReady && !!state.sourceBytes && !state.isMorphing;
    elements.morphButton.disabled = !canMorph;
    elements.downloadButton.disabled = !state.outputBytes || state.isMorphing;
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
