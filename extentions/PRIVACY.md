# Privacy Policy for RoleTect Ingest

**Last Updated: May 20, 2026**

## 1. Introduction
This Privacy Policy applies to the RoleTect Ingest browser extension ("the Extension"). The Extension is designed as a "bring-your-own-backend" tool that allows users to capture specific elements from their currently active web page and send them to a personal, user-defined server (such as localhost or a custom webhook). 

Privacy is a core principle of this Extension. The developer of this Extension does NOT collect, store, transmit, or have any access to your personal data, browsing history, or the content you capture.

## 2. Information the Extension Processes
To function, the Extension processes the following information strictly on your local device:
* The URL of your currently active tab.
* The HTML content of the active tab (specifically the `<body>` tag or a custom HTML tag defined by you).
* A Secret Key (defined by you, used to authenticate your requests).
* A Destination URL (defined by you, defaulting to localhost, where the data will be sent).

The Extension ONLY accesses the URL and content of your current, active tab when you explicitly trigger it. It does not monitor tabs in the background, and it does not access data from tabs you have not actively triggered the Extension on.

## 3. Where Your Data Goes
The data extracted by the Extension is transmitted directly from your browser to the Destination URL you have configured in the Extension's settings. 

* The data does NOT pass through any servers owned, operated, or controlled by the developer.
* The developer has zero access to the URLs you visit, the data you scrape, your secret key, or your destination URL.
* All data transmission is strictly between your local browser and your configured backend endpoint.

## 4. Permissions Used and Why
The Extension requires specific browser permissions to operate:
* **"activeTab"**: This permission is used to read the URL and extract the targeted HTML tags from the webpage you are currently viewing, but only when you actively interact with the Extension.
* **"storage"**: This permission is used to save your Extension settings (Destination URL, Target Tags, and Secret Key) locally on your device.
* **Optional Network Permissions**: The Extension will request dynamic permission to send network requests to your specific custom Destination URL to prevent unauthorized data exfiltration.

## 5. Third-Party Sharing
Because the developer does not collect any of your data, the developer does not (and cannot) sell, trade, or share your data with any third parties. 

## 6. Data Security
Your Secret Key and settings are stored locally on your device using the browser's native storage API. When configuring a custom Destination URL (outside of localhost), users are strongly encouraged to use HTTPS endpoints to ensure their captured data and secret keys are encrypted in transit.

## 7. Changes to This Privacy Policy
We may update this Privacy Policy from time to time. Any changes will be reflected by the "Last Updated" date at the top of this policy.

## 8. Contact Information
If you have any questions or concerns regarding this Privacy Policy or how the Extension handles data, please contact the developer at: b220305006@cse.jnu.ac.bd
