import initWasmModule, { rules } from '../pkg/grs_wasm.js';


function getDefaultRules() {
  const wasmRules = rules();
  for (const activeRuleCode of ["MDA", "OS", "MA", "MNA"]) {
    const rule = wasmRules.find(r => r.code === activeRuleCode);
    rule.active = true;
  }
  return wasmRules;
}

// Cache and return rules
async function getRules() {
  return new Promise(resolve => {
    chrome.storage.local.get("rules", async (result) => {
      if (result.rules) {
        resolve(result.rules);
      } else {
        const defaultRules = getDefaultRules();
        chrome.storage.local.set({ rules: defaultRules });
        resolve(defaultRules);
      }
    });
  });
}

// Set the active state of a specific rule
function setRule(code, active) {
  chrome.storage.local.get("rules", (result) => {
    if (!result.rules) {
      throw new Error("Rules not loaded in storage. Call getRules() first.");
    }

    const rules = result.rules;
    const rule = rules.find(r => r.code === code);

    if (!rule) {
      throw new Error(`Rule with code "${code}" not found.`);
    }

    rule.active = active;
    chrome.storage.local.set({ rules });
  });
}

function handleMessage(msg, _sender, sendResponse) {
  switch (msg.action) {
    case "getRules":
      getRules().then(rules => sendResponse({ rules }));
      return true; // async sendResponse

    case "getDefaultRules":
      sendResponse({ rules: getDefaultRules() });
      return true;

    case "setRule":
      const { code, active } = msg;
      try {
        setRule(code, active);
        sendResponse({ success: true });
      } catch (err) {
        sendResponse({ success: false, error: err.message });
      }
      return true;

    default:
      console.debug("Unknown action received:", msg.action);
      return false;
  }
}

async function main() {
  await initWasmModule();

  // For testing without cache
  // chrome.storage.local.clear();

  await getRules();

  chrome.runtime.onMessage.addListener(handleMessage);
}

main();

