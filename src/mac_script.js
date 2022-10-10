window.sendToPlugin = function (msg) {
  webkit.messageHandlers.main.postMessage(JSON.stringify(msg));
}

window.onPluginMessage = function() {};

window.onPluginMessageInternal = function(msg) {
  const json = JSON.parse(msg);
  window.onPluginMessage && window.onPluginMessage(json);
}
