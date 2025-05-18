// To inspect the console logs, one has to open the popup's devtools
// (as opposed to the default webpage's devtools that is opened in chrome)
//
// There are some repeated requests to get lastDiagnosticCnt from content.js
// but performance wise it should be fine, and I don't think it deserves some refactor.


// TODO: get defaults from wasm && sync with content.js
const defaultRuleStates = {
  "MDA": true,
  "OS": true,
  "MA": true,
  "MNA": true,
};

const COLORS = {
  green: '#4CAF50',
  red: '#b22222',
  yellow: '#e6b800',
  black: '#000000',
};

// Display a simple feedback message
function showFeedback(message, colorName = 'black') {
  const feedback = document.getElementById('feedback-msg');
  const color = COLORS[colorName.toLowerCase()] || COLORS.black;
  feedback.textContent = message;
  feedback.style.color = color;

  clearTimeout(showFeedback.timeoutId);
  showFeedback.timeoutId = setTimeout(() => {
    feedback.textContent = '';
  }, 1500); // 1.5 seconds timeout
}

// Function to fetch and display the extension version from manifest.json
function displayExtensionVersion() {
  const versionElement = document.getElementById('version');
  const manifest = chrome.runtime.getManifest();
  if (versionElement && manifest.version) {
    versionElement.textContent = `v.${manifest.version}`;
  }
}

function initCounterDisplay(counterElement) {
  counterElement.textContent = "0";
  counterElement.style.fontWeight = "normal";
  counterElement.style.fontSize = "1.1em";
}

// Dynamically create rule buttons and append them to the given container.
function createRuleButtons(rulesContainer, ruleCodes) {
  const title = document.createElement("span");
  title.className = "title";
  title.textContent = "Rules";
  rulesContainer.appendChild(title);

  const table = document.createElement("table");
  const tbody = document.createElement("tbody");

  ruleCodes.sort();
  ruleCodes.forEach(code => {
    const row = document.createElement("tr");

    // Create the cell for the rule button
    const buttonCell = document.createElement("td");
    buttonCell.className = "button-cell";
    const button = document.createElement("button");
    button.className = "rule-btn";
    button.dataset.rule = code;
    button.id = `${code.toLowerCase()}-toggle`;
    button.textContent = code;
    buttonCell.appendChild(button);

    // Create the cell for the counter
    const counterCell = document.createElement("td");
    counterCell.className = "counter-cell";
    const counter = document.createElement("span");
    counter.className = "counter";
    initCounterDisplay(counter);
    counterCell.appendChild(counter);

    row.appendChild(buttonCell);
    row.appendChild(counterCell);
    tbody.appendChild(row);
  });

  table.appendChild(tbody);
  rulesContainer.appendChild(table);
}

function updateRuleButtonCounters() {
  chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
    if (tabs.length === 0) return;
    chrome.tabs.sendMessage(tabs[0].id, { action: "getLastDiagnosticCnt" }, (response) => {
      if (chrome.runtime.lastError) return;
      if (!response || !response.lastDiagnosticCnt) return;

      // Reset counterCells
      document.querySelectorAll(".counter-cell").forEach(counterCell => {
        initCounterDisplay(counterCell);
      });

      for (const key in response.lastDiagnosticCnt) {
        const button = document.querySelector(`button[data-rule="${key}"]`);
        const row = button.closest("tr");
        const counterCell = row.querySelector(".counter-cell");

        const count = Number(response.lastDiagnosticCnt[key]);
        counterCell.textContent = count;
        counterCell.style.fontWeight = count > 0 ? "bold" : "normal";
      }
    });
  });
}

// Save the selected color to Chrome storage and triggers a scan on the active tab.
function saveColor(colorPicker) {
  const selectedColor = colorPicker.value;
  chrome.storage.local.set({ selectedColor: selectedColor });
  chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
    if (tabs.length === 0) return;
    chrome.tabs.sendMessage(tabs[0].id, { action: "runScan" });
  });
}

// Fetch rule settings from content.js
async function loadRuleStateFromContentScript() {
  return new Promise((resolve, reject) => {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      if (tabs.length === 0) return reject('No active tabs found.');
      chrome.tabs.sendMessage(tabs[0].id, { action: 'getRuleSettings' }, (response) => {
        resolve(response?.ruleSettings || defaultRuleStates);
      });
    });
  });
}

// Load rule states from localStorage, or fallback to content.js
async function loadRuleState() {
  return new Promise((resolve) => {
    chrome.storage.local.get('ruleStates', async function(result) {
      if (result.ruleStates) {
        resolve(result.ruleStates);
      } else {
        const ruleStates = await loadRuleStateFromContentScript();
        chrome.storage.local.set({ ruleStates });
        resolve(ruleStates);
      }
    });
  });
}

// Wrapped into a Promise to guarantee completion.
function sendSetRuleMessage(rule) {
  return new Promise((resolve, reject) => {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      if (tabs.length === 0) reject("No active tab found");
      chrome.tabs.sendMessage(tabs[0].id, { action: "setRule", rule: rule }, (response) => {
        resolve(response);
      });
    });
  });
}

document.addEventListener('DOMContentLoaded', async function() {
  const ruleState = await loadRuleState();

  displayExtensionVersion();

  // console.log(ruleState);
  ruleCodes = Object.keys(ruleState);
  const rulesContainer = document.querySelector(".rules");
  createRuleButtons(rulesContainer, ruleCodes);

  updateRuleButtonCounters();

  const colorPicker = document.getElementById('color-picker');
  const checkButton = document.getElementById("check-btn");
  const toMonotonicButton = document.getElementById("to-monotonic-btn");
  const fixButton = document.getElementById("fix-btn");
  const ruleButtons = document.getElementsByClassName("rule-btn");
  const flipRulesButton = document.getElementById("flip-all-rules-btn");
  const resetRulesButton = document.getElementById("reset-all-rules-btn");
  let debounceTimer;

  // COLORS
  // Load the stored color if it exists
  chrome.storage.local.get('selectedColor', function(result) {
    if (result.selectedColor) {
      colorPicker.value = result.selectedColor;
    }
  });

  // Debounced function to save color once user stops input
  colorPicker.addEventListener("input", function() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      saveColor(colorPicker);
      showFeedback(`Changed color to ${colorPicker.value}`, "green");
    }, 500);
  });

  // *** Buttons ***
  // * Button - CHECK
  checkButton.addEventListener("click", () => {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      if (tabs.length === 0) return;
      chrome.tabs.sendMessage(tabs[0].id, { action: "runScan" }, () => {
        if (chrome.runtime.lastError) return;
        updateRuleButtonCounters();
      });

      chrome.tabs.sendMessage(tabs[0].id, { action: "getLastDiagnosticCnt" }, (response) => {
        if (chrome.runtime.lastError) return;
        if (!response || !response.lastDiagnosticCnt) return;
        const counterMap = new Map(Object.entries(response.lastDiagnosticCnt));
        const numErrors = [...counterMap.values()].reduce((acc, val) => acc + Number(val), 0);
        if (numErrors > 0) {
          showFeedback(`Found ${numErrors} error(s)`, "green");
        } else {
          showFeedback("\u{2705} Found no errors", "green");
        }
      });
    });
  });

  // * Button - MONOTONIC
  toMonotonicButton.addEventListener("click", () => {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      if (tabs.length === 0) return;
      chrome.tabs.sendMessage(tabs[0].id, { action: "runToMono" }, () => {
        if (chrome.runtime.lastError) return;
        showFeedback("Converted to monotonic", "green");
      });

      chrome.tabs.sendMessage(tabs[0].id, { action: "runScan" }, () => {
        if (chrome.runtime.lastError) return;
        updateRuleButtonCounters();
      });
    });
  });

  // * Button - FIX
  fixButton.addEventListener("click", () => {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      if (tabs.length === 0) return;
      chrome.tabs.sendMessage(tabs[0].id, { action: "runFix" }, (response) => {
        if (chrome.runtime.lastError) return;

        if (response && response.counter) {
          const counterMap = new Map(Object.entries(response.counter));
          const numErrors = [...counterMap.values()].reduce((acc, val) => acc + Number(val), 0);
          const finalS = numErrors > 1 ? "s" : "";
          if (numErrors > 0) {
            showFeedback(`Fixed ${numErrors} error${finalS}`, "green");
          } else {
            showFeedback("No fixable errors found", "yellow");
          }
        } else {
          showFeedback("No errors found", "yellow");
        }

        updateRuleButtonCounters();
      });
    });
  });

  // * Button - TOGGLE ALL RULES
  let flipState = true;

  // Update flipState on popup load (in order to not show Disable All, when
  // all the rules where already disabled beforehand).
  chrome.storage.local.get('ruleStates', (result) => {
    const ruleStates = result.ruleStates || DEFAULT_RULE_STATES;
    const allDisabled = Object.values(ruleStates).every(state => state === false);
    if (allDisabled) {
      flipRulesButton.textContent = "Enable All";
      flipState = false;
    } else {
      flipRulesButton.textContent = "Disable All";
      flipState = true;
    }
  });

  flipRulesButton.addEventListener("click", async function() {
    chrome.storage.local.get('ruleStates', function(result) {
      const ruleStates = result.ruleStates || ruleState;
      for (const rule in ruleStates) {
        ruleStates[rule] = flipState;
        // Update button CSS
        const button = document.querySelector(`button[data-rule="${rule}"]`);
        button.classList.toggle("inactive", !flipState);
      }
      chrome.storage.local.set({ ruleStates: ruleStates });
    });
    updateToggleButtonText(flipRulesButton, flipState);
    flipState = !flipState;
    updateRuleButtonCounters();
    showFeedback(flipState ? "\u{1F31E} Enabled all" : "\u{1F634} Disabled all", "green");
  });

  function updateToggleButtonText(button, state) {
    button.textContent = state ? "Enable All" : "Disable All";
  }

  // * Button - RESET RULES
  resetRulesButton.addEventListener("click", async function() {
    chrome.storage.local.get('ruleStates', function(result) {
      const ruleStates = result.ruleStates || ruleState;
      for (const rule in ruleStates) {
        ruleStates[rule] = defaultRuleStates[rule] || false;
        // Update button CSS
        const button = document.querySelector(`button[data-rule="${rule}"]`);
        button.classList.toggle("inactive", !ruleStates[rule]);
      }
      chrome.storage.local.set({ ruleStates: ruleStates });
    });
    updateRuleButtonCounters();
    showFeedback("\uD83D\uDD04 Rules reset to default", "green");
  });

  // RULES
  // Update popup CSS with local storage
  chrome.storage.local.get('ruleStates', function(result) {
    const ruleStates = result.ruleStates || ruleState;
    Array.from(ruleButtons).forEach(button => {
      const rule = button.dataset.rule;
      if (ruleStates[rule] === false) {
        button.classList.add("inactive");
      }
    });
  });

  // Update popup CSS and local storage when clicking a rule button
  Array.from(ruleButtons).forEach(button => {
    button.addEventListener("click", async () => {
      const rule = button.dataset.rule;
      // Modify the class to apply the CSS (but not contents.js config object!).
      button.classList.toggle("inactive");
      // Get the current states from storage and update
      chrome.storage.local.get('ruleStates', function(result) {
        const ruleStates = result.ruleStates || ruleState;
        ruleStates[rule] = !button.classList.contains("inactive");
        console.log(ruleStates);
        chrome.storage.local.set({ ruleStates: ruleStates });
      });
      // Modify contents.js config and scan the page again.
      await sendSetRuleMessage();
      // Finally, update the rule button counters.
      updateRuleButtonCounters();
    });
  });
});
