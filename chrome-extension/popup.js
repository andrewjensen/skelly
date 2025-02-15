let skellyHost = null;

function main() {
  skellyHost = localStorage.getItem("skellyHost");
  if (skellyHost) {
    document.getElementById("skelly-host").value = skellyHost;
  }

  document.getElementById("send-to-skelly").addEventListener("click", handleClickSendToSkelly);

  document.getElementById("save-skelly-host").addEventListener("click", handleClickSaveSkellyHost);

  chrome.runtime.onMessage.addListener(handleMessage);
}

main();

async function handleMessage(message, sender, sendResponse) {
  if (message.type === "send-to-skelly") {
    console.log("Received send-to-skelly message", message);
    const { pageHtml, pageUrl } = message;
    await requestRender(pageHtml, pageUrl);
    sendResponse({ status: "ok" });
  }
}

function handleClickSaveSkellyHost() {
  console.log("Clicked Save Skelly Host");
  let newSkellyHost = document.getElementById("skelly-host").value;

  try {
    const url = new URL(newSkellyHost);
    newSkellyHost = `${url.protocol}//${url.host}`;
  } catch (e) {
    // If URL parsing fails, use the input as-is
    newSkellyHost = newSkellyHost;
  }

  console.log("New Skelly Host:", newSkellyHost);

  document.getElementById("skelly-host").value = newSkellyHost;
  localStorage.setItem("skellyHost", newSkellyHost);
  skellyHost = newSkellyHost;
}

function handleClickSendToSkelly() {
  console.log("Clicked Send to Skelly");
  chrome.tabs.query({ active: true, currentWindow: true }, function (tabs) {
    const currentTabId = tabs[0].id;
    console.log("Current tab ID:", currentTabId);
    chrome.scripting
      .executeScript({
        target: { tabId: currentTabId },
        files: ["content.js"],
      })
      .then(() => console.log("script injected"));
  });
}

async function requestRender(pageHtml, pageUrl) {
  console.log("sendToSkelly", pageUrl);

  if (!skellyHost) {
    throw new Error("Skelly host not configured");
  }

  const response = await fetch(`${skellyHost}/render`, {
    method: "POST",
    // mode: "no-cors",
    headers: {
      "Content-Type": "text/html",
      "X-Skelly-Page-Url": pageUrl,
    },
    body: pageHtml,
  });

  if (response.ok) {
    console.log("Successfully sent to Skelly");
  } else {
    throw new Error(
      `Failed to send to Skelly: ${response.status} ${response.statusText}`,
    );
  }
}
