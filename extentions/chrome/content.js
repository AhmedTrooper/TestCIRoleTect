// content.js: Runs inside the webpage context to read the DOM safely (Chrome)
if (typeof window.roletectInjected === 'undefined') {
  window.roletectInjected = true;

chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.action === "GET_DOM") {
    try {
      // Use querySelectorAll to catch multiple distinct sections
      const targetElements = document.querySelectorAll(request.selector);

      if (targetElements.length === 0) {
        sendResponse({
          success: false,
          error: `Selector '${request.selector}' not found on this page.`,
        });
        return true;
      }

      // Process each element found and join their results
      const allExtractedData = Array.from(targetElements)
        .map(el => extractStructuredData(el, request.excludeSelector))
        .filter(text => text.length > 0)
        .join(". ");

      // Final pass to ensure no double periods or mess from joining
      const finalCleanData = allExtractedData
        .replace(/\.{2,}/g, ".")
        .replace(/\. \./g, ".")
        .trim();

      sendResponse({
        success: true,
        url: window.location.href,
        html: finalCleanData,
      });
    } catch (err) {
      sendResponse({ success: false, error: err.message });
    }
    return true; 
  }
  if (request.action === "ENABLE_INTERACTIVE") {
    currentExcludeSelector = request.excludeSelector || null;
    enableInteractiveMode();
    sendResponse({ success: true });
    return true;
  }
});

let currentExcludeSelector = null;
let interactiveActive = false;
let hoveredElement = null;
let originalOutline = '';
let originalCursor = '';
let originalTransition = '';

function enableInteractiveMode() {
  if (interactiveActive) return;
  interactiveActive = true;

  const overlay = document.createElement('div');
  overlay.id = 'roletect-interactive-overlay';
  overlay.style.position = 'fixed';
  overlay.style.top = '10px';
  overlay.style.left = '50%';
  overlay.style.transform = 'translateX(-50%)';
  overlay.style.background = '#238636';
  overlay.style.color = '#fff';
  overlay.style.padding = '8px 16px';
  overlay.style.borderRadius = '20px';
  overlay.style.zIndex = '2147483647';
  overlay.style.fontFamily = 'sans-serif';
  overlay.style.fontSize = '14px';
  overlay.style.pointerEvents = 'none';
  overlay.style.boxShadow = '0 4px 12px rgba(0,0,0,0.3)';
  overlay.textContent = 'RoleTect: Click any element to ingest. Press ESC to cancel.';
  document.body.appendChild(overlay);

  document.addEventListener('mouseover', handleMouseOver, true);
  document.addEventListener('mouseout', handleMouseOut, true);
  document.addEventListener('click', handleClick, true);
  document.addEventListener('keydown', handleKeyDown, true);
}

function disableInteractiveMode() {
  if (!interactiveActive) return;
  interactiveActive = false;
  
  const overlay = document.getElementById('roletect-interactive-overlay');
  if (overlay) overlay.remove();

  if (hoveredElement) {
    hoveredElement.style.outline = originalOutline;
    hoveredElement.style.cursor = originalCursor;
    hoveredElement.style.transition = originalTransition;
    hoveredElement = null;
  }

  document.removeEventListener('mouseover', handleMouseOver, true);
  document.removeEventListener('mouseout', handleMouseOut, true);
  document.removeEventListener('click', handleClick, true);
  document.removeEventListener('keydown', handleKeyDown, true);
}

function handleMouseOver(e) {
  if (!interactiveActive) return;
  if (hoveredElement) {
    hoveredElement.style.outline = originalOutline;
    hoveredElement.style.cursor = originalCursor;
    hoveredElement.style.transition = originalTransition;
  }
  hoveredElement = e.target;
  originalOutline = hoveredElement.style.outline;
  originalCursor = hoveredElement.style.cursor;
  originalTransition = hoveredElement.style.transition;
  
  hoveredElement.style.transition = 'outline 0.1s ease';
  hoveredElement.style.outline = '3px solid #238636';
  hoveredElement.style.cursor = 'crosshair';
}

function handleMouseOut(e) {
  if (!interactiveActive) return;
  if (e.target === hoveredElement) {
    hoveredElement.style.outline = originalOutline;
    hoveredElement.style.cursor = originalCursor;
    hoveredElement.style.transition = originalTransition;
    hoveredElement = null;
  }
}

function handleClick(e) {
  if (!interactiveActive) return;
  e.preventDefault();
  e.stopPropagation();

  const target = e.target;
  disableInteractiveMode();

  const extracted = extractStructuredData(target, currentExcludeSelector);
  
  if (!extracted || extracted.length < 5) {
    alert("RoleTect: No valid text could be extracted from this element.");
    return;
  }

  // Visual feedback
  const originalBackground = target.style.backgroundColor;
  target.style.transition = 'background-color 0.5s';
  target.style.backgroundColor = 'rgba(35, 134, 54, 0.4)';
  setTimeout(() => {
    target.style.backgroundColor = originalBackground;
  }, 1000);

  chrome.runtime.sendMessage({
    action: "PROCESS_INTERACTIVE_SELECTION",
    content: extracted,
    url: window.location.href
  });
}

function handleKeyDown(e) {
  if (e.key === 'Escape' && interactiveActive) {
    disableInteractiveMode();
  }
}

/**
 * Cleans the DOM node and extracts a highly compressed, token-efficient string.
 */
function extractStructuredData(element, userExcludeSelector) {
  // 1. Clone the element so we don't accidentally mutate the live webpage
  const clone = element.cloneNode(true);

  // 2. Remove noisy and non-text tags that confuse AI and bloat the payload
  const noiseSelectors =
    "script, style, noscript, svg, img, iframe, nav, footer, button, .visually-hidden, audio, video, picture, source, track, embed, object, param, canvas, math, map, area, progress, meter, datalist, output";
  clone.querySelectorAll(noiseSelectors).forEach((el) => el.remove());

  // 3. Remove user-defined excluded elements if provided
  if (userExcludeSelector) {
    try {
      // If the cloned element itself matches the exclude selector, drop it entirely
      if (clone.matches && clone.matches(userExcludeSelector)) {
        return "";
      }
      // Otherwise, remove any matching child elements
      clone.querySelectorAll(userExcludeSelector).forEach((el) => el.remove());
    } catch (e) {
      console.error("Invalid user exclude selector:", userExcludeSelector);
    }
  }

  // 4. Ensure blocks don't get smashed together by adding newlines before block elements
  clone.querySelectorAll("div, p, br, h1, h2, h3, h4, h5, h6, li, article, section, main, header, tr, td").forEach(el => {
    el.prepend(document.createTextNode("\n"));
  });

  // 5. Extract the remaining text using innerText or textContent as fallback
  let finalString = clone.innerText || clone.textContent || "";

  // 6. The Ultimate Token-Squashing RegEx Pipeline
  return finalString
    .replace(/[\n\r]+/g, ". ") // Turns ANY newline or carriage return into a period
    .replace(/\s+/g, " ") // Squashes massive horizontal gaps into 1 single space
    .replace(/\.{2,}/g, ".") // Squashes weird double periods (e.g. "...") into 1 period
    .replace(/\. \./g, ".") // Cleans up messy period-space-period gaps
    .trim(); // Chops off any spaces at the very beginning or end
}

} // End of injection guard
