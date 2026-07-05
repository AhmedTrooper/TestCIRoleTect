// Background Script: Handles network requests to the local RoleTect server (Firefox)
browser.runtime.onMessage.addListener((request, sender) => {
  if (request.action === "START_EXTRACTION") {
    return handleExtraction(request.selector, request.excludeSelector);
  }
  if (request.action === "START_INTERACTIVE") {
    return handleInteractiveStart(request.excludeSelector);
  }
  if (request.action === "PROCESS_INTERACTIVE_SELECTION") {
    return handleInteractiveProcess(request);
  }
});

async function handleInteractiveStart(excludeSelector) {
  try {
    const tabs = await browser.tabs.query({ active: true, currentWindow: true });
    const tab = tabs[0];
    if (!tab || !tab.id) throw new Error("No active tab found.");

    await browser.scripting.executeScript({
      target: { tabId: tab.id },
      files: ["content.js"]
    });

    const res = await browser.tabs.sendMessage(tab.id, { action: "ENABLE_INTERACTIVE", excludeSelector });
    if (!res || !res.success) throw new Error(res?.error || "Failed to start interactive mode");

    return { success: true };
  } catch (error) {
    console.error("RoleTect Firefox Extension Error:", error);
    return { success: false, error: error.message };
  }
}

async function handleInteractiveProcess(request) {
  try {
    const settings = await browser.storage.local.get(['host', 'secret']);
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

    browser.notifications.create({
      type: "basic",
      iconUrl: "icon.png",
      title: "RoleTect",
      message: "Job successfully ingested into Inbox vault!"
    });

    return { success: true };

  } catch (error) {
    console.error("RoleTect Firefox Extension Error:", error);
    browser.notifications.create({
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
    const settings = await browser.storage.local.get(['host', 'secret']);
    const host = settings.host || 'http://127.0.0.1:14207';
    const secret = settings.secret;

    if (!secret) {
      throw new Error("Secret Key missing. Please set it in Extension Settings.");
    }

    // 2. Find the active tab
    const tabs = await browser.tabs.query({ active: true, currentWindow: true });
    const tab = tabs[0];
    if (!tab || !tab.id) throw new Error("No active tab found.");

    // 3. Inject content script
    await browser.scripting.executeScript({
      target: { tabId: tab.id },
      files: ["content.js"]
    });

    // 4. Extract data from page
    const domData = await browser.tabs.sendMessage(tab.id, {
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
        raw_description: domData.html,
        secret: secret
      })
    });

    if (!serverResponse.ok) {
      const errorData = await serverResponse.json().catch(() => ({}));
      throw new Error(errorData.message || `Server rejected with status: ${serverResponse.status}`);
    }

    return { success: true };

  } catch (error) {
    console.error("RoleTect Firefox Extension Error:", error);
    return { success: false, error: error.message };
  }
}
