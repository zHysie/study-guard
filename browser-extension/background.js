const HOST_NAME = "com.local.study_guardian";

async function getActiveTab(windowId) {
  const query = { active: true };
  if (typeof windowId === "number" && windowId >= 0) {
    query.windowId = windowId;
  } else {
    query.currentWindow = true;
  }
  const tabs = await chrome.tabs.query(query);
  return tabs[0];
}

function publishTab(tab) {
  if (!tab || !tab.url) {
    return;
  }

  const isWebUrl = /^https?:\/\//i.test(tab.url);

  chrome.runtime.sendNativeMessage(
    HOST_NAME,
    {
      url: isWebUrl ? tab.url : "",
      title: tab.title || "",
      timestamp: Date.now(),
    },
    () => {
      void chrome.runtime.lastError;
    },
  );
}

chrome.tabs.onActivated.addListener(async ({ tabId }) => {
  try {
    publishTab(await chrome.tabs.get(tabId));
  } catch (_) {
    // The tab may have closed before we read it.
  }
});

chrome.tabs.onUpdated.addListener((_tabId, changeInfo, tab) => {
  if (changeInfo.url || changeInfo.status === "complete") {
    publishTab(tab);
  }
});

chrome.windows.onFocusChanged.addListener(async (windowId) => {
  if (windowId === chrome.windows.WINDOW_ID_NONE) {
    return;
  }

  try {
    publishTab(await getActiveTab(windowId));
  } catch (_) {
    // Ignore transient browser focus races.
  }
});

chrome.runtime.onStartup.addListener(async () => {
  try {
    publishTab(await getActiveTab());
  } catch (_) {
    // Ignore startup without an active tab.
  }
});

chrome.runtime.onInstalled.addListener(async () => {
  try {
    publishTab(await getActiveTab());
  } catch (_) {
    // Ignore install pages that cannot be queried.
  }
});

setInterval(async () => {
  try {
    publishTab(await getActiveTab());
  } catch (_) {
    // Service workers can wake without a focused browser window.
  }
}, 2000);
