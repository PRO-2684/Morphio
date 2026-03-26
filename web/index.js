import init, { morphFontMany, MorphOptions } from "./wasm/morphio.js";

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
    ruleList: document.querySelector("#rule-list"),
    ruleTemplate: document.querySelector("#morph-rule-template"),
    addRuleButton: document.querySelector("#add-rule-button"),
    wordMatchStart: document.querySelector("#word-match-start"),
    wordMatchEnd: document.querySelector("#word-match-end"),
    status: document.querySelector("#status"),
    statusText: document.querySelector("#status-text"),
    sourcePreview: document.querySelector("#source-preview"),
    morphedPreview: document.querySelector("#morphed-preview"),
    morphButton: document.querySelector("#morph-button"),
    downloadButton: document.querySelector("#download-button"),
};

function registerServiceWorker() {
    navigator.serviceWorker
        .register("/sw.js")
        .then((registration) => {
            console.log("Service Worker registered:", registration);
        })
        .catch((error) => {
            console.log("Service Worker registration failed:", error);
        });
}

async function boot() {
    if ("serviceWorker" in navigator) {
        registerServiceWorker();
    }
    hydrateFromSearchParams();
    syncStateToUrl();
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
    elements.addRuleButton.addEventListener("click", addRuleRow);
    elements.ruleList.addEventListener("click", onRuleListClick);
    elements.ruleList.addEventListener("input", syncStateToUrl);
    elements.sourcePreview.addEventListener("input", mirrorPreviewText);
    elements.sourcePreview.addEventListener("input", syncStateToUrl);
    elements.wordMatchStart.addEventListener("change", syncStateToUrl);
    elements.wordMatchEnd.addEventListener("change", syncStateToUrl);
    elements.morphButton.addEventListener("click", morphCurrentFont);
    elements.downloadButton.addEventListener("click", downloadFont);
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
        const options = new MorphOptions(
            elements.wordMatchStart.checked,
            elements.wordMatchEnd.checked,
        );
        const morphed = morphFontMany(
            state.sourceBytes,
            collectRules(),
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

function addRuleRow() {
    const fragment = elements.ruleTemplate.content.cloneNode(true);
    elements.ruleList.append(fragment);
    syncStateToUrl();
}

function onRuleListClick(event) {
    const removeButton = event.target.closest(".remove-rule-button");
    if (!removeButton) {
        return;
    }

    removeButton.closest(".rule-row")?.remove();
    syncStateToUrl();
}

function collectRules() {
    const rows = Array.from(elements.ruleList.querySelectorAll(".rule-row"));
    const rules = rows
        .map((row) => ({
            from: row.querySelector('[data-role="from"]').value.trim(),
            to: row.querySelector('[data-role="to"]').value.trim(),
        }))
        .filter((rule) => rule.from || rule.to);

    if (rules.length === 0) {
        throw new Error("Add at least one morph rule.");
    }

    for (const rule of rules) {
        if (!rule.from || !rule.to) {
            throw new Error(
                "Each morph rule must include both source and target words.",
            );
        }
    }

    return rules.map((rule) => [rule.from, rule.to]);
}

function hydrateFromSearchParams() {
    const params = new URLSearchParams(window.location.search);
    const start = params.get("start");
    const end = params.get("end");
    const preview = params.get("preview");
    const fromValues = params.getAll("from");
    const toValues = params.getAll("to");

    if (start !== null) {
        elements.wordMatchStart.checked = start !== "0";
    }
    if (end !== null) {
        elements.wordMatchEnd.checked = end !== "0";
    }
    if (preview !== null) {
        elements.sourcePreview.value = preview;
    }

    if (fromValues.length === 0 && toValues.length === 0) {
        mirrorPreviewText();
        return;
    }

    elements.ruleList.innerHTML = "";
    const pairCount = Math.min(fromValues.length, toValues.length);
    for (let index = 0; index < pairCount; index += 1) {
        addRuleRowWithValues(fromValues[index], toValues[index], index === 0);
    }
    if (pairCount === 0) {
        addRuleRowWithValues("banana", "orange", true);
    }
    mirrorPreviewText();
}

function addRuleRowWithValues(from, to, isFirstRow) {
    const fragment = elements.ruleTemplate.content.cloneNode(true);
    const row = fragment.querySelector(".rule-row");
    row.querySelector('[data-role="from"]').value = from;
    row.querySelector('[data-role="to"]').value = to;

    if (isFirstRow) {
        row.querySelector(".remove-rule-button")?.remove();
        const addButton = document.createElement("button");
        addButton.id = "add-rule-button";
        addButton.className = "secondary rule-action-button";
        addButton.type = "button";
        addButton.textContent = "+";
        row.append(addButton);
    }

    elements.ruleList.append(fragment);
}

function collectRulesForUrl() {
    return Array.from(elements.ruleList.querySelectorAll(".rule-row"))
        .map((row) => [
            row.querySelector('[data-role="from"]').value.trim(),
            row.querySelector('[data-role="to"]').value.trim(),
        ])
        .filter(([from, to]) => from || to);
}

function syncStateToUrl() {
    const params = new URLSearchParams();
    params.set("start", elements.wordMatchStart.checked ? "1" : "0");
    params.set("end", elements.wordMatchEnd.checked ? "1" : "0");
    params.set("preview", elements.sourcePreview.value);

    for (const [from, to] of collectRulesForUrl()) {
        params.append("from", from);
        params.append("to", to);
    }

    const query = params.toString();
    const url = query
        ? `${window.location.pathname}?${query}`
        : window.location.pathname;
    window.history.replaceState(null, "", url);
}

mirrorPreviewText();
boot();
