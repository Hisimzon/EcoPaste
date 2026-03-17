import { emit } from "@tauri-apps/api/event";
import { Menu, MenuItem, PredefinedMenuItem } from "@tauri-apps/api/menu";
import { resolveResource } from "@tauri-apps/api/path";
import { TrayIcon, type TrayIconOptions } from "@tauri-apps/api/tray";
import { openUrl } from "@tauri-apps/plugin-opener";
import { exit, relaunch } from "@tauri-apps/plugin-process";
import { useBoolean, useUpdateEffect } from "ahooks";
import { useTranslation } from "react-i18next";
import { GITHUB_LINK, LISTEN_KEY } from "@/constants";
import { showWindow } from "@/plugins/window";
import { globalStore } from "@/stores/global";
import { isMac } from "@/utils/is";
import { useSubscribeKey } from "./useSubscribeKey";

const TRAY_ID = "app-tray";
const TRAY_MENU_PREFERENCE_ID = "tray.preference";
const TRAY_MENU_TOGGLE_LISTEN_ID = "tray.toggle-listen";
const TRAY_MENU_CHECK_UPDATE_ID = "tray.check-update";
const TRAY_MENU_OPEN_SOURCE_ID = "tray.open-source";
const TRAY_MENU_RELAUNCH_ID = "tray.relaunch";
const TRAY_MENU_EXIT_ID = "tray.exit";
const ALLOW_APP_EXIT_EVENT = "allow-app-exit";

let creatingTrayPromise: Promise<TrayIcon | void> | null = null;

export const useTray = () => {
  const [startListen, { toggle }] = useBoolean(true);
  const { t } = useTranslation();

  // 监听是否显示菜单栏图标
  useSubscribeKey(globalStore.app, "showMenubarIcon", async (value) => {
    const tray = await getTrayById();

    if (tray) {
      tray.setVisible(value);
    } else {
      createTray();
    }
  });

  // 监听语言变更
  useSubscribeKey(globalStore.appearance, "language", () => {
    updateTrayMenu();
  });

  useUpdateEffect(() => {
    updateTrayMenu();

    emit(LISTEN_KEY.TOGGLE_LISTEN_CLIPBOARD, startListen);
  }, [startListen]);

  // 通过 id 获取托盘图标
  const getTrayById = () => {
    return TrayIcon.getById(TRAY_ID);
  };

  // 创建托盘
  const createTray = async () => {
    if (creatingTrayPromise) {
      await creatingTrayPromise;

      return;
    }

    creatingTrayPromise = (async () => {
      const tray = await getTrayById();

      if (tray) {
        tray.setVisible(globalStore.app.showMenubarIcon);

        const menu = await getTrayMenu();

        tray.setMenu(menu);

        return tray;
      }

      if (!globalStore.app.showMenubarIcon) return;

      const { appName, appVersion } = globalStore.env;

      const menu = await getTrayMenu();

      const iconPath = isMac ? "assets/tray-mac.ico" : "assets/tray.ico";
      const icon = await resolveResource(iconPath);

      const options: TrayIconOptions = {
        action: (event) => {
          if (isMac) return;

          if (event.type === "Click" && event.button === "Left") {
            showWindow("main");
          }
        },
        icon,
        iconAsTemplate: true,
        id: TRAY_ID,
        menu,
        menuOnLeftClick: isMac,
        tooltip: `${appName} v${appVersion}`,
      };

      return TrayIcon.new(options);
    })();

    try {
      await creatingTrayPromise;
    } finally {
      creatingTrayPromise = null;
    }
  };

  // 获取托盘菜单
  const getTrayMenu = async () => {
    const { appVersion } = globalStore.env;

    const items = await Promise.all([
      MenuItem.new({
        accelerator: isMac ? "Cmd+," : void 0,
        action: () => showWindow("preference"),
        id: TRAY_MENU_PREFERENCE_ID,
        text: t("component.tray.label.preference"),
      }),
      MenuItem.new({
        action: toggle,
        id: TRAY_MENU_TOGGLE_LISTEN_ID,
        text: startListen
          ? t("component.tray.label.stop_listening")
          : t("component.tray.label.start_listening"),
      }),
      PredefinedMenuItem.new({ item: "Separator" }),
      MenuItem.new({
        action: () => {
          showWindow();

          emit(LISTEN_KEY.UPDATE_APP, true);
        },
        id: TRAY_MENU_CHECK_UPDATE_ID,
        text: t("component.tray.label.check_update"),
      }),
      MenuItem.new({
        action: () => openUrl(GITHUB_LINK),
        id: TRAY_MENU_OPEN_SOURCE_ID,
        text: t("component.tray.label.open_source_address"),
      }),
      PredefinedMenuItem.new({ item: "Separator" }),
      MenuItem.new({
        enabled: false,
        text: `${t("component.tray.label.version")} ${appVersion}`,
      }),
      MenuItem.new({
        action: async () => {
          await emit(ALLOW_APP_EXIT_EVENT);
          await relaunch();
        },
        id: TRAY_MENU_RELAUNCH_ID,
        text: t("component.tray.label.relaunch"),
      }),
      MenuItem.new({
        accelerator: isMac ? "Cmd+Q" : void 0,
        action: async () => {
          await emit(ALLOW_APP_EXIT_EVENT);
          await exit(0);
        },
        id: TRAY_MENU_EXIT_ID,
        text: t("component.tray.label.exit"),
      }),
    ]);

    return Menu.new({ items });
  };

  // 更新托盘菜单
  const updateTrayMenu = async () => {
    const tray = await getTrayById();

    if (!tray) return;

    const menu = await getTrayMenu();

    tray.setMenu(menu);
  };

  return {
    createTray,
  };
};
