const ALL_RULES = [
  "MDA", // MissingDoubleAccents
  "MAC", // MissingAccentCapital
  "DW",  // DuplicatedWord
  "AFN", // AddFinalN
  "RFN", // RemoveFinalN
  "OS",  // OutdatedSpelling
  "MA",  // MonosyllableAccented
  "MNA", // MultisyllableNotAccented
  "MS",  // MixedScripts
  "AC",  // AmbiguousChar
];
const DEFAULT_RULE_STATES = Object.fromEntries(ALL_RULES.map(rule => [rule, true]));

const WASM_MOD_URL = chrome.runtime.getURL('pkg/grs_wasm.js');

// Import Wasm module binding using dynamic import.
// "init" may fail if the current site CSP restricts the use of Wasm (e.g. any github.com page).
// In this case instantiate module in the background worker (see background.js) and use message passing.
const loadWasmModule = async () => {
  const mod = await import(WASM_MOD_URL);

  // default export is an init function
  const isOk = await mod.default().catch((e) => {
    console.warn('Failed to init wasm module in content script. Probably CSP of the page has restricted wasm loading.', e);
    return null;
  });

  return isOk ? mod : null;
};

function walk(node, iterNode) {
  if (node.nodeType === Node.TEXT_NODE) {
    iterNode(node);
  } else {
    node.childNodes.forEach((node) => walk(node, iterNode));
  }
}

function groupDiagnostics(diagnostics) {
  // Note that the highlighting logic could fail if two diagnostic ranges overlap.
  //
  // We can not simply iterate the diagnostics, instead we (partially) deal with 
  // overlap by highlighting once but showing all kinds at that range.
  const grouped = new Map();
  for (const { kind, range } of diagnostics) {
    const key = JSON.stringify(range);
    if (!grouped.has(key)) {
      grouped.set(key, []);
    }
    grouped.get(key).push(kind);
  }
  return grouped;
}

const SPAN_CLASS = "grs-highlight";

function highlightNode(node, color, diagnostics) {
  let modifiedText = node.textContent;
  let offset = 0;
  for (const [key, kindArray] of groupDiagnostics(diagnostics)) {
    const range = JSON.parse(key);
    const kinds = kindArray.join(", ");
    const start = range.start + offset;
    const end = range.end + offset;

    const style = `"background-color: ${color};"`;
    const textSlice = modifiedText.slice(start, end);
    const highlightedText = `<span class=${SPAN_CLASS} style=${style} title="${kinds}">${textSlice}</span>`;
    modifiedText = modifiedText.slice(0, start) + highlightedText + modifiedText.slice(end);
    offset += highlightedText.length - (end - start);
  }
  const fragment = document.createRange().createContextualFragment(modifiedText);
  node.replaceWith(fragment);
}

// https://github.com/brandon1024/find/blob/42806fcc53e8843564ae463e6b246003d3d7a085/content/highlighter.js#L333
function removeHighlight() {
  document.querySelectorAll(`.${SPAN_CLASS}`).forEach(span => {
    let parent = span.parentElement;

    while (span.firstChild) {
      parent.insertBefore(span.firstChild, span);
    }

    parent.removeChild(span);
    parent.normalize();
  });
}

async function scanPage() {
  const mod = await loadWasmModule();
  if (!mod) return;

  chrome.storage.local.get(["selectedColor", "ruleStates"], (data) => {
    const color = data.selectedColor || "#FFFF00"; // Default to yellow
    const ruleStates = data.ruleStates || DEFAULT_RULE_STATES; // Default to all true
    scanPageGo(mod, color, ruleStates);
  });
}

function scanPageGo(mod, color, ruleStates) {
  console.log(`Running scanPage with color: ${color}`);

  const { scan_text } = mod;
  const cnt = new Map();

  const iterNode = (node) => {
    try {
      const diagnostics = scan_text(node.textContent, ruleStates);
      for (const { kind, range, fix } of diagnostics) {
        if (!cnt.has(kind)) {
          cnt.set(kind, 0);
        }
        cnt.set(kind, cnt.get(kind) + 1);
      }
      if (diagnostics.length > 0) {
        highlightNode(node, color, diagnostics);
      }
    } catch (e) {
      console.log(e);
      console.log("Failed with text " + textContent);
    }
  }

  walk(document.body, iterNode);

  const sortedCnt = new Map(
    [...cnt.entries()].sort((a, b) => a[0].localeCompare(b[0]))
  );
  console.log("Diagnostic counter", sortedCnt);
}

async function toMonotonic() {
  const mod = await loadWasmModule();
  if (!mod) return;

  const { to_monotonic } = mod;
  const iterNode = (node) => {
    node.textContent = to_monotonic(node.textContent);
  }
  walk(document.body, iterNode);
}

async function fixText() {
  const mod = await loadWasmModule();
  if (!mod) return;

  chrome.storage.local.get(["ruleStates"], (data) => {
    const ruleStates = data.ruleStates || DEFAULT_RULE_STATES;
    const { fix } = mod;
    const iterNode = (node) => {
      node.textContent = fix(node.textContent, ruleStates);
    }
    walk(document.body, iterNode);
  });
}

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  console.log("Received message:", message);
  console.log(`[L] Running ${message.action}...`);

  switch (message.action) {
    case "runScan":
      removeHighlight();
      scanPage();
      break;

    case "runToMono":
      removeHighlight();
      toMonotonic();
      scanPage();
      break;

    case "runFix":
      removeHighlight();
      fixText();
      scanPage();
      break;

    case "setRule":
      removeHighlight();
      scanPage();
      break;

    default:
      console.warn("[L] Unknown action received:", message.action);
  }

  sendResponse({ status: `${message.action} finished` });
});

console.log("Content script loaded!");
// chrome.storage.local.clear(); // For testing without cache
scanPage();
