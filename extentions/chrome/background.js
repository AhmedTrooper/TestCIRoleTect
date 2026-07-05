// Service Worker: Handles network requests to the local RoleTect server
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.action === "START_EXTRACTION") {
    handleExtraction(request.selector, request.excludeSelector).then(sendResponse);
    return true; // Keep the messaging channel open for the async response
  }
  if (request.action === "START_INTERACTIVE") {
    handleInteractiveStart(request.excludeSelector).then(sendResponse);
    return true;
  }
  if (request.action === "PROCESS_INTERACTIVE_SELECTION") {
    handleInteractiveProcess(request).then(sendResponse);
    return true;
  }
});

async function handleInteractiveStart(excludeSelector) {
  try {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (!tab || !tab.id) throw new Error("No active tab found.");

    await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      files: ["content.js"]
    });

    const res = await chrome.tabs.sendMessage(tab.id, { action: "ENABLE_INTERACTIVE", excludeSelector });
    if (!res || !res.success) throw new Error(res?.error || "Failed to start interactive mode");

    return { success: true };
  } catch (error) {
    console.error("RoleTect Extension Error:", error);
    return { success: false, error: error.message };
  }
}

async function handleInteractiveProcess(request) {
  try {
    const settings = await chrome.storage.local.get(['host', 'secret']);
    const host = settings.host || 'http://127.0.0.1:14207';
    const secret = settings.secret;

    if (!secret) throw new Error("Secret Key missing. Please set it in Extension Settings.");

    const serverUrl = `${host}/inbox/ingest`;
    
    const serverResponse = await fetch(serverUrl, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({
        url: request.url,
        raw_description: request.content,
        secret: secret
      })
    });

    if (!serverResponse.ok) {
      const errorData = await serverResponse.json().catch(() => ({}));
      throw new Error(errorData.message || `Server rejected with status: ${serverResponse.status}`);
    }

    chrome.notifications.create({
      type: "basic",
      iconUrl: "icon.png",
      title: "RoleTect",
      message: "Job successfully ingested into Inbox vault!"
    });

    return { success: true };

  } catch (error) {
    console.error("RoleTect Extension Error:", error);
    chrome.notifications.create({
      type: "basic",
      iconUrl: "icon.png",
      title: "RoleTect Error",
      message: error.message
    });
    return { success: false, error: error.message };
  }
}

async function handleExtraction(selector, excludeSelector) {
  try {
    // 1. Get Host and Secret from storage
    const settings = await chrome.storage.local.get(['host', 'secret']);
    const host = settings.host || 'http://127.0.0.1:14207';
    const secret = settings.secret;

    if (!secret) {
      throw new Error("Secret Key missing. Please set it in Extension Settings.");
    }

    // 2. Find the active tab
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (!tab || !tab.id) throw new Error("No active tab found.");

    // 3. Inject content script (Self-contained in MV3)
    await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      files: ["content.js"]
    });

    // 4. Extract data from page
    const domData = await chrome.tabs.sendMessage(tab.id, {
      action: "GET_DOM",
      selector: selector,
      excludeSelector: excludeSelector
    });

    if (!domData.success) throw new Error(domData.error);

    // 5. POST to RoleTect server
    const serverUrl = `${host}/inbox/ingest`;
    
    const serverResponse = await fetch(serverUrl, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({
        url: domData.url,
        raw_description: domData.html, // Injected as raw_description
        secret: secret
      })
    });

    if (!serverResponse.ok) {
      const errorData = await serverResponse.json().catch(() => ({}));
      throw new Error(errorData.message || `Server rejected with status: ${serverResponse.status}`);
    }

    return { success: true };

  } catch (error) {
    console.error("RoleTect Extension Error:", error);
    return { success: false, error: error.message };
  }
}
