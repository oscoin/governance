const ipcRenderer = require("electron").ipcRenderer;
window.electron = {
  ipcRenderer: {
    invoke: ipcRenderer.invoke.bind(ipcRenderer),
    on: ipcRenderer.on.bind(ipcRenderer),
  },
  isDev: process.env.NODE_ENV === "development",
  isExperimental: process.env.RADICLE_UPSTREAM_EXPERIMENTAL === "true",
};
