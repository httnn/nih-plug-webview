window.sendToPlugin = function (msg) {
  window.ipc.postMessage(JSON.stringify(msg));
}

window.onPluginMessage = function() {};

window.onPluginMessageInternal = function(msg) {
  const json = JSON.parse(msg);
  window.onPluginMessage && window.onPluginMessage(json);
}
