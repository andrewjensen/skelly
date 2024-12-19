console.log("hello from the content script!");

async function sendToSkelly() {
  console.log("sending to skelly");

  const pageHtml = document.documentElement.outerHTML;
  console.log("page HTML:", pageHtml);

  chrome.runtime.sendMessage({ type: "send-to-skelly", html: pageHtml }, (response) => {
    console.log("Message sent to extension, response:", response);
  });
}

sendToSkelly();
