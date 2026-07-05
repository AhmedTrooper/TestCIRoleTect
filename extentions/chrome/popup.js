// Initialize UI and Load Settings
document.addEventListener('DOMContentLoaded', () => {
  const hostTypeSelect = document.getElementById('hostType');
  const localPortInput = document.getElementById('localPort');
  const customHostInput = document.getElementById('customHost');
  const customPortInput = document.getElementById('customPort');
  const remoteUrlInput = document.getElementById('remoteUrl');
  
  const secretInput = document.getElementById('secret');
  const selectorInput = document.getElementById('selector');
  const excludeSelectorInput = document.getElementById('excludeSelector');
  const statusDiv = document.getElementById('status');

  // Advanced Mode Elements
  const modeToggle = document.getElementById('modeToggle');
  const simpleInputArea = document.getElementById('simpleInputArea');
  const advancedInputArea = document.getElementById('advancedInputArea');
  const siteSelector = document.getElementById('siteSelector');

  // Site Management Elements
  const siteMapList = document.getElementById('siteMapList');
  const newSiteTitle = document.getElementById('newSiteTitle');
  const newSiteSelector = document.getElementById('newSiteSelector');
  const newSiteExclude = document.getElementById('newSiteExclude');
  const addSiteBtn = document.getElementById('addSiteBtn');

  // Backup Elements
  const exportBtn = document.getElementById('exportBtn');
  const importFile = document.getElementById('importFile');
  const restoreOptions = document.getElementById('restoreOptions');
  const safeRestoreBtn = document.getElementById('safeRestoreBtn');
  const unsafeRestoreBtn = document.getElementById('unsafeRestoreBtn');

  let importedData = null;
  let siteMaps = [];

  // Built-in Defaults
  const BUILT_IN_SITES = [
    { title: "LinkedIn", selector: ".jobs-search__job-details--wrapper" },
    { title: "Indeed", selector: "#jobDescriptionText" },
    { title: "Glassdoor", selector: ".JobDetails_jobDescription__uW_fK" },
    { title: "Wellfound (AngelList)", selector: ".styles_jobDescription__xL_qW" },
    { title: "Y Combinator (WnYC)", selector: ".job-description" },
    { title: "Greenhouse", selector: "#content" },
    { title: "Lever", selector: ".section-wrapper.page-full-width" },
    { title: "Workday", selector: "[data-automation-id='jobPostingDescription']" }
  ];

  const hostGroups = {
    localhost: document.getElementById('localhostGroup'),
    customLocal: document.getElementById('customLocalGroup'),
    remote: document.getElementById('remoteGroup')
  };

  // Tab Switching
  document.querySelectorAll('.tab').forEach(tab => {
    tab.addEventListener('click', () => {
      document.querySelectorAll('.tab, .tab-content').forEach(el => el.classList.remove('active'));
      tab.classList.add('active');
      document.getElementById(tab.dataset.tab).classList.add('active');

      // Reload settings on tab change to discard unsaved changes
      loadSettings();
    });
  });

  async function loadSettings() {
    const result = await chrome.storage.local.get([
      'hostType', 'localPort', 'customHost', 'customPort', 'remoteUrl', 'secret', 'selector', 'excludeSelector',
      'uiMode', 'siteMaps', 'activeSiteIndex'
    ]);

    if (result.hostType) {
      hostTypeSelect.value = result.hostType;
      updateHostGroups(result.hostType);
    }
    if (result.localPort) localPortInput.value = result.localPort;
    if (result.customHost) customHostInput.value = result.customHost;
    if (result.customPort) customPortInput.value = result.customPort;
    if (result.remoteUrl) remoteUrlInput.value = result.remoteUrl;
    if (result.secret) secretInput.value = result.secret;
    if (result.selector) selectorInput.value = result.selector;
    if (result.excludeSelector) excludeSelectorInput.value = result.excludeSelector;

    // Mode logic
    const isAdvanced = result.uiMode === 'advanced';
    modeToggle.checked = isAdvanced;
    updateModeUI(isAdvanced);

    // Site Maps logic
    siteMaps = result.siteMaps || [];
    renderSiteMaps();
    
    if (result.activeSiteIndex !== undefined) {
      siteSelector.value = result.activeSiteIndex;
    }
  }

  // Mode Toggle Logic
  modeToggle.addEventListener('change', async () => {
    const isAdvanced = modeToggle.checked;
    updateModeUI(isAdvanced);
    await chrome.storage.local.set({ uiMode: isAdvanced ? 'advanced' : 'simple' });
  });

  function updateModeUI(isAdvanced) {
    simpleInputArea.style.display = isAdvanced ? 'none' : 'block';
    advancedInputArea.style.display = isAdvanced ? 'block' : 'none';
  }

  function escapeHTML(str) {
    return String(str).replace(/[&<>'"]/g, tag => ({
      '&': '&amp;',
      '<': '&lt;',
      '>': '&gt;',
      "'": '&#39;',
      '"': '&quot;'
    }[tag]));
  }

  function renderSiteMaps() {
    // Update Select Dropdown in Extract Tab
    siteSelector.innerHTML = siteMaps.length > 0 
      ? siteMaps.map((site, index) => `<option value="${index}">${escapeHTML(site.title)}</option>`).join('')
      : '<option value="">-- No Sites Saved --</option>';

    // Update List in Settings Tab
    let listHtml = siteMaps.map((site, index) => {
      const defaultSite = BUILT_IN_SITES.find(s => s.title === site.title);
      const isModified = defaultSite && (defaultSite.selector !== site.selector || defaultSite.exclude !== site.exclude);
      
      return `
        <div class="site-map-item" style="border-bottom: 1px solid rgba(255,255,255,0.05); padding-bottom: 8px;">
          <div style="flex-grow: 1;">
            <div style="display: flex; align-items: center; gap: 5px;">
              <strong style="font-size: 11px;">${escapeHTML(site.title)}</strong>
              ${defaultSite ? '<span style="font-size: 8px; background: var(--accent); color: white; padding: 1px 4px; border-radius: 3px;">Built-in</span>' : ''}
              ${isModified ? '<span style="font-size: 8px; background: #e3b341; color: black; padding: 1px 4px; border-radius: 3px;">Modified</span>' : ''}
            </div>
            <code style="display: block; font-size: 9px; color: var(--muted); margin-top: 2px;">IN: ${escapeHTML(site.selector)}</code>
            ${site.exclude ? `<code style="display: block; font-size: 9px; color: #f85149; margin-top: 2px;">EX: ${escapeHTML(site.exclude)}</code>` : ''}
          </div>
          <div style="display: flex; flex-direction: column; gap: 4px;">
            <button class="edit-btn secondary-btn" data-index="${index}" style="font-size: 9px !important; padding: 2px 5px !important; border-color: var(--accent) !important;">Edit</button>
            <button class="delete-btn" data-index="${index}">Delete</button>
            ${defaultSite ? `<button class="reset-site-btn secondary-btn" style="font-size: 9px !important; padding: 2px 5px !important;" data-title="${site.title}" data-index="${index}">Reset</button>` : ''}
          </div>
        </div>
      `;
    }).join('');

    // Add "Load Built-ins" button if missing
    const missingBuiltIns = BUILT_IN_SITES.filter(b => !siteMaps.find(s => s.title === b.title));
    if (missingBuiltIns.length > 0) {
      listHtml += `
        <button id="loadBuiltInsBtn" class="secondary-btn" style="width: 100%; margin-top: 10px; font-size: 11px;">
          Load ${missingBuiltIns.length} Default Templates
        </button>
      `;
    }

    if (siteMaps.length > 0) {
      listHtml += `
        <button id="factoryResetBtn" class="secondary-btn" style="width: 100%; margin-top: 10px; font-size: 11px; border-color: #f8514944 !important; color: #f85149 !important;">
          Factory Reset All Sites
        </button>
      `;
    }

    siteMapList.innerHTML = listHtml;

    // Attach Events
    document.querySelectorAll('.delete-btn').forEach(btn => {
      btn.addEventListener('click', async (e) => {
        const index = parseInt(e.target.dataset.index);
        siteMaps.splice(index, 1);
        await saveSiteMaps();
      });
    });

    document.querySelectorAll('.reset-site-btn').forEach(btn => {
      btn.addEventListener('click', (e) => {
        const title = e.target.dataset.title;
        const index = parseInt(e.target.dataset.index);
        const defaultSite = BUILT_IN_SITES.find(s => s.title === title);
        if (defaultSite) {
          siteMaps[index].selector = defaultSite.selector;
          delete siteMaps[index].exclude;
          saveSiteMaps();
          showStatus(`${title} reset to default.`, "success");
        }
      });
    });

    document.querySelectorAll('.edit-btn').forEach(btn => {
      btn.addEventListener('click', (e) => {
        const index = parseInt(e.target.dataset.index);
        const site = siteMaps[index];
        newSiteTitle.value = site.title;
        newSiteSelector.value = site.selector;
        newSiteExclude.value = site.exclude || '';
        
        addSiteBtn.textContent = "Update Site Map";
        addSiteBtn.dataset.editIndex = index;
        newSiteTitle.focus();
      });
    });

    const loadBtn = document.getElementById('loadBuiltInsBtn');
    if (loadBtn) {
      loadBtn.addEventListener('click', async () => {
        missingBuiltIns.forEach(b => siteMaps.push({ ...b }));
        await saveSiteMaps();
        showStatus("Defaults loaded!", "success");
      });
    }

    const factoryBtn = document.getElementById('factoryResetBtn');
    if (factoryBtn) {
      factoryBtn.addEventListener('click', async () => {
        if (confirm("This will delete all custom sites and reset built-ins. Proceed?")) {
          siteMaps = BUILT_IN_SITES.map(s => ({ ...s }));
          await saveSiteMaps();
          showStatus("Factory reset complete.", "success");
        }
      });
    }
  }

  async function saveSiteMaps() {
    await chrome.storage.local.set({ siteMaps });
    renderSiteMaps();
  }

  // Add/Update Site Map
  addSiteBtn.addEventListener('click', () => {
    const title = newSiteTitle.value.trim();
    const selector = newSiteSelector.value.trim();
    const exclude = newSiteExclude.value.trim();

    if (!title || !selector) {
      showStatus("Please enter both Title and Selector.", "error");
      return;
    }

    if (addSiteBtn.dataset.editIndex !== undefined) {
      const index = parseInt(addSiteBtn.dataset.editIndex);
      siteMaps[index] = { title, selector, exclude };
      delete addSiteBtn.dataset.editIndex;
      addSiteBtn.textContent = "Add Site Map";
      showStatus("Site map updated!", "success");
    } else {
      siteMaps.push({ title, selector, exclude });
      showStatus("Site map added!", "success");
    }

    newSiteTitle.value = '';
    newSiteSelector.value = '';
    newSiteExclude.value = '';
    saveSiteMaps();
  });

  // Site Selector Change
  siteSelector.addEventListener('change', async () => {
    await chrome.storage.local.set({ activeSiteIndex: siteSelector.value });
  });

  // Export Logic
  exportBtn.addEventListener('click', async () => {
    const allData = await chrome.storage.local.get(null);
    const blob = new Blob([JSON.stringify(allData, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `roletect_backup_${new Date().toISOString().split('T')[0]}.json`;
    a.click();
    URL.revokeObjectURL(url);
    showStatus("Backup exported!", "success");
  });

  // Import Logic
  importFile.addEventListener('change', (e) => {
    const file = e.target.files[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (event) => {
      try {
        importedData = JSON.parse(event.target.result);
        restoreOptions.style.display = 'block';
        showStatus("JSON valid. Choose Restore method.", "neutral");
      } catch (err) {
        showStatus("Invalid JSON file.", "error");
      }
    };
    reader.readAsText(file);
  });

  safeRestoreBtn.addEventListener('click', async () => {
    if (!importedData) return;
    
    const result = await chrome.storage.local.get(['siteMaps']);
    const currentMaps = result.siteMaps || [];
    const incomingMaps = importedData.siteMaps || [];
    
    // Merge unique by title
    const mergedMaps = [...currentMaps];
    incomingMaps.forEach(inMap => {
      if (!mergedMaps.find(m => m.title === inMap.title)) {
        mergedMaps.push(inMap);
      }
    });

    await chrome.storage.local.set({ siteMaps: mergedMaps });
    showStatus("Safe Merge complete!", "success");
    restoreOptions.style.display = 'none';
    loadSettings();
  });

  unsafeRestoreBtn.addEventListener('click', async () => {
    if (!importedData) return;
    await chrome.storage.local.clear();
    await chrome.storage.local.set(importedData);
    showStatus("Overwrite complete!", "success");
    restoreOptions.style.display = 'none';
    loadSettings();
  });

  // GitHub Button Logic
  document.getElementById('githubIssueBtn').addEventListener('click', () => {
    const baseUrl = "https://github.com/AhmedTrooper/RoleTect/issues/new";
    const title = encodeURIComponent("[Site Template] Add support for [SITE NAME]");
    const body = encodeURIComponent("### Site Name\n(e.g. Indeed, Glassdoor)\n\n### Selector\n(e.g. .job-details-container)\n\n### Sample URL\n(optional link to a job post)");
    window.open(`${baseUrl}?title=${title}&body=${body}`, '_blank');
  });

  // Load saved settings initially
  loadSettings();

  // Save Selector
  document.getElementById('saveSelectorBtn').addEventListener('click', async () => {
    const selector = selectorInput.value.trim() || 'body';
    const excludeSelector = excludeSelectorInput.value.trim();
    await chrome.storage.local.set({ selector, excludeSelector });
    showStatus("Selectors saved!", "success");
  });

  // Reset Selector
  document.getElementById('resetSelectorBtn').addEventListener('click', async () => {
    selectorInput.value = 'body';
    excludeSelectorInput.value = '';
    await chrome.storage.local.set({ selector: 'body', excludeSelector: '' });
    showStatus("Selectors reset to default.", "neutral");
  });

  // Host Type Change Logic
  hostTypeSelect.addEventListener('change', (e) => {
    updateHostGroups(e.target.value);
  });

  function updateHostGroups(selectedType) {
    Object.keys(hostGroups).forEach(type => {
      hostGroups[type].style.display = (type === selectedType) ? 'block' : 'none';
    });
  }

  // Save Settings
  document.getElementById('saveSettingsBtn').addEventListener('click', async () => {
    const hostType = hostTypeSelect.value;
    const localPort = localPortInput.value.trim() || '14207';
    const customHost = customHostInput.value.trim();
    const customPort = customPortInput.value.trim() || '14207';
    const remoteUrl = remoteUrlInput.value.trim();
    const secret = secretInput.value.trim();

    let finalHost = '';

    if (hostType === 'localhost') {
      finalHost = `http://127.0.0.1:${localPort}`;
    } else if (hostType === 'customLocal') {
      const h = customHost || '127.0.0.1';
      finalHost = `http://${h}:${customPort}`;
    } else if (hostType === 'remote') {
      finalHost = remoteUrl;
      // Force HTTPS for remote URLs (security best practice)
      if (finalHost) {
        if (finalHost.startsWith('http://')) {
          finalHost = finalHost.replace('http://', 'https://');
        } else if (!finalHost.startsWith('https://')) {
          finalHost = 'https://' + finalHost;
        }
      }
    }

    // Remove trailing slash for consistency
    if (finalHost.endsWith('/')) {
      finalHost = finalHost.slice(0, -1);
    }

    if (!finalHost) {
      showStatus("Please provide a valid Host or URL.", "error");
      return;
    }

    const saveAction = async () => {
      await chrome.storage.local.set({ 
        host: finalHost, 
        hostType, 
        localPort, 
        customHost, 
        customPort, 
        remoteUrl, 
        secret 
      });
      showStatus("Settings saved successfully!", "success");
    };

    // If it's a remote URL, we must request permission dynamically
    if (hostType === 'remote' || (hostType === 'customLocal' && customHost !== '127.0.0.1' && customHost !== 'localhost')) {
      try {
        const origin = new URL(finalHost).origin + '/*';
        const granted = await chrome.permissions.request({
          origins: [origin]
        });
        if (granted) {
          await saveAction();
        } else {
          showStatus("Permission denied. Cannot save custom host.", "error");
        }
      } catch (e) {
        showStatus("Invalid URL format.", "error");
      }
    } else {
      await saveAction();
    }
  });

  // Extract and Send
  document.getElementById('extractBtn').addEventListener('click', async () => {
    let selector = 'body';
    let excludeSelector = '';

    if (modeToggle.checked) {
      const index = siteSelector.value;
      if (index !== "" && siteMaps[index]) {
        selector = siteMaps[index].selector;
        excludeSelector = siteMaps[index].exclude || '';
      } else {
        showStatus("No site template selected.", "error");
        return;
      }
    } else {
      selector = selectorInput.value.trim() || 'body';
      excludeSelector = excludeSelectorInput.value.trim();
    }
    
    showStatus("Extracting content...", "neutral");

    try {
      const response = await chrome.runtime.sendMessage({ action: "START_EXTRACTION", selector, excludeSelector });
      if (response && response.success) {
        showStatus("Job ingested into Inbox vault!", "success");
      } else {
        const errorMsg = response?.error || "Connection failed. Is your RoleTect instance reachable?";
        showStatus("Error: " + errorMsg, "error");
      }
    } catch (err) {
      showStatus("Error: " + err.message, "error");
    }
  });

  document.getElementById('interactiveBtn').addEventListener('click', async () => {
    let excludeSelector = '';
    if (modeToggle.checked) {
      const index = siteSelector.value;
      if (index !== "" && siteMaps[index]) {
        excludeSelector = siteMaps[index].exclude || '';
      }
    } else {
      excludeSelector = excludeSelectorInput.value.trim();
    }
    
    try {
      const response = await chrome.runtime.sendMessage({ action: "START_INTERACTIVE", excludeSelector });
      if (response && response.success) {
        window.close();
      } else {
        const errorMsg = response?.error || "Failed to start interactive mode.";
        showStatus("Error: " + errorMsg, "error");
      }
    } catch (err) {
      showStatus("Error: " + err.message, "error");
    }
  });

  function showStatus(msg, type) {
    statusDiv.textContent = msg;
    statusDiv.className = "";
    statusDiv.style.display = "block";
    
    // Apply highly visible toast colors
    if (type === "success") {
      statusDiv.style.background = "#238636";
      statusDiv.style.color = "#ffffff";
    } else if (type === "error") {
      statusDiv.style.background = "#da3633";
      statusDiv.style.color = "#ffffff";
    } else {
      statusDiv.style.background = "#30363d";
      statusDiv.style.color = "#ffffff";
    }

    // Auto-hide the toast after 3 seconds
    clearTimeout(window.statusTimeout);
    window.statusTimeout = setTimeout(() => {
      statusDiv.style.display = "none";
    }, 3000);
  }
});
