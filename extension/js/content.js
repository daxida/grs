const WASM_MOD_URL = chrome.runtime.getURL('pkg/grs_wasm.js');

// Import Wasm module binding using dynamic import.
// https://github.com/theberrigan/rust-wasm-chrome-ext/blob/master/extension/js/content.js
const loadWasmModule = async () => {
  const mod = await import(WASM_MOD_URL);
  const isOk = await mod.default().catch((e) => {
    console.warn('Failed to init wasm module in content script. Probably CSP of the page has restricted wasm loading.', e);
    return null;
  });
  return isOk ? mod : null;
};

// TODO: Sync with popup.js
let DEFAULT_RULE_STATES = null;
let lastDiagnosticCnt = null;

function walk(node, iterNode) {
  if (node.nodeType === Node.TEXT_NODE) {
    iterNode(node);
  } else {
    node.childNodes.forEach((node) => walk(node, iterNode));
  }
}

loadWasmModule().then((mod) => {
  if (mod === null) {
    return;
  }

  // Init settings
  const codes = mod.rule_codes();
  DEFAULT_RULE_STATES = Object.fromEntries(codes.map(rule => [rule, true]));
  // console.log('Initialized DEFAULT_RULE_STATES to ', DEFAULT_RULE_STATES);

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

  function scanPage() {
    return new Promise((resolve) => {
      chrome.storage.local.get(["selectedColor", "ruleStates"], (data) => {
        const color = data.selectedColor || "#FFFF00"; // Default to yellow
        const ruleStates = data.ruleStates || DEFAULT_RULE_STATES;
        const counter = scanPageGo(color, ruleStates);
        resolve(counter);
      });
    });
  }

  function scanPageGo(color, ruleStates) {
    // console.log(`Running scanPage with color: ${color} and `, ruleStates);
    const cnt = new Map();

    const iterNode = (node) => {
      try {
        const diagnostics = mod.scan_text(node.textContent, ruleStates);
        for (const { kind } of diagnostics) {
          if (!cnt.has(kind)) {
            cnt.set(kind, 0);
          }
          cnt.set(kind, cnt.get(kind) + 1);
        }
        if (diagnostics.length > 0) {
          highlightNode(node, color, diagnostics);
        }
      } catch (e) {
        console.warn("Failed with text:", node.textContent, e);
      }
    }

    walk(document.body, iterNode);

    const sortedCnt = new Map(
      [...cnt.entries()].sort((a, b) => a[0].localeCompare(b[0]))
    );
    console.debug("Diagnostic counter", sortedCnt);

    // Store it so popup can display it
    lastDiagnosticCnt = sortedCnt;
    return sortedCnt;
  }

  function toMonotonic() {
    const iterNode = (node) => {
      node.textContent = mod.to_monotonic(node.textContent);
    }
    walk(document.body, iterNode);
  }

  function fixText() {
    chrome.storage.local.get(["ruleStates"], (data) => {
      const ruleStates = data.ruleStates || DEFAULT_RULE_STATES;
      const iterNode = (node) => {
        node.textContent = mod.fix(node.textContent, ruleStates);
      }
      walk(document.body, iterNode);
    });
  }

  // Message passing
  chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
    console.log(`[L] Received message: ${message.action}`, message);

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
        (async () => {
          removeHighlight();
          fixText();
          const previousDiagnosticCnt = lastDiagnosticCnt;
          const counter = await scanPage();

          const difCounter = new Map();
          for (const [key, prevValue] of previousDiagnosticCnt.entries()) {
            const currValue = counter.get(key) || 0;
            const diff = prevValue - currValue;
            if (diff > 0) {
              difCounter.set(key, diff);
            }
          }

          const serializedMap = Object.fromEntries(difCounter);
          sendResponse({ status: "runFix finished", counter: serializedMap });
        })();
        return true; // IMPORTANT: keep message channel open for async sendResponse

      case "setRule":
        removeHighlight();
        scanPage();
        break;

      case "getRuleSettings":
        sendResponse({ ruleSettings: DEFAULT_RULE_STATES })
        break;

      case "getLastDiagnosticCnt":
        // Use codes instead of rules as keys
        const serializedMap = Object.fromEntries(
          [...lastDiagnosticCnt.entries()].map(([key, value]) => {
            const transformedKey = key
              .split("_")
              .map(word => word.charAt(0))
              .join("")
              .toUpperCase();
            return [transformedKey, value];
          }));
        sendResponse({ lastDiagnosticCnt: serializedMap });
        break;

      default:
        console.warn("[L] Unknown action received:", message.action);
    }

    sendResponse({ status: `${message.action} finished` });
  });

  // console.log("Content script loaded!");
  // chrome.storage.local.clear(); // For testing without cache
  scanPage();
});

