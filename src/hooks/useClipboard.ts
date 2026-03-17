import { emit } from "@tauri-apps/api/event";
import { useMount } from "ahooks";
import { cloneDeep } from "es-toolkit";
import { isEmpty, remove } from "es-toolkit/compat";
import { nanoid } from "nanoid";
import {
  type ClipboardChangeOptions,
  onClipboardChange,
  type ReadClipboard,
  readClipboard,
  startListening,
} from "tauri-plugin-clipboard-x-api";
import { fullName } from "tauri-plugin-fs-pro-api";
import { LISTEN_KEY } from "@/constants";
import {
  insertHistory,
  selectHistory,
  updateHistory,
} from "@/database/history";
import type { State } from "@/pages/Main";
import { getClipboardTextSubtype } from "@/plugins/clipboard";
import {
  consumeLowResourceClipboardDirty,
  drainLowResourceClipboardQueue,
} from "@/plugins/window";
import { clipboardStore } from "@/stores/clipboard";
import type { DatabaseSchemaHistory } from "@/types/database";
import { formatDate } from "@/utils/dayjs";
import { appendPinyinToSearch } from "@/utils/pinyin";
import { normalizeTextThreshold } from "@/utils/threshold";

let clipboardChangeQueue: Promise<void> = Promise.resolve();

export const useClipboard = (
  state: State,
  options?: ClipboardChangeOptions,
) => {
  const processClipboardChange = async (
    result: ReadClipboard,
    config?: {
      skipVisibleInsert?: boolean;
    },
  ): Promise<string | undefined> => {
    const { files, image, html, rtf, text } = result;

    if (isEmpty(result) || Object.values(result).every(isEmpty)) return;

    const { copyPlain, textThreshold } = clipboardStore.content;
    const normalizedTextThreshold = normalizeTextThreshold(textThreshold);

    const data = {
      createTime: formatDate(void 0, "YYYY-MM-DD HH:mm:ss.SSS"),
      favorite: false,
      group: "text",
      id: nanoid(),
      search: text?.value,
    } as DatabaseSchemaHistory;

    if (files) {
      Object.assign(data, files, {
        group: "files",
        search: files.value.join(" "),
      });
    } else if (html && !copyPlain) {
      Object.assign(data, html);
    } else if (rtf && !copyPlain) {
      Object.assign(data, rtf);
    } else if (text) {
      const subtype = await getClipboardTextSubtype(text.value);

      Object.assign(data, text, {
        subtype,
      });
    } else if (image) {
      Object.assign(data, image, {
        group: "image",
      });
    }

    const sqlData = cloneDeep(data);

    const { type, value, group, createTime } = data;

    if (type === "image") {
      sqlData.value = await fullName(value);
    }

    if (type === "files") {
      sqlData.value = JSON.stringify(value);
    }

    sqlData.search = appendPinyinToSearch(
      sqlData.search,
      normalizedTextThreshold,
    );

    const [matched] = await selectHistory((qb) => {
      const { type, value } = sqlData;

      return qb.where("type", "=", type).where("value", "=", value);
    });

    const visible = state.group === "all" || state.group === group;
    const skipVisibleInsert = Boolean(config?.skipVisibleInsert);

    if (matched) {
      if (!clipboardStore.content.autoSort) return;

      const { favorite, id, note, search } = matched;

      if (visible && !skipVisibleInsert) {
        remove(state.list, { id });

        state.list.unshift({
          ...matched,
          ...data,
          favorite,
          id,
          note,
          search,
        });
      }

      await updateHistory(id, { createTime });

      return id;
    }

    if (visible && !skipVisibleInsert) {
      state.list.unshift(data);
    }

    await insertHistory(sqlData);

    return data.id;
  };

  const enqueueClipboardChange = async (
    result: ReadClipboard,
    config?: {
      skipVisibleInsert?: boolean;
    },
  ) => {
    let processedId: string | undefined;

    clipboardChangeQueue = clipboardChangeQueue
      .catch(() => {})
      .then(async () => {
        processedId = await processClipboardChange(result, config);
      });

    await clipboardChangeQueue;

    return processedId;
  };

  useMount(async () => {
    await startListening();

    const queued = await drainLowResourceClipboardQueue().catch(() => []);

    if (queued.length) {
      const replayInsertedIds: string[] = [];

      for (const item of queued) {
        const id = await enqueueClipboardChange(item, {
          skipVisibleInsert: true,
        });

        if (id) {
          replayInsertedIds.push(id);
        }
      }

      await emit(LISTEN_KEY.REFRESH_CLIPBOARD_LIST);

      state.replayInsertedIds = [...new Set(replayInsertedIds.reverse())].slice(
        0,
        12,
      );
    } else {
      const dirty = await consumeLowResourceClipboardDirty().catch(() => false);

      if (dirty) {
        const latest = await readClipboard().catch(() => ({}));

        const id = await enqueueClipboardChange(latest);

        state.replayInsertedIds = id ? [id] : [];
      }
    }

    onClipboardChange(async (result) => {
      await enqueueClipboardChange(result);
    }, options);
  });
};
