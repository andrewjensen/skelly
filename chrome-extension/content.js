console.log("hello from the content script!");

async function sendToSkelly() {
  console.log("sending to skelly");

  const pageHtml = document.documentElement.outerHTML;
  console.log("page HTML:", pageHtml);

  const pageUrl = window.location.href;

  const message = {
    type: "send-to-skelly",
    pageHtml,
    pageUrl,
  };

  chrome.runtime.sendMessage(message, (response) => {
    console.log("Message sent to extension, response:", response);
  });
}

sendToSkelly();
