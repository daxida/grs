// To inspect the console logs, one has to open the popup's devtools
// (as opposed to the default webpage's devtools that is opened in chrome)

function handleError(response) {
  console.log("[POPUP] Response status:", response?.status);
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
    counter.textContent = "0";
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
      if (!response || !response.lastDiagnosticCnt) {
        // console.warn("No diagnostic count received.");
        return;
      }

      // First clear the counters by setting them to 0
      document.querySelectorAll(".counter-cell").forEach(counter => {
        counter.textContent = "0";
      });

      for (const key in response.lastDiagnosticCnt) {
        // Find the button corresponding to the rule
        const button = document.querySelector(`button[data-rule="${key}"]`);
        // Locate the parent row (tr) of the button
        const row = button.closest("tr");
        const counterCell = row.querySelector(".counter-cell");
        counterCell.textContent = response.lastDiagnosticCnt[key];
        console.log("Updated countercell with ", response.lastDiagnosticCnt[key]);
      }
      console.log("Updated button counters");
    });
  });
}

// Save the selected color to Chrome storage and triggers a scan on the active tab.
function saveColor(colorPicker) {
  const selectedColor = colorPicker.value;
  chrome.storage.local.set({ selectedColor: selectedColor });
  chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
    if (tabs.length === 0) return;
    chrome.tabs.sendMessage(tabs[0].id, { action: "runScan" }, handleError);
  });
}

// Fetch rule settings from content.js
async function loadRuleStateFromContentScript() {
  return new Promise((resolve, reject) => {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      if (tabs.length === 0) return reject('No active tabs found.');
      chrome.tabs.sendMessage(tabs[0].id, { action: 'getRuleSettings' }, (response) => {
        console.log('Received response from content:', response);
        resolve(response.ruleSettings);
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

  // console.log(ruleState);
  ruleCodes = Object.keys(ruleState);
  const rulesContainer = document.querySelector(".rules");
  createRuleButtons(rulesContainer, ruleCodes);

  updateRuleButtonCounters();

  const colorPicker = document.getElementById('color-picker');
  const toMonotonicButton = document.getElementById("to-monotonic-btn");
  const fixButton = document.getElementById("fix-btn");
  const ruleButtons = document.getElementsByClassName("rule-btn");
  const flipRulesBtn = document.getElementById("flip-all-rules-btn");
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
    debounceTimer = setTimeout(() => saveColor(colorPicker), 500);
  });

  // *** Buttons ***
  // * Button - MONOTONIC
  // NOTE: This does not update button counters
  toMonotonicButton.addEventListener("click", () => {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      if (tabs.length === 0) return;
      chrome.tabs.sendMessage(tabs[0].id, { action: "runToMono" }, handleError)
    });
  });

  // * Button - FIX
  // NOTE: This does not update button counters
  //
  // FIXME: Should update really
  fixButton.addEventListener("click", () => {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      if (tabs.length === 0) return;
      chrome.tabs.sendMessage(tabs[0].id, { action: "runFix" }, handleError)
    });
  });

  // * Button - TOGGLE ALL RULES
  let flipState = true;
  flipRulesBtn.addEventListener("click", async function() {
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
    updateToggleButtonText(flipRulesBtn, flipState);
    flipState = !flipState;
    updateRuleButtonCounters();
  });

  function updateToggleButtonText(button, state) {
    button.textContent = state ? "Enable All" : "Disable All";
  }

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
