import { useEventEmitter, useKeyPress, useMount, useReactive } from "ahooks";
import type { EventEmitter } from "ahooks/lib/useEventEmitter";
import { range } from "es-toolkit";
import { find, last } from "es-toolkit/compat";
import { createContext, useRef } from "react";
import { startListening, stopListening } from "tauri-plugin-clipboard-x-api";
import { useSnapshot } from "valtio";
import Audio, { type AudioRef } from "@/components/Audio";
import { LISTEN_KEY, PRESET_SHORTCUT } from "@/constants";
import { useClipboard } from "@/hooks/useClipboard";
import { useImmediateKey } from "@/hooks/useImmediateKey";
import { useRegister } from "@/hooks/useRegister";
import { useSubscribeKey } from "@/hooks/useSubscribeKey";
import { useTauriListen } from "@/hooks/useTauriListen";
import { pasteToClipboard } from "@/plugins/clipboard";
import {
  showTaskbarIcon,
  showWindow,
  toggleWindowVisible,
} from "@/plugins/window";
import { clipboardStore } from "@/stores/clipboard";
import { globalStore } from "@/stores/global";
import type {
  DatabaseSchemaGroupId,
  DatabaseSchemaHistory,
} from "@/types/database";
import type { Store } from "@/types/store";
import { deepAssign } from "@/utils/object";
import DockMode from "./components/DockMode";
import StandardMode from "./components/StandardMode";

interface EventBusPayload {
  id: string;
  action: string;
}

export interface State {
  group: DatabaseSchemaGroupId;
  search?: string;
  pinned?: boolean;
  activeId?: string;
  list: DatabaseSchemaHistory[];
  eventBus?: EventEmitter<EventBusPayload>;
  quickPasteKeys: string[];
}

const INITIAL_STATE: State = {
  group: "all",
  list: [],
  quickPasteKeys: [],
};

interface MainContextValue {
  rootState: State;
}

export const MainContext = createContext<MainContextValue>({
  rootState: INITIAL_STATE,
});

const Main = () => {
  const state = useReactive<State>(INITIAL_STATE);
  const { shortcut } = useSnapshot(globalStore);
  const { window } = useSnapshot(clipboardStore);
  const eventBus = useEventEmitter<EventBusPayload>();
  const audioRef = useRef<AudioRef>(null);

  useMount(() => {
    state.eventBus = eventBus;
  });

  useClipboard(state, {
    beforeRead() {
      if (!clipboardStore.audio.copy) return;

      audioRef.current?.play();
    },
  });

  // 任务栏图标的显示与隐藏
  useImmediateKey(globalStore.app, "showTaskbarIcon", showTaskbarIcon);

  // 同步配置项
  useTauriListen<Store>(LISTEN_KEY.STORE_CHANGED, ({ payload }) => {
    deepAssign(globalStore, payload.globalStore);
    deepAssign(clipboardStore, payload.clipboardStore);
  });

  // 窗口显示与隐藏
  useRegister(toggleWindowVisible, [shortcut.clipboard]);

  // 打开偏好设置窗口
  useKeyPress(PRESET_SHORTCUT.OPEN_PREFERENCES, () => {
    showWindow("preference");
  });

  // 设置快捷粘贴的快捷键
  const setQuickPasteKeys = () => {
    const { enable, value } = globalStore.shortcut.quickPaste;

    if (!enable) {
      state.quickPasteKeys = [];

      return;
    }

    state.quickPasteKeys = range(1, 10).map((item) => [value, item].join("+"));
  };

  // 监听快速粘贴的启用状态变更
  useImmediateKey(globalStore.shortcut.quickPaste, "enable", () => {
    setQuickPasteKeys();
  });

  // 监听快速粘贴的快捷键变更
  useSubscribeKey(globalStore.shortcut.quickPaste, "value", () => {
    setQuickPasteKeys();
  });

  // 切换剪贴板监听状态
  useTauriListen<boolean>(LISTEN_KEY.TOGGLE_LISTEN_CLIPBOARD, ({ payload }) => {
    if (payload) {
      startListening();
    } else {
      stopListening();
    }
  });

  // 监听粘贴为纯文本的快捷键
  useKeyPress(shortcut.pastePlain, (event) => {
    event.preventDefault();

    const data = find(state.list, { id: state.activeId });

    if (!data) return;

    pasteToClipboard(data, true);
  });

  // 监听快速粘贴的快捷键
  useRegister(
    async (event) => {
      if (!globalStore.shortcut.quickPaste.enable) return;

      const index = Number(last(event.shortcut));

      const data = state.list[index - 1];

      pasteToClipboard(data);
    },
    [state.quickPasteKeys],
  );

  // 监听全局快捷键事件（由后端 rdev 全局捕获）
  useTauriListen<{ code: string }>("dispatch-event", ({ payload }) => {
    const { code } = payload;
    const { activeId, eventBus } = state;

    switch (code) {
      // 空格预览
      case "Space":
        return eventBus?.emit({
          action: LISTEN_KEY.CLIPBOARD_ITEM_PREVIEW,
          id: activeId,
        });
      // 回车粘贴
      case "Enter": {
        if (!activeId) return;
        const data = find(state.list, { id: activeId });
        if (data) {
          return pasteToClipboard(data);
        }
        return;
      }
      // 删除
      case "Backspace":
      case "Delete":
        return eventBus?.emit({
          action: LISTEN_KEY.CLIPBOARD_ITEM_DELETE,
          id: activeId,
        });
      // 选中上一个
      case "ArrowUp":
        return eventBus?.emit({
          action: LISTEN_KEY.CLIPBOARD_ITEM_SELECT_PREV,
          id: activeId,
        });
      // 选中下一个
      case "ArrowDown":
        return eventBus?.emit({
          action: LISTEN_KEY.CLIPBOARD_ITEM_SELECT_NEXT,
          id: activeId,
        });
      // 搜索框聚焦
      case "KeyF":
        return eventBus?.emit({
          action: "focus-search",
          id: "",
        });
      // 隐藏窗口
      case "KeyW":
        return toggleWindowVisible();
    }
  });

  // useTauriListen("dispatch-event", ({ payload }) => {
  //   // console.log("payload", payload);
  // });

  return (
    <MainContext.Provider
      value={{
        rootState: state,
      }}
    >
      <Audio ref={audioRef} />

      {window.style === "standard" ? <StandardMode /> : <DockMode />}
    </MainContext.Provider>
  );
};

export default Main;
