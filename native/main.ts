import {
  app,
  BrowserWindow,
  ipcMain,
  dialog,
  clipboard,
  shell,
} from "electron";
import fs from "fs";
import path from "path";
import { ProxyProcessManager } from "./proxy-process-manager";
import { RendererMessage, MainMessage, MainMessageKind } from "./ipc-types";

const isDev = process.env.NODE_ENV === "development";

let proxyPath;
let proxyArgs: string[] = [];

if (isDev) {
  if (process.env.RADICLE_UPSTREAM_PROXY_PATH) {
    proxyPath = path.join(__dirname, process.env.RADICLE_UPSTREAM_PROXY_PATH);
  } else {
    throw new Error(
      "RADICLE_UPSTREAM_PROXY_PATH must be set when running in dev mode!"
    );
  }

  if (process.env.RADICLE_UPSTREAM_PROXY_ARGS) {
    proxyArgs = process.env.RADICLE_UPSTREAM_PROXY_ARGS.split(/[, ]/).filter(
      Boolean
    );
  }
  proxyArgs.push("--default-seed");
  proxyArgs.push(
    "hybz9gfgtd9d4pd14a6r66j5hz6f77fed4jdu7pana4fxaxbt369kg@setzling.radicle.xyz:12345"
  );
} else {
  // Packaged app, i.e. production.
  proxyPath = path.join(__dirname, "../../radicle-proxy");
  proxyArgs = [];
}

if (process.env.RAD_HOME) {
  const electronPath = `${process.env.RAD_HOME}/electron`;
  if (!fs.existsSync(electronPath)) fs.mkdirSync(electronPath);
  app.setPath("userData", electronPath);
  app.setPath("appData", electronPath);
}

// The default value of app.allowRendererProcessReuse is deprecated, it is
// currently "false".  It will change to be "true" in Electron 9.  For more
// information please check https://github.com/electron/electron/issues/18397
app.allowRendererProcessReuse = true;

class WindowManager {
  public window: BrowserWindow | null;
  private messages: MainMessage[];

  constructor() {
    this.window = null;
    this.messages = [];
  }

  // Send a message on the "message" channel to the renderer window
  sendMessage(message: MainMessage) {
    if (this.window === null || this.window.webContents.isLoading()) {
      this.messages.push(message);
    } else {
      this.window.webContents.send("message", message);
    }
  }

  reload() {
    if (this.window) {
      this.window.reload();
    }
  }

  open() {
    if (this.window) {
      return;
    }

    const window = new BrowserWindow({
      width: 1200,
      height: 680,
      icon: path.join(__dirname, "../public/icon.png"),
      show: false,
      autoHideMenuBar: true,
      webPreferences: {
        preload: path.join(__dirname, "preload.js"),
      },
    });

    window.once("ready-to-show", () => {
      window.maximize();
      window.show();
    });

    window.webContents.on("will-navigate", (event, url) => {
      event.preventDefault();
      openExternalLink(url);
    });

    window.webContents.on("new-window", (event, url) => {
      event.preventDefault();
      openExternalLink(url);
    });

    window.on("closed", () => {
      this.window = null;
    });

    window.webContents.on("did-finish-load", () => {
      this.messages.forEach(message => {
        window.webContents.send("message", message);
      });
      this.messages = [];
    });

    let uiUrl;

    if (isDev && process.env.RADICLE_UPSTREAM_UI_ARGS) {
      uiUrl = `../public/index.html?${process.env.RADICLE_UPSTREAM_UI_ARGS}`;
    } else {
      uiUrl = "../public/index.html";
    }

    window.loadURL(`file://${path.join(__dirname, uiUrl)}`);

    this.window = window;
  }
}

const windowManager = new WindowManager();
const proxyProcessManager = new ProxyProcessManager({
  proxyPath,
  proxyArgs,
  lineLimit: 500,
});

ipcMain.handle(RendererMessage.DIALOG_SHOWOPENDIALOG, async () => {
  const window = windowManager.window;
  if (window === null) {
    return;
  }

  const result = await dialog.showOpenDialog(window, {
    properties: ["openDirectory", "showHiddenFiles", "createDirectory"],
  });

  if (result.filePaths.length === 1) {
    return result.filePaths[0];
  } else {
    return "";
  }
});

ipcMain.handle(RendererMessage.CLIPBOARD_WRITETEXT, async (_event, text) => {
  clipboard.writeText(text);
});

ipcMain.handle(RendererMessage.OPEN_PATH, async (_event, path) => {
  shell.openPath(path);
});

ipcMain.handle(RendererMessage.GET_VERSION, () => {
  return app.getVersion();
});

ipcMain.handle(RendererMessage.OPEN_URL, (_event, url) => {
  openExternalLink(url);
});

function setupWatcher() {
  // eslint-disable-next-line @typescript-eslint/no-var-requires
  const chokidar = require("chokidar");
  const watcher = chokidar.watch(path.join(__dirname, "../public/**"), {
    ignoreInitial: true,
  });

  watcher.on("change", () => {
    windowManager.reload();
  });
}

const openExternalLink = (url: string): void => {
  if (
    url.toLowerCase().startsWith("http://") ||
    url.toLowerCase().startsWith("https://")
  ) {
    shell.openExternal(url);
  } else {
    console.warn(`User tried opening URL with invalid URI scheme: ${url}`);
  }
};

app.on("render-process-gone", (_event, _webContents, details) => {
  if (details.reason !== "clean-exit") {
    console.error(`Electron render process is gone. Reason: ${details.reason}`);
    app.quit();
  }
});

app.on("will-quit", () => {
  proxyProcessManager.kill();
});

// This method will be called when Electron has finished
// initialization and is ready to create browser windows.
// Some APIs can only be used after this event occurs.
app.on("ready", () => {
  proxyProcessManager.run().then(({ status, signal, output }) => {
    windowManager.sendMessage({
      kind: MainMessageKind.PROXY_ERROR,
      data: {
        status,
        signal,
        output,
      },
    });
  });

  if (isDev) {
    setupWatcher();
  }

  windowManager.open();
});

// Quit when all windows are closed.
app.on("window-all-closed", () => {
  // On macOS it is common for applications and their menu bar
  // to stay active until the user quits explicitly with Cmd + Q
  if (process.platform !== "darwin") {
    app.quit();
  }
});

app.on("activate", () => {
  if (app.isReady() && !windowManager.window) {
    windowManager.open();
  }
});
