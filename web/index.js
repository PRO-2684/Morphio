import init, {
    morphFontMany,
    MorphOptions,
    parseRecipe,
    serializeRecipe,
} from "./wasm/morphio.js";

const ORIGINAL_FAMILY = "MorphioOriginalPreview";
const MORPHED_FAMILY = "MorphioPreview";

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
    recipeFileInput: document.querySelector("#recipe-file"),
    ruleList: document.querySelector("#rule-list"),
    ruleTemplate: document.querySelector("#morph-rule-template"),
    importRecipeButton: document.querySelector("#import-recipe-button"),
    exportRecipeButton: document.querySelector("#export-recipe-button"),
    wordMatchStart: document.querySelector("#word-match-start"),
    wordMatchEnd: document.querySelector("#word-match-end"),
    skipMissingGlyphs: document.querySelector("#skip-missing-glyphs"),
    status: document.querySelector("#status"),
    statusText: document.querySelector("#status-text"),
    sourcePreview: document.querySelector("#source-preview"),
    morphedPreview: document.querySelector("#morphed-preview"),
    morphButton: document.querySelector("#morph-button"),
    downloadButton: document.querySelector("#download-button"),
};

function registerServiceWorker() {
    navigator.serviceWorker
        .register("./sw.js")
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
    elements.recipeFileInput.addEventListener("change", importRecipe);
    elements.ruleList.addEventListener("click", onRuleListClick);
    elements.sourcePreview.addEventListener("input", mirrorPreviewText);
    elements.importRecipeButton.addEventListener("click", openRecipePicker);
    elements.exportRecipeButton.addEventListener("click", exportRecipe);
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
            elements.skipMissingGlyphs.checked,
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
    elements.exportRecipeButton.disabled = state.isMorphing;
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
}

function onRuleListClick(event) {
    const addButton = event.target.closest("#add-rule-button");
    if (addButton) {
        addRuleRow();
        return;
    }

    const removeButton = event.target.closest(".remove-rule-button");
    if (!removeButton) {
        return;
    }

    removeButton.closest(".rule-row")?.remove();
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

function openRecipePicker() {
    elements.recipeFileInput.click();
}

async function importRecipe(event) {
    const [file] = event.currentTarget.files ?? [];
    if (!file) {
        return;
    }

    try {
        const contents = await file.text();
        applyRecipe(parseRecipe(contents));
        setStatus("ready", `Loaded recipe: ${file.name}`);
    } catch (error) {
        setStatus("error", `Failed to import recipe: ${formatError(error)}`);
    } finally {
        event.target.value = "";
    }
}

function exportRecipe() {
    try {
        const recipe = serializeRecipe(
            collectRules(),
            new MorphOptions(
                elements.wordMatchStart.checked,
                elements.wordMatchEnd.checked,
                elements.skipMissingGlyphs.checked,
            ),
        );
        const blob = new Blob([recipe], { type: "text/plain;charset=utf-8" });
        const url = URL.createObjectURL(blob);
        const link = document.createElement("a");
        link.href = url;
        link.download = "morphio-recipe.toml";
        link.click();
        URL.revokeObjectURL(url);
        setStatus("ready", "Recipe exported.");
    } catch (error) {
        setStatus("error", `Failed to export recipe: ${formatError(error)}`);
    }
}

function applyRecipe(recipe) {
    elements.wordMatchStart.checked = recipe.options.word_match_start;
    elements.wordMatchEnd.checked = recipe.options.word_match_end;
    elements.skipMissingGlyphs.checked = recipe.options.skip_missing_glyphs;
    elements.ruleList.innerHTML = "";

    if (recipe.rules.length === 0) {
        addRuleRowWithValues("banana", "orange", true);
    } else {
        for (const [index, rule] of recipe.rules.entries()) {
            addRuleRowWithValues(rule[0], rule[1], index === 0);
        }
    }
}

mirrorPreviewText();
boot();
