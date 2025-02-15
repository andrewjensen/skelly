const STATUSES = {
  INITIAL: {
    cssClass: "status-initial",
    message: "Initialized",
  },
  CONNECTED: {
    cssClass: "status-connected",
    message: "Connected to Skelly",
  },
  DISCONNECTED: {
    cssClass: "status-disconnected",
    message: "Disconnected from Skelly",
  },
  CONNECTING: {
    cssClass: "status-pending",
    message: "Connecting to Skelly...",
  },
  RENDER_LOADING: {
    cssClass: "status-pending",
    message: "Sending...",
  },
  RENDER_SUCCESS: {
    cssClass: "status-success",
    message: "Sent successfully",
  },
  RENDER_ERROR: {
    cssClass: "status-error",
    message: "Failed to send",
  },
};

let skellyHost = null;

let appStatus = STATUSES.INITIAL;

function main() {
  skellyHost = localStorage.getItem("skellyHost");
  if (skellyHost) {
    document.getElementById("skelly-host").value = skellyHost;
  }

  setStatus(STATUSES.INITIAL);

  document.getElementById("send-to-skelly").addEventListener("click", handleClickSendToSkelly);

  document.getElementById("save-skelly-host").addEventListener("click", handleClickSaveSkellyHost);

  chrome.runtime.onMessage.addListener(handleMessage);

}

main();

function setStatus(newStatus) {
  console.log("Setting status to", newStatus);

  appStatus = newStatus;
  const statusBanner = document.getElementById("status-banner");
  statusBanner.textContent = appStatus.message;
  statusBanner.className = appStatus.cssClass;
}

async function handleMessage(message, sender, sendResponse) {
  if (message.type === "send-to-skelly") {
    console.log("Received send-to-skelly message", message);
    const { pageHtml, pageUrl } = message;
    try {
      await requestRender(pageHtml, pageUrl);
      sendResponse({ status: "ok" });
      setStatus(STATUSES.RENDER_SUCCESS);
    } catch (e) {
      console.error("Error sending to Skelly", e);
      sendResponse({ status: "error" });
      setStatus(STATUSES.RENDER_ERROR);
    }
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
  setStatus(STATUSES.RENDER_LOADING);

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
