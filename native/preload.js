window.electron = {
  ipcRenderer: { invoke: require("electron").ipcRenderer.invoke },
  isDev: process.env.NODE_ENV === "development",
  isExperimental: process.env.RADICLE_UPSTREAM_EXPERIMENTAL === "true",
};
