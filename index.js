import init, { morphFont, MorphOptions } from "./pkg/morphio.js";

const ORIGINAL_FAMILY = "MorphioOriginalPreview";
const MORPHED_FAMILY = "MorphioPreview";
const DEFAULT_PREVIEW = "banana.";

const state = {
    wasmReady: false,
    sourceBytes: null,
    sourceName: "",
    outputBytes: null,
    sourceFontUrl: null,
    fontUrl: null,
    isMorphing: false,
};

const elements = {
    fileInput: document.querySelector("#font-file"),
    fromWord: document.querySelector("#from-word"),
    toWord: document.querySelector("#to-word"),
    wordMatch: document.querySelector("#word-match"),
    status: document.querySelector("#status"),
    statusText: document.querySelector("#status-text"),
    sourcePreview: document.querySelector("#source-preview"),
    morphedPreview: document.querySelector("#morphed-preview"),
    morphButton: document.querySelector("#morph-button"),
    downloadButton: document.querySelector("#download-button"),
    resetPreviewButton: document.querySelector("#reset-preview"),
};

async function boot() {
    wireUi();

    try {
        await init();
        state.wasmReady = true;
        updateActionState();
        setStatus("idle", "Choose a font and click Morph.");
    } catch (error) {
        setStatus("error", `Failed to load WebAssembly: ${formatError(error)}`);
    }
}

function wireUi() {
    elements.fileInput.addEventListener("change", onFileChange);
    elements.sourcePreview.addEventListener("input", mirrorPreviewText);
    elements.morphButton.addEventListener("click", morphCurrentFont);
    elements.downloadButton.addEventListener("click", downloadFont);
    elements.resetPreviewButton.addEventListener("click", () => {
        elements.sourcePreview.value = DEFAULT_PREVIEW;
        mirrorPreviewText();
    });
}

async function onFileChange(event) {
    const [file] = event.currentTarget.files ?? [];
    clearOutputFont();

    if (!file) {
        state.sourceBytes = null;
        state.sourceName = "";
        clearOriginalPreviewFontUrl();
        elements.sourcePreview.style.fontFamily = "";
        updateActionState();
        setStatus(
            state.wasmReady ? "idle" : "loading",
            state.wasmReady
                ? "Choose a font and click Morph."
                : "Loading WebAssembly…",
        );
        return;
    }

    const buffer = await file.arrayBuffer();
    state.sourceBytes = new Uint8Array(buffer);
    state.sourceName = file.name;
    applyOriginalPreviewFont(state.sourceBytes);
    updateActionState();
    setStatus("idle", "Font loaded. Click Morph.");
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
        const options = new MorphOptions(elements.wordMatch.checked);
        const morphed = morphFont(
            state.sourceBytes,
            elements.fromWord.value,
            elements.toWord.value,
            options,
        );

        state.outputBytes = morphed;
        applyPreviewFont(morphed);
        setStatus("ready", "Morphed font ready.");
    } catch (error) {
        clearOutputFont();
        setStatus("error", formatError(error));
    } finally {
        state.isMorphing = false;
        updateActionState();
    }
}

function applyPreviewFont(fontBytes) {
    clearMorphedPreviewFontUrl();
    const blob = new Blob([fontBytes], { type: "font/ttf" });
    const url = URL.createObjectURL(blob);
    state.fontUrl = url;

    setPreviewFontFace("morphio-morphed-style", MORPHED_FAMILY, url);
    elements.morphedPreview.style.fontFamily = `"${MORPHED_FAMILY}", sans-serif`;
}

function applyOriginalPreviewFont(fontBytes) {
    clearOriginalPreviewFontUrl();
    const blob = new Blob([fontBytes], { type: "font/ttf" });
    const url = URL.createObjectURL(blob);
    state.sourceFontUrl = url;

    setPreviewFontFace("morphio-original-style", ORIGINAL_FAMILY, url);
    elements.sourcePreview.style.fontFamily = `"${ORIGINAL_FAMILY}", sans-serif`;
}

function clearOutputFont() {
    state.outputBytes = null;
    clearMorphedPreviewFontUrl();
    elements.morphedPreview.style.fontFamily = "";
    updateActionState();
}

function clearOriginalPreviewFontUrl() {
    if (state.sourceFontUrl) {
        URL.revokeObjectURL(state.sourceFontUrl);
        state.sourceFontUrl = null;
    }
}

function clearMorphedPreviewFontUrl() {
    if (state.fontUrl) {
        URL.revokeObjectURL(state.fontUrl);
        state.fontUrl = null;
    }
}

function setPreviewFontFace(styleId, family, url) {
    let style = document.getElementById(styleId);
    if (!style) {
        style = document.createElement("style");
        style.id = styleId;
        document.head.append(style);
    }

    style.textContent = `
        @font-face {
            font-family: "${family}";
            src: url("${url}");
        }
    `;
}

function mirrorPreviewText() {
    elements.morphedPreview.value = elements.sourcePreview.value;
}

function updateActionState() {
    const canMorph =
        state.wasmReady && !!state.sourceBytes && !state.isMorphing;
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

function formatError(error) {
    return error instanceof Error ? error.message : String(error);
}

function nextFrame() {
    return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

mirrorPreviewText();
boot();
