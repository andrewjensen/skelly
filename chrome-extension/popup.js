const SKELLY_HOST = "http://192.168.86.28:3000";

console.log("Hello Extensions");

document.getElementById("send-to-skelly").addEventListener("click", () => {
  console.log("sending to skelly");
  chrome.tabs.query({ active: true, currentWindow: true }, function(tabs) {
    const currentTabId = tabs[0].id;
    console.log("Current tab ID:", currentTabId);
    injectContentScript(currentTabId);
  });
});

chrome.runtime.onMessage.addListener(async (message, sender, sendResponse) => {
  if (message.type === "send-to-skelly") {
    console.log("Received send-to-skelly message", message);
    const pageHtml = message.html;
    await sendToSkelly(pageHtml);
    sendResponse({ status: "ok" });
  }
});

function injectContentScript(currentTabId) {
  chrome.scripting
    .executeScript({
      target: { tabId: currentTabId },
      files: ["content.js"],
    })
    .then(() => console.log("script injected"));
}

async function sendToSkelly(pageHtml) {
  const response = await fetch(`${SKELLY_HOST}/render`, {
    method: "POST",
    // mode: "no-cors",
    headers: {
      "Content-Type": "text/html", 
      "X-Skelly-Page-Url": window.location.href,
    },
    body: pageHtml,
  });

  if (response.ok) {
    console.log("Successfully sent to Skelly");
  } else {
    throw new Error(
      `Failed to send to Skelly: ${response.status} ${response.statusText}`
    );
  }
}
