// To inspect the console logs, one has to open the popup's devtools
// (as opposed to the default webpage's devtools that is opened in chrome)
//
// There are some repeated requests to get lastDiagnosticCnt from content.js
// but performance wise it should be fine, and I don't think it deserves some refactor.

const COLORS = {
  green: '#2e7d32',
  red: '#c62828',
  yellow: '#f9a825',
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
function createRuleButtons(rulesContainer, rules) {
  const title = document.createElement("span");
  title.className = "title";
  title.textContent = "Rules";
  rulesContainer.appendChild(title);

  const table = document.createElement("table");
  const tbody = document.createElement("tbody");

  rules.sort((a, b) => a.code.localeCompare(b.code));
  rules.forEach(({ code, name, active }) => {
    const row = document.createElement("tr");

    // Create the cell for the rule button
    const buttonCell = document.createElement("td");
    buttonCell.className = "button-cell";
    const button = document.createElement("button");
    button.className = "rule-btn";
    if (!active) button.classList.add("inactive");
    button.dataset.rule = code;
    button.id = `${code.toLowerCase()}-toggle`;
    button.textContent = code;
    buttonCell.appendChild(button);

    // Show rule name on hover
    button.title = name;

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

function getRuleButtonFromCode(code) {
  return document.querySelector(`button[data-rule="${code}"]`);
}

// Update error counters in the popup
function updateRuleCounters() {
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
        const button = getRuleButtonFromCode(key);
        const row = button.closest("tr");
        const counterCell = row.querySelector(".counter-cell");

        const count = Number(response.lastDiagnosticCnt[key]);
        counterCell.textContent = count;
        counterCell.style.fontWeight = count > 0 ? "bold" : "normal";
      }
    });
  });
}

function runScanAndUpdateCounters() {
  chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
    if (tabs.length === 0) return;
    chrome.tabs.sendMessage(tabs[0].id, { action: "runScan" }, updateRuleCounters);
  });
}

// Save the selected color to Chrome storage and triggers a scan on the active tab.
function saveColor(colorPicker) {
  const selectedColor = colorPicker.value;
  chrome.storage.local.set({ selectedColor: selectedColor });
  chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
    if (tabs.length > 0) chrome.tabs.sendMessage(tabs[0].id, { action: "runScan" });
  });
}

document.addEventListener('DOMContentLoaded', async function() {
  const rules = await new Promise(resolve => {
    chrome.runtime.sendMessage({ action: "getRules" }, (response) => {
      resolve(response.rules);
    });
  });

  displayExtensionVersion();

  const rulesContainer = document.querySelector(".rules");
  createRuleButtons(rulesContainer, rules);

  updateRuleCounters();

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
        updateRuleCounters();
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
        updateRuleCounters();
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

        updateRuleCounters();
      });
    });
  });

  // * Button - TOGGLE ALL RULES
  let flipState = true;

  // Update flipState on popup load (in order to not show Disable All, when
  // all the rules were already disabled).
  chrome.storage.local.get('rules', (result) => {
    const rules = result.rules;
    const allDisabled = rules.every(rule => !rule.active);
    if (allDisabled) {
      flipRulesButton.textContent = "Enable All";
      flipState = false;
    } else {
      flipRulesButton.textContent = "Disable All";
      flipState = true;
    }
  });

  flipRulesButton.addEventListener("click", async function() {
    chrome.storage.local.get('rules', function(result) {
      const rules = result.rules;
      // Update button CSS
      for (const rule of rules) {
        const button = getRuleButtonFromCode(rule.code);
        button.classList.toggle("inactive", !flipState);
        rule.active = flipState;
      }
      chrome.storage.local.set({ rules });
    });
    flipRulesButton.textContent = flipState ? "Enable All" : "Disable All";
    flipState = !flipState;
    runScanAndUpdateCounters();
    showFeedback(flipState ? "\u{1F31E} Enabled all" : "\u{1F634} Disabled all", "green");
  });

  // * Button - RESET RULES
  resetRulesButton.addEventListener("click", async function() {
    chrome.runtime.sendMessage({ action: "getDefaultRules" }, (response) => {
      const rules = response.rules;
      // Update button CSS
      for (const rule of rules) {
        const button = getRuleButtonFromCode(rule.code);
        button.classList.toggle("inactive", !rule.active);
      }
      chrome.storage.local.set({ rules });
    });
    runScanAndUpdateCounters();
    showFeedback("\uD83D\uDD04 Rules reset to default", "green");
  });

  // Update popup CSS and local storage when clicking a rule button
  Array.from(ruleButtons).forEach(button => {
    button.addEventListener("click", async () => {
      const buttonRuleCode = button.dataset.rule;
      // Modify the class to apply the CSS (but not contents.js config object!).
      button.classList.toggle("inactive");
      // Get the current states from storage and update
      const active = !button.classList.contains("inactive");
      await chrome.runtime.sendMessage({ action: "setRule", code: buttonRuleCode, active });
      runScanAndUpdateCounters();
    });
  });
});
